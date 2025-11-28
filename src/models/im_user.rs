use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImUser {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub del_flag: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ImSafeUser {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub del_flag: Option<i32>,
}

impl From<ImUser> for ImSafeUser {
    fn from(value: ImUser) -> Self {
        ImSafeUser {
            user_id: value.user_id,
            user_name: value.user_name,
            mobile: value.mobile,
            create_time: value.create_time,
            update_time: value.update_time,
            version: value.version,
            del_flag: value.del_flag,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ImUserData {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_signature: Option<String>,
    pub friend_allow_type: i32,
    pub forbidden_flag: i32,
    pub disable_add_friend: i32,
    pub silent_flag: i32,
    pub user_type: i32,
    pub del_flag: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}
