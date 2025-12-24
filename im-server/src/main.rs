use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use im_server::utils::subcription::SubscriptionService;
use im_server::{mqtt, prelude::*};
use salvo::server::ServerHandle;
use salvo::{catcher::Catcher, prelude::*};
use tokio::signal;

#[tokio::main]
async fn main() {
    im_server::config::init();
    let config = im_server::config::get();
    let _guard = config.log.guard();

    im_server::db::init(&config.db).await;

    match tokio::time::timeout(
        Duration::from_secs(5),
        im_server::utils::init_redis_client(&config.redis),
    )
    .await
    {
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

    mqtt::init_mqtt_client(&config.mqtt)
        .await
        .map_err(|e| format!("mqtt init error: {}", e))
        .unwrap();

    let router = im_server::routers::root();
    info!("{config:#?}");
    info!("{router:?}");

    let catcher = Catcher::default().hoop(im_server::hoops::catch_status_error);
    let service = Service::new(router)
        .catcher(catcher)
        .hoop(affix_state::inject(Arc::new(SubscriptionService::new())))
        .hoop(Logger::new())
        .hoop(im_server::hoops::cors_hoop());

    let listen_addr = "127.0.0.1:8080";

    println!(
        "Open API 页面: http://{}/scalar",
        listen_addr.replace("0.0.0.0", "127.0.0.1")
    );

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
