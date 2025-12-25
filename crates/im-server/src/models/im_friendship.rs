use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

impl ImFriendshipRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        from_id: String,
        to_id: String,
        remark: Option<String>,
        read_status: Option<i32>,
        add_source: Option<String>,
        message: Option<String>,
        approve_status: Option<i32>,
        create_time: Option<OffsetDateTime>,
        update_time: Option<OffsetDateTime>,
        sequence: Option<i64>,
        del_flag: Option<i16>,
        version: Option<i64>,
    ) -> Self {
        Self {
            id,
            from_id,
            to_id,
            remark,
            read_status,
            add_source,
            message,
            approve_status,
            create_time,
            update_time,
            sequence,
            del_flag,
            version,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("好友请求ID为空".to_string());
        }
        if self.from_id.is_empty() {
            return Err("发送者ID为空".to_string());
        }
        if self.to_id.is_empty() {
            return Err("接收者ID为空".to_string());
        }
        if self.from_id == self.to_id {
            return Err("发送者和接收者不能相同".to_string());
        }
        if self.id.len() > 100 {
            return Err(format!("好友请求ID长度超过限制: {} > 100", self.id.len()));
        }
        if self.from_id.len() > 100 {
            return Err(format!(
                "发送者ID长度超过限制: {} > 100",
                self.from_id.len()
            ));
        }
        if self.to_id.len() > 100 {
            return Err(format!("接收者ID长度超过限制: {} > 100", self.to_id.len()));
        }
        if let Some(ref remark) = self.remark
            && remark.len() > 100
        {
            return Err(format!("备注长度超过限制: {} > 100", remark.len()));
        }
        if let Some(ref message) = self.message
            && message.len() > 500
        {
            return Err(format!("验证消息长度超过限制: {} > 500", message.len()));
        }
        if let Some(ref add_source) = self.add_source
            && add_source.len() > 100
        {
            return Err(format!("添加来源长度超过限制: {} > 100", add_source.len()));
        }
        Ok(())
    }
}
