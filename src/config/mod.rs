mod db_config;
mod jwt_config;
mod log_config;

use crate::utils::RedisConfig;
use figment::Figment;
use figment::providers::{Env, Format, Toml, Yaml};
use serde::Deserialize;
use std::sync::OnceLock;

pub use db_config::DbConfig;
pub use jwt_config::JwtConfig;
pub use log_config::LogConfig;

pub static CONFIG: OnceLock<ServerConfig> = OnceLock::new();

pub fn init() {
    let raw_config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Yaml::file("config.yaml"))
        .merge(Env::prefixed("APP_"));

    let config = match raw_config.extract::<ServerConfig>() {
        Ok(config) => config,
        Err(err) => {
            println!("It looks like your config is invalid. The following error occurred: {err}");
            std::process::exit(1);
        }
    };

    crate::config::CONFIG
        .set(config)
        .expect("config should be set");
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub log: LogConfig,
    pub db: DbConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
}

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn get() -> &'static ServerConfig {
    CONFIG.get().expect("CONFIG should be initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        init();
        let config = get();
        println!("{config:?}")
    }
}
