use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_id: Option<String>,
    pub name: String,
    pub email: String,

    #[serde(skip_serializing)]
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
    pub fn new(id: i64, open_id: Option<String>, name: String, email: String) -> Self {
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
