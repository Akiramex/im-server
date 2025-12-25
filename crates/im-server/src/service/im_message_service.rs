use crate::{
    db,
    dto::ImGroupMessageStatus,
    models::{ImGroupMessage, ImSingleMessage},
    prelude::*,
};
use im_share::redis::RedisClient;
use time::OffsetDateTime;

static USE_REDIS: bool = true;

pub async fn save_single_message(message: ImSingleMessage) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();
    sqlx::query!(
        r#"
        INSERT INTO im_single_message
         (message_id, from_id, to_id, message_body, message_time, message_content_type,
          read_status, extra, del_flag, sequence, message_random, create_time, update_time, version, reply_to,
          to_type, file_url, file_name, file_type)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, 1, $8, $9, $10, $11, 1, $12, $13, $14, $15, $16)
         ON CONFLICT (message_id) DO UPDATE SET
         message_id = EXCLUDED.message_id
        "#,
        message.message_id,
        message.from_id,
        message.to_id,
        message.message_body,
        message.message_time,
        message.message_content_type,
        message.extra,
        message.sequence,
        message.message_random,
        now,
        now,
        message.reply_to,
        message.to_type,
        message.file_url,
        message.file_name,
        message.file_type
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 获取单聊消息列表
/// 重要：过滤掉通话邀请消息（message_content_type = 4），因为通话邀请是实时消息，过期后没有意义
pub async fn get_single_messages(
    from_id: &str,
    to_id: &str,
    since_sequence: Option<i64>,
    limit: i32,
) -> AppResult<Vec<ImSingleMessage>> {
    let conn = db::pool();

    let mut query = "SELECT message_id, from_id, to_id, message_body, message_time, message_content_type,
                            read_status, extra, del_flag, sequence, message_random, create_time, update_time, version, reply_to,
                            to_type, file_url, file_name, file_type
                     FROM im_single_message
                     WHERE ((from_id = ? AND to_id = ?) OR (from_id = ? AND to_id = ?))
                     AND del_flag = 1 AND message_content_type != 4".to_string();

    if let Some(seq) = since_sequence {
        query.push_str(&format!(" AND sequence > {}", seq));
    }

    query.push_str(" ORDER BY sequence ASC LIMIT ?");

    let messages = sqlx::query_as::<_, ImSingleMessage>(&query)
        .bind(from_id)
        .bind(to_id)
        .bind(to_id)
        .bind(from_id)
        .bind(limit)
        .fetch_all(conn)
        .await?;

    Ok(messages)
}

/// 标记消息为已读
pub async fn mark_single_message_read(message_id: &str, to_id: &str) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
            UPDATE im_single_message
            SET read_status = 1, update_time = $1, version = version + 1
            WHERE message_id = $2 AND to_id = $3
        "#,
        now,
        message_id,
        to_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 保存群聊消息
pub async fn save_group_message(message: ImGroupMessage) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();
    sqlx::query!(
        r#"
        INSERT INTO im_group_message
         (message_id, group_id, from_id, message_body, message_time, message_content_type,
          extra, del_flag, sequence, message_random, create_time, update_time, version, reply_to)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9, $10, $11, 1, $12)
         ON CONFLICT (message_id) DO UPDATE SET
         message_id = EXCLUDED.message_id
        "#,
        message.message_id,
        message.group_id,
        message.from_id,
        message.message_body,
        message.message_time,
        message.message_content_type,
        message.extra,
        message.sequence,
        message.message_random,
        now,
        now,
        message.reply_to
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 获取群聊消息列表
/// 重要：过滤掉通话邀请消息（message_content_type = 4），因为通话邀请是实时消息，过期后没有意义
pub async fn get_group_messages(
    group_id: &str,
    since_sequence: Option<i64>,
    limit: i32,
) -> AppResult<Vec<ImGroupMessage>> {
    let conn = db::pool();
    let mut query = "SELECT message_id, group_id, from_id, message_body, message_time, message_content_type,
                            extra, del_flag, sequence, message_random, create_time, update_time, version, reply_to
                     FROM im_group_message
                     WHERE group_id = ? AND del_flag = 1 AND message_content_type != 4".to_string();

    if let Some(seq) = since_sequence {
        query.push_str(&format!(" AND sequence > {}", seq));
    }

    query.push_str(" ORDER BY sequence ASC LIMIT ?");

    let messages = sqlx::query_as::<_, ImGroupMessage>(&query)
        .bind(group_id)
        .bind(limit)
        .fetch_all(conn)
        .await?;

    Ok(messages)
}

/// 标记群消息为已读（使用 Redis）
pub async fn mark_group_message_read(
    group_id: &str,
    message_id: &str,
    to_id: &str,
) -> AppResult<()> {
    if USE_REDIS {
        RedisClient::mark_group_message_read(group_id, message_id, to_id)
            .await
            .map_err(|_| AppError::internal("Redis 操作失败"))?;
        Ok(())
    } else {
        Err(AppError::internal("Redis 状态异常"))
    }
}

/// 获取群消息的已读状态（使用 Redis）
pub async fn get_group_message_status(
    group_id: &str,
    message_id: &str,
) -> AppResult<Vec<ImGroupMessageStatus>> {
    if USE_REDIS {
        let user_ids = RedisClient::get_group_message_read_users(group_id, message_id)
            .await
            .map_err(|_| AppError::internal("Redis 操作失败"))?;
        let timestamp = OffsetDateTime::now_utc();
        // 转换为 ImGroupMessageStatus 格式
        let statuses = user_ids
            .into_iter()
            .map(|to_id| ImGroupMessageStatus {
                group_id: group_id.to_string(),
                message_id: message_id.to_string(),
                to_id,
                read_status: Some(1),
                create_time: Some(timestamp.unix_timestamp() * 1000),
                update_time: Some(timestamp.unix_timestamp() * 1000),
                version: Some(1),
            })
            .collect();

        Ok(statuses)
    } else {
        Err(AppError::internal("Redis 状态异常"))
    }
}

/// 获取用户在群组中的消息已读状态（使用 Redis）
/// 注意：这个方法在 Redis 模式下需要遍历所有消息，性能可能不如数据库
/// 建议使用 get_group_message_read_count 来获取已读数量
pub async fn get_user_group_message_status(
    _group_id: &str,
    _to_id: &str,
    _limit: Option<i32>,
) -> AppResult<Vec<ImGroupMessageStatus>> {
    // Redis 模式下，这个方法不太实用，因为需要遍历所有消息
    // 暂时返回空列表，或者可以从数据库获取消息列表，然后检查 Redis
    if USE_REDIS {
        Ok(vec![])
    } else {
        Err(AppError::internal("Redis 状态异常"))
    }
}

/// 检查用户是否已读群消息（使用 Redis）
#[allow(dead_code)]
pub async fn is_group_message_read(
    group_id: &str,
    message_id: &str,
    to_id: &str,
) -> AppResult<bool> {
    if USE_REDIS {
        let result = RedisClient::is_group_message_read(group_id, message_id, to_id)
            .await
            .map_err(|_| AppError::internal("Redis 操作失败"))?;
        Ok(result)
    } else {
        Err(AppError::internal("Redis 状态异常"))
    }
}

/// 获取群消息的已读数量（使用 Redis）
#[allow(dead_code)]
pub async fn get_group_message_read_count(group_id: &str, message_id: &str) -> AppResult<usize> {
    if USE_REDIS {
        let count = RedisClient::get_group_message_read_count(group_id, message_id)
            .await
            .map_err(|_| AppError::internal("Redis 操作失败"))?;
        Ok(count)
    } else {
        Err(AppError::internal("Redis 状态异常"))
    }
}
