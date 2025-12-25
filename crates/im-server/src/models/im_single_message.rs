use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ImSingleMessage {
    pub message_id: String,
    pub from_id: String,
    pub to_id: String,
    pub message_body: String,
    pub message_time: OffsetDateTime,
    pub message_content_type: i32,
    pub read_status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    pub del_flag: i16,
    pub sequence: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_random: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
}

impl ImSingleMessage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        message_id: String,
        from_id: String,
        to_id: String,
        message_body: String,
        message_time: OffsetDateTime,
        message_content_type: i32,
        read_status: i32,
        extra: Option<String>,
        del_flag: i16,
        sequence: i64,
        message_random: Option<String>,
        create_time: Option<OffsetDateTime>,
        update_time: Option<OffsetDateTime>,
        version: Option<i64>,
        reply_to: Option<String>,
        to_type: Option<String>,
        file_url: Option<String>,
        file_name: Option<String>,
        file_type: Option<String>,
    ) -> Self {
        Self {
            message_id,
            from_id,
            to_id,
            message_body,
            message_time,
            message_content_type,
            read_status,
            extra,
            del_flag,
            sequence,
            message_random,
            create_time,
            update_time,
            version,
            reply_to,
            to_type,
            file_url,
            file_name,
            file_type,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.message_id.is_empty() {
            return Err("消息ID为空".to_string());
        }
        if self.from_id.is_empty() {
            return Err("发送者ID为空".to_string());
        }
        if self.to_id.is_empty() {
            return Err("接收者ID为空".to_string());
        }
        if self.message_body.is_empty() {
            return Err("消息内容为空".to_string());
        }
        if self.message_id.len() > 512 {
            return Err(format!(
                "消息ID长度超过限制: {} > 512",
                self.message_id.len()
            ));
        }
        if self.from_id.len() > 50 {
            return Err(format!("发送者ID长度超过限制: {} > 50", self.from_id.len()));
        }
        if self.to_id.len() > 50 {
            return Err(format!("接收者ID长度超过限制: {} > 50", self.to_id.len()));
        }
        if let Some(ref message_random) = self.message_random
            && message_random.len() > 255
        {
            return Err(format!(
                "随机标识长度超过限制: {} > 255",
                message_random.len()
            ));
        }
        if let Some(ref reply_to) = self.reply_to
            && reply_to.len() > 255
        {
            return Err(format!("引用消息ID长度超过限制: {} > 255", reply_to.len()));
        }
        if let Some(ref to_type) = self.to_type
            && !["User", "Group"].contains(&to_type.as_str())
        {
            return Err(format!("接收者类型无效: {}, 应为 User 或 Group", to_type));
        }
        if let Some(ref file_url) = self.file_url
            && file_url.len() > 512
        {
            return Err(format!("文件URL长度超过限制: {} > 512", file_url.len()));
        }
        if let Some(ref file_name) = self.file_name
            && file_name.len() > 255
        {
            return Err(format!("文件名长度超过限制: {} > 255", file_name.len()));
        }
        if let Some(ref file_type) = self.file_type
            && file_type.len() > 64
        {
            return Err(format!("文件类型长度超过限制: {} > 64", file_type.len()));
        }
        Ok(())
    }
}
