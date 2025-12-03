use salvo::oapi::ToSchema;

use serde::{Deserialize, Serialize};

use crate::models::{SafeUser, im_friendship::ImFriendship, im_friendship::ImFriendshipRequest};

#[derive(Deserialize, ToSchema)]
pub struct CreateUserReq {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateImUserReq {
    pub user_id: String,
    pub user_name: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct HandleFriendshipRequests {
    pub approve_status: i32,
}

#[derive(Debug, Clone, Deserialize, ToSchema, Default)]
pub struct GetFriendshipRequests {
    pub approve_status: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GetFriendshipResp {
    pub friendship_req: ImFriendshipRequest,
    pub user: Option<SafeUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GetFriendsResp {
    pub friendship: ImFriendship,
    pub user: Option<SafeUser>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateRemarkReq {
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SimpleFriendshipResp {
    pub to_id: String,
    pub owner_id: String,
}

impl From<ImFriendship> for SimpleFriendshipResp {
    fn from(friendship: ImFriendship) -> Self {
        SimpleFriendshipResp {
            to_id: friendship.to_id,
            owner_id: friendship.owner_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddFriendRequest {
    pub to_id: String,
    pub remark: Option<String>,
    pub add_source: Option<String>,
    pub message: Option<String>, // 好友验证信息
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubscriptionInfoResp {
    pub user_id: i64,
    pub open_id: String,
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImGroupMessageStatus {
    pub group_id: String,
    pub message_id: String,
    pub to_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}
