use im_share::redis::init_redis_client;
use prelude::*;

use salvo::prelude::*;
use salvo::server::ServerHandle;
use std::process::exit;
use std::time::Duration;
use tokio::signal;

mod config;
mod hoops;
mod prelude;
mod routers;

#[tokio::main]
async fn main() {
    config::init();
    let config = config::get();
    let _ = config.log.guard();

    match tokio::time::timeout(Duration::from_secs(5), init_redis_client(&config.redis)).await {
        Ok(result) => {
            result
                .map_err(|e| format!("redis init error: {}", e))
                .unwrap();
        }
        Err(_) => {
            error!("Redis 链接超时，server启动失败: Timeout limit {}s", 5);
            exit(1);
        }
    }

    let router = crate::routers::root();
    info!("{config:#?}");
    info!("{router:?}");

    let service = Service::new(router)
        .hoop(Logger::new())
        .hoop(crate::hoops::cors_hoop());

    let listen_addr = "127.0.0.1:8081";

    let acceptor = TcpListener::new(listen_addr).bind().await;
    let server = Server::new(acceptor);
    tokio::spawn(shutdown_signal(server.handle()));
    server.serve(service).await
}

async fn shutdown_signal(handle: ServerHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("ctrl_c signal received"),
        _ = terminate => info!("terminate signal received"),
    }
    handle.stop_graceful(std::time::Duration::from_secs(60));
}
