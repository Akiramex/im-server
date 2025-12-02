use std::sync::Arc;

use salvo::server::ServerHandle;
use salvo::{catcher::Catcher, prelude::*};
use tokio::signal;
use tracing::info;

mod api;
mod config;
mod db;
mod dto;
mod error;
mod hoops;
mod models;
mod routers;
mod service;
mod utils;

pub use error::AppError;
pub use models::MyResponse;

use crate::utils::subcription::SubscriptionService;
pub type AppResult<T> = Result<T, AppError>;
pub type JsonResult<T> = Result<Json<T>, AppError>;
pub fn json_ok<T>(data: T) -> JsonResult<T> {
    Ok(Json(data))
}

#[tokio::main]
async fn main() {
    crate::config::init();
    let config = crate::config::get();
    let _guard = config.log.guard();

    crate::db::init(&config.db).await;
    crate::utils::init_redis_client(&config.redis)
        .await
        .map_err(|e| format!("redis init error: {}", e))
        .unwrap();

    let router = crate::routers::root();
    info!("{config:#?}");
    info!("{router:?}");

    let catcher = Catcher::default().hoop(hoops::catch_status_error);
    let service = Service::new(router)
        .catcher(catcher)
        .hoop(affix_state::inject(Arc::new(SubscriptionService::new())))
        .hoop(Logger::new())
        .hoop(hoops::cors_hoop());

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
