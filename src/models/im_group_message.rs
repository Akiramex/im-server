use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ImGroupMessage {
    pub message_id: String,
    pub group_id: String,
    pub from_id: String,
    pub message_body: String,
    pub message_time: OffsetDateTime,
    pub message_content_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    pub del_flag: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
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
}

impl ImGroupMessage {
    pub fn new(
        message_id: String,
        group_id: String,
        from_id: String,
        message_body: String,
        message_time: OffsetDateTime,
        message_content_type: i32,
        extra: Option<String>,
        del_flag: i16,
        sequence: Option<i64>,
        message_random: Option<String>,
        create_time: Option<OffsetDateTime>,
        update_time: Option<OffsetDateTime>,
        version: Option<i64>,
        reply_to: Option<String>,
    ) -> Self {
        Self {
            message_id,
            group_id,
            from_id,
            message_body,
            message_time,
            message_content_type,
            extra,
            del_flag,
            sequence,
            message_random,
            create_time,
            update_time,
            version,
            reply_to,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.message_id.is_empty() {
            return Err("消息ID为空".to_string());
        }
        if self.group_id.is_empty() {
            return Err("群组ID为空".to_string());
        }
        if self.from_id.is_empty() {
            return Err("发送者ID为空".to_string());
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
        if self.group_id.len() > 255 {
            return Err(format!("群组ID长度超过限制: {} > 255", self.group_id.len()));
        }
        if self.from_id.len() > 20 {
            return Err(format!("发送者ID长度超过限制: {} > 20", self.from_id.len()));
        }
        if let Some(ref message_random) = self.message_random {
            if message_random.len() > 255 {
                return Err(format!(
                    "随机标识长度超过限制: {} > 255",
                    message_random.len()
                ));
            }
        }
        if let Some(ref reply_to) = self.reply_to {
            if reply_to.len() > 255 {
                return Err(format!("引用消息ID长度超过限制: {} > 255", reply_to.len()));
            }
        }
        Ok(())
    }
}
