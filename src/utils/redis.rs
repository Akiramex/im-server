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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

    // ========== 群消息已读状态相关方法（使用 Redis Set） ==========

    /// 标记群消息为已读
    /// key: group:read:{group_id}:{message_id}
    /// 使用 Set 存储已读用户的 open_id
    pub async fn mark_group_message_read(
        group_id: &str,
        message_id: &str,
        user_id: &str,
    ) -> Result<(), redis::RedisError> {
        let key = format!("group:read:{}:{}", group_id, message_id);
        let mut conn = RedisClient::get_connection();
        // Redis 的 `SADD` 命令是 Set Add 的缩写，用于向 Redis 集合（Set）中添加一个或多个成员。
        let _: i64 = redis::cmd("SADD")
            .arg(&key)
            .arg(user_id)
            .query_async(&mut conn)
            .await?;
        // 设置过期时间：30天
        let _: i64 = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(2592000u64) // 30 * 24 * 60 * 60
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    /// 检查用户是否已读群消息
    pub async fn is_group_message_read(
        group_id: &str,
        message_id: &str,
        user_id: &str,
    ) -> Result<bool, redis::RedisError> {
        let key = format!("group:read:{}:{}", group_id, message_id);
        let mut conn = RedisClient::get_connection();
        // SISMEMBER：检查成员是否在集合中
        let result: i64 = redis::cmd("SISMEMBER")
            .arg(&key)
            .arg(user_id)
            .query_async(&mut conn)
            .await?;
        Ok(result > 0)
    }

    /// 获取群消息的已读用户列表
    pub async fn get_group_message_read_users(
        group_id: &str,
        message_id: &str,
    ) -> Result<Vec<String>, redis::RedisError> {
        let key = format!("group:read:{}:{}", group_id, message_id);
        let mut conn = RedisClient::get_connection();
        // SMEMBERS：获取集合中的所有成员
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        Ok(users)
    }

    /// 获取群消息的已读数量
    pub async fn get_group_message_read_count(
        group_id: &str,
        message_id: &str,
    ) -> Result<usize, redis::RedisError> {
        let key = format!("group:read:{}:{}", group_id, message_id);
        let mut conn = RedisClient::get_connection();
        // SCARD：获取集合的成员数量
        let count: i64 = redis::cmd("SCARD").arg(&key).query_async(&mut conn).await?;
        Ok(count as usize)
    }

    /// 批量标记群消息为已读
    pub async fn mark_group_messages_read(
        group_id: &str,
        message_ids: &[&str],
        user_id: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = RedisClient::get_connection();
        for message_id in message_ids {
            let key = format!("group:read:{}:{}", group_id, message_id);
            let _: i64 = redis::cmd("SADD")
                .arg(&key)
                .arg(user_id)
                .query_async(&mut conn)
                .await?;
            // 设置过期时间：30天
            let _: i64 = redis::cmd("EXPIRE")
                .arg(&key)
                .arg(2592000u64)
                .query_async(&mut conn)
                .await?;
        }
        Ok(())
    }
}
