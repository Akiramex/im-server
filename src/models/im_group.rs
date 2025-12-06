use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImGroup {
    pub group_id: String,
    pub owner_id: String,
    pub group_type: i32,
    pub group_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<i16>,
    pub apply_join_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_member_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    pub del_flag: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i64>,
}

impl ImGroup {
    pub fn validate(&self) -> Result<(), String> {
        if self.group_id.is_empty() {
            return Err("群组ID为空".to_string());
        }
        if self.group_name.is_empty() {
            return Err("群组名称为空".to_string());
        }
        if self.owner_id.is_empty() {
            return Err("群主ID为空".to_string());
        }

        // 检查 group_id 长度（数据库限制为 VARCHAR(50)）
        if self.group_id.len() > 50 {
            return Err(format!("群组ID长度超过限制: {} > 50", self.group_id.len()));
        }

        // 检查 group_name 长度（数据库限制为 VARCHAR(100)）
        if self.group_name.len() > 100 {
            return Err(format!(
                "群组名称长度超过限制: {} > 100",
                self.group_name.len()
            ));
        }

        // 检查 introduction 长度（数据库限制为 VARCHAR(100)）
        if let Some(ref intro) = self.introduction {
            if intro.len() > 100 {
                return Err(format!("群组简介长度超过限制: {} > 100", intro.len()));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImGroupMember {
    pub group_member_id: String,
    pub group_id: String,
    pub member_id: String,
    pub role: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speak_date: Option<OffsetDateTime>,
    pub mute: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    pub del_flag: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}
