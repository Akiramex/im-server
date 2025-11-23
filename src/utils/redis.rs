use redis::{Client, aio::ConnectionManager};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
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

pub struct RedisClient {
    manager: Arc<Mutex<ConnectionManager>>,
}

impl RedisClient {
    pub async fn new(config: RedisConfig) -> anyhow::Result<Self> {
        let url = if let Some(password) = config.password {
            format!(
                "redis://:{}@{}:{}/{}",
                password, config.host, config.port, config.db
            )
        } else {
            format!("redis://{}:{}/{}", config.host, config.port, config.db)
        };

        info!("连接 Redis: {}:{}/{}", config.host, config.port, config.db);

        let client = Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;

        let mut conn = manager.clone();
        redis::cmd("PING").query_async::<String>(&mut conn).await?;

        info!("Redis 连接成功");

        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }

    pub async fn get_connection(&self) -> ConnectionManager {
        self.manager.lock().await.clone()
    }

    /// 设置键值对（带过期时间，单位：秒）
    pub async fn set_with_ttl(
        &self,
        key: &str,
        value: &str,
        ttl: u32,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await;
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl)
            .query_async(&mut conn)
            .await
    }

    /// 设置键值对（永久）
    pub async fn set(&self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await;
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut conn)
            .await
    }

    /// 获取值
    pub async fn get(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.get_connection().await;
        redis::cmd("GET").arg(key).query_async(&mut conn).await
    }

    /// 删除键
    pub async fn del(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await;
        redis::cmd("DEL").arg(key).query_async(&mut conn).await
    }

    /// 删除多个键
    pub async fn del_many(&self, keys: &[&str]) -> Result<(), redis::RedisError> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut conn = self.get_connection().await;
        let mut cmd = redis::cmd("DEL");
        for key in keys {
            cmd.arg(key);
        }
        cmd.query_async(&mut conn).await
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool, redis::RedisError> {
        let mut conn = self.get_connection().await;
        let result: i64 = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(result > 0)
    }

    /// 设置过期时间
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await;
        redis::cmd("EXPIRE")
            .arg(key)
            .arg(seconds)
            .query_async(&mut conn)
            .await
    }
}
