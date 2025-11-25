use redis::{Client, aio::ConnectionManager};
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    #[serde(default = "default_redis_host")]
    pub host: String,
    #[serde(default = "default_redis_port")]
    pub port: u16,
    #[serde(default)]
    pub db: u8,
    #[serde(default)]
    pub password: Option<String>,
}

fn default_redis_host() -> String {
    "127.0.0.1".to_string()
}

fn default_redis_port() -> u16 {
    6379
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: default_redis_host(),
            port: default_redis_port(),
            db: 0,
            password: None,
        }
    }
}

impl RedisConfig {
    pub fn new(host: String, port: u16, db: u8, password: Option<String>) -> Self {
        Self {
            host,
            port,
            db,
            password,
        }
    }
}

pub static REDIS_CLIENT: OnceLock<ConnectionManager> = OnceLock::new();

pub async fn init_redis_client(config: &RedisConfig) -> anyhow::Result<()> {
    let url = if let Some(password) = config.password.clone() {
        format!(
            "redis://:{}@{}:{}/{}",
            password, config.host, config.port, config.db
        )
    } else {
        format!("redis://{}:{}/{}", config.host, config.port, config.db)
    };

    let client = Client::open(url)?;
    let manager = ConnectionManager::new(client).await?;

    let mut conn = manager.clone();
    redis::cmd("PING").query_async::<String>(&mut conn).await?;

    info!(
        "Redis 连接成功: {}:{}/{}",
        config.host, config.port, config.db
    );

    REDIS_CLIENT
        .set(manager)
        .map_err(|_| anyhow::anyhow!("redis client should be set"))?;

    Ok(())
}

pub struct RedisClient;

impl RedisClient {
    pub fn get_connection() -> ConnectionManager {
        REDIS_CLIENT
            .get()
            .expect("redis client should be set")
            .clone()
    }

    /// 设置键值对（带过期时间，单位：秒）
    pub async fn set_with_ttl(key: &str, value: &str, ttl: u64) -> Result<(), redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl)
            .query_async(&mut conn)
            .await
    }

    /// 设置键值对（永久）
    pub async fn set(key: &str, value: &str) -> Result<(), redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut conn)
            .await
    }

    /// 获取值
    pub async fn get(key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        redis::cmd("GET").arg(key).query_async(&mut conn).await
    }

    /// 删除键
    pub async fn del(key: &str) -> Result<(), redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        redis::cmd("DEL").arg(key).query_async(&mut conn).await
    }

    /// 删除多个键
    pub async fn del_many(keys: &[&str]) -> Result<(), redis::RedisError> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut conn = RedisClient::get_connection();
        let mut cmd = redis::cmd("DEL");
        for key in keys {
            cmd.arg(key);
        }
        cmd.query_async(&mut conn).await
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        let result: i64 = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(result > 0)
    }

    /// 设置过期时间
    pub async fn expire(key: &str, seconds: u64) -> Result<(), redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        redis::cmd("EXPIRE")
            .arg(key)
            .arg(seconds)
            .query_async(&mut conn)
            .await
    }
}
