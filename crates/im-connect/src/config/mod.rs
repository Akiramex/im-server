mod jwt_config;
mod log_config;

use figment::Figment;
use figment::providers::{Env, Format, Toml, Yaml};
use im_share::mqtt::MqttConfig;
use im_share::redis::RedisConfig;
use serde::Deserialize;
use std::sync::OnceLock;

pub use jwt_config::JwtConfig;
pub use log_config::LogConfig;

pub static CONFIG: OnceLock<ServerConfig> = OnceLock::new();

pub fn init() {
    let raw_config = Figment::new()
        .merge(Yaml::file("config.yaml"))
        .merge(Toml::file("config.toml"))
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
    pub jwt: JwtConfig,
    pub redis: RedisConfig,
    pub mqtt: MqttConfig,
}

pub fn default_true() -> bool {
    true
}

#[allow(dead_code)]
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
