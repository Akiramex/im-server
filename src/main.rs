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
pub type AppResult<T> = Result<T, AppError>;
pub type JsonResult<T> = Result<Json<T>, AppError>;

#[tokio::main]
async fn main() {
    crate::config::init();
    let config = crate::config::get();
    crate::db::init(&config.db).await;
    let _guard = config.log.guard();

    let router = crate::routers::root();
    info!("{router:?}");
    let service = Service::new(router)
        .catcher(Catcher::default().hoop(hoops::error_404))
        .hoop(hoops::cors_hoop());

    let acceptor = TcpListener::new("127.0.0.1:8080").bind().await;
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
