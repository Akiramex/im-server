#![allow(dead_code)]
pub mod password;
pub use password::{hash_password, verify_password};

pub mod redis;
pub use redis::{RedisClient, RedisConfig, init_redis_client};

pub mod mqtt;

pub mod snowflake;

pub mod subcription;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::ChatMessage;

pub fn mqtt_user_topic(user_id: &str) -> String {
    format!("user/{user_id}/inbox")
}

pub fn encode_message(message: &ChatMessage) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(message)
}

pub fn decode_message(bytes: &[u8]) -> serde_json::Result<ChatMessage> {
    serde_json::from_slice(bytes)
}

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
