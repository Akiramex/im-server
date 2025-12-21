use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub message: String,
    pub timestamp_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    /// 聊天类型：1=单聊，2=群聊
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_type: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "to_type", content = "to_id")]
pub enum Target {
    User(String),
    Group(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendRequest {
    pub from_user_id: String,
    pub target: Target,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
}

static GROUP_MEMBERS: LazyLock<RwLock<HashMap<String, Vec<String>>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "g1".to_string(),
        vec!["u1".to_string(), "u2".to_string(), "u3".to_string()],
    );
    map.insert("g2".to_string(), vec!["u2".to_string(), "u4".to_string()]);
    RwLock::new(map)
});

pub fn get_group_members(group_id: &str) -> Vec<String> {
    GROUP_MEMBERS
        .read()
        .ok()
        .and_then(|m| m.get(group_id).cloned())
        .unwrap_or_default()
}

pub fn set_group_members(group_id: &str, members: Vec<String>) {
    if let Ok(mut m) = GROUP_MEMBERS.write() {
        m.insert(group_id.to_string(), members);
    }
}
