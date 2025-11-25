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

#[derive(Deserialize, ToSchema)]
pub struct LoginReq {
    pub username: String, // 支持用户名或邮箱登录
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResp {
    pub token: String,
}
