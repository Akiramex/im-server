#![allow(dead_code)]
pub mod password;
pub use password::{hash_password, verify_password};

pub mod redis;
pub use redis::{RedisClient, RedisConfig, init_redis_client};

pub mod mqtt;

pub mod snowflake;

pub mod subcription;

use std::time::{SystemTime, UNIX_EPOCH};
/// 获取当前时间戳（毫秒）
pub fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// 获取当前时间戳（秒）
pub fn now_timestamp_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
