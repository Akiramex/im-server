use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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
    pub fn new(
        id: i64,
        open_id: String,
        name: String,
        email: String,
        password_hash: Option<String>,
    ) -> Self {
        User {
            id,
            open_id,
            name,
            email,
            password_hash,
            file_name: None,
            abstract_field: None,
            phone: None,
            status: None,
            gender: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SafeUser {
    pub id: i64,
    pub open_id: String,
    pub name: String,
    pub email: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "abstract"))]
    pub abstract_field: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<i32>,
}

impl From<User> for SafeUser {
    fn from(val: User) -> Self {
        SafeUser {
            id: val.id,
            open_id: val.open_id,
            name: val.name,
            email: val.email,
            file_name: val.file_name,
            abstract_field: val.abstract_field,
            phone: val.phone,
            status: val.status,
            gender: val.gender,
        }
    }
}
