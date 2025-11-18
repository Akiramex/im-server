use sqlx::postgres::{PgConnectOptions, PgPool};
use std::sync::OnceLock;

use crate::config::DbConfig;

pub static SQLX_POOL: OnceLock<PgPool> = OnceLock::new();

pub async fn init(config: &DbConfig) {
    let options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.username)
        .password(&config.password)
        .ssl_mode(sqlx::postgres::PgSslMode::Disable)
        .database(&config.database);

    let sqlx_pool = PgPool::connect_with(options)
        .await
        .expect("Database connection failed.");
    crate::db::SQLX_POOL
        .set(sqlx_pool)
        .expect("sqlx pool should be set")
}

pub fn pool() -> &'static PgPool {
    SQLX_POOL.get().expect("sqlx pool should be set")
}

// PgPool::connect()
#[allow(dead_code)]
fn dsn(config: &DbConfig) -> String {
    format!(
        "postgresql://{}:{}@{}:{}/{}?sslmode=disable",
        config.username, config.password, config.host, config.port, config.database
    )
}
