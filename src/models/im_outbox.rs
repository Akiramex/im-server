use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ImOutbox {
    pub id: i64,
    pub message_id: String,
    pub payload: String,
    pub exchange: String,
    pub routing_key: String,
    pub attempts: i32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_try_at: Option<OffsetDateTime>,
}
