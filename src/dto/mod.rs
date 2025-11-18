use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}
