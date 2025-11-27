mod resp;
pub use resp::MyResponse;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub open_id: String,
    pub name: String,
    pub email: String,

    pub password_hash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "abstract"))]
    // 序列化时使用 abstract，反序列化时仍接受 abstract_field
    pub abstract_field: Option<String>, // abstract 是 Rust 关键字，使用 abstract_field

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<i32>,
}

impl User {
    pub fn new(id: i64, open_id: String, name: String, email: String) -> Self {
        User {
            id,
            open_id,
            name,
            email,
            password_hash: None,
            file_name: None,
            abstract_field: None,
            phone: None,
            status: None,
            gender: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImFriendship {
    pub owner_id: String,
    pub to_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub del_flag: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub black: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub black_sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImFriendshipRequest {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approve_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub del_flag: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}
