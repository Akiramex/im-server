use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub username: String, // 支持用户名或邮箱登录
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub token: String,
}
