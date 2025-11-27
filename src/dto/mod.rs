use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

use crate::models::User;

#[derive(Deserialize, ToSchema)]
pub struct CreateUserReq {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
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

impl Into<SafeUser> for User {
    fn into(self) -> SafeUser {
        SafeUser {
            id: self.id,
            open_id: self.open_id,
            name: self.name,
            email: self.email,
            file_name: self.file_name,
            abstract_field: self.abstract_field,
            phone: self.phone,
            status: self.status,
            gender: self.gender,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserListQuery {
    pub username: Option<String>,
    #[serde(default = "default_page")]
    pub current_page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResp {
    pub users: Vec<SafeUser>,
    pub total: i64,
    pub current_page: i64,
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    10
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserReq {
    pub name: Option<String>,
    pub file_name: Option<String>,
    pub abstract_field: Option<String>,
    pub phone: Option<String>,
    pub gender: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginReq {
    pub username: String, // 支持用户名或邮箱登录
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResp {
    pub token: String,
}
