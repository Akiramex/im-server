use time::OffsetDateTime;

use crate::{db, models::ImOutbox, prelude::*};

pub async fn create(
    message_id: &str,
    payload: &str,
    exchange: &str,
    routing_key: &str,
) -> AppResult<ImOutbox> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO im_outbox
        (message_id, payload, exchange, routing_key, attempts, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 0, 'PENDING', $5, $6)
        RETURNING id
        "#,
        message_id,
        payload,
        exchange,
        routing_key,
        now,
        now
    )
    .fetch_one(conn)
    .await?;

    get_by_id(id).await
}

/// 根据ID获取发件箱记录
pub async fn get_by_id(id: i64) -> AppResult<ImOutbox> {
    let conn = db::pool();
    let outbox = sqlx::query_as!(
        ImOutbox,
        r#"
        SELECT id, message_id, payload, exchange, routing_key, attempts, status,
        last_error, created_at, updated_at, next_try_at
        FROM im_outbox
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(conn)
    .await?;

    match outbox {
        Some(o) => Ok(o),
        None => Err(AppError::NotFound(None)),
    }
}

/// 更新发件箱状态
pub async fn update_status(id: i64, status: &str) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
        UPDATE im_outbox
        SET status = $1,
            updated_at = $2
        WHERE id = $3
        "#,
        status,
        now,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 增加尝试次数并记录错误
#[allow(dead_code)]
pub async fn increment_attempts(id: i64, error: Option<&str>) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
        UPDATE im_outbox
        SET attempts = attempts + 1,
            last_error = $1,
            updated_at = $2
        WHERE id = $3
        "#,
        error,
        now,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 设置下次重试时间
#[allow(dead_code)]
pub async fn set_next_try_at(id: i64, next_try_at: Option<OffsetDateTime>) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
        UPDATE im_outbox
        SET next_try_at = $1, updated_at = $2
        WHERE id = $3
        "#,
        next_try_at,
        now,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 标记为已发送
pub async fn mark_sent(id: i64) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
        UPDATE im_outbox
        SET status = 'SENT', updated_at = $1
        WHERE id = $2
        "#,
        now,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 获取待发送的消息
pub async fn get_pending_messages(limit: i64) -> AppResult<Vec<ImOutbox>> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    let messages = sqlx::query_as!(
        ImOutbox,
        r#"
        SELECT id, message_id, payload, exchange, routing_key, attempts, status,
               last_error, created_at, updated_at, next_try_at
        FROM im_outbox
        WHERE status = 'PENDING'
        AND (next_try_at IS NULL OR next_try_at <= $1)
        ORDER BY created_at ASC
        LIMIT $2
        "#,
        now,
        limit
    )
    .fetch_all(conn)
    .await?;

    Ok(messages)
}

/// 获取失败的消息
pub async fn get_failed_messages(limit: i64) -> AppResult<Vec<ImOutbox>> {
    let conn = db::pool();

    let messages = sqlx::query_as!(
        ImOutbox,
        r#"
        SELECT id, message_id, payload, exchange, routing_key, attempts, status,
               last_error, created_at, updated_at, next_try_at
        FROM im_outbox
        WHERE status = 'FAILED'
        ORDER BY updated_at DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(conn)
    .await?;

    Ok(messages)
}
