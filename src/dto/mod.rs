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
    pub users: Vec<User>,
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

#[derive(Deserialize, ToSchema)]
pub struct LoginReq {
    pub username: String, // 支持用户名或邮箱登录
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResp {
    pub token: String,
}
