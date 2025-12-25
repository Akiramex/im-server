use crate::models::ChatWithName;
use crate::prelude::*;
use crate::{db, models::ImChat};
use serde_json::json;
use std::collections::HashMap;

/// 获取或创建聊天会话
pub async fn get_or_create_chat(
    chat_id: String,
    chat_type: i32,
    owner_id: String,
    to_id: String,
) -> AppResult<ImChat> {
    let conn = db::pool();
    let now = time::OffsetDateTime::now_utc();
    // 尝试获取现有会话（同时考虑 chat_id、owner_id 和 chat_type，确保类型正确）
    // 重要：必须检查 chat_type，避免单聊和群聊混淆
    let chat = sqlx::query_as!(ImChat,
        r#"
        SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                read_sequence, remark, create_time, update_time, del_flag, version
         FROM im_chat
         WHERE chat_id = $1 AND owner_id = $2 AND chat_type = $3 AND (del_flag IS NULL OR del_flag = 1)
         "#,
         chat_id,
         owner_id,
         chat_type
    )
    .fetch_optional(conn)
    .await?;

    if let Some(c) = chat {
        // 如果找到的记录 chat_type 匹配，直接返回
        // 但如果 to_id 不同，可能需要更新（这种情况应该很少见）
        if c.to_id != to_id {
            warn!(
                chat_id = %chat_id,
                owner_id = %owner_id,
                chat_type = %chat_type,
                existing_to_id = %c.to_id,
                new_to_id = %to_id,
                "找到的聊天记录 to_id 不匹配，但 chat_type 匹配，返回现有记录"
            );
        }
        return Ok(c);
    }

    // 检查是否存在相同 chat_id 和 owner_id 但 chat_type 不同的记录
    // 如果存在，说明数据不一致，需要修复或报错
    let conflicting_chat = sqlx::query_as!(ImChat,
        r#"
        SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                read_sequence, remark, create_time, update_time, del_flag, version
         FROM im_chat
         WHERE chat_id = $1 AND owner_id = $2 AND chat_type != $3 AND (del_flag IS NULL OR del_flag = 1)
         "#,
         chat_id,
         owner_id,
         chat_type
    )
    .fetch_optional(conn)
    .await?;

    if let Some(conflicting) = conflicting_chat {
        // 发现冲突：相同 chat_id 和 owner_id 但 chat_type 不同
        // 这种情况不应该发生，但为了数据一致性，我们尝试更新现有记录的 chat_type
        // 如果更新失败，我们仍然返回现有记录，确保消息能正常发送和接收
        warn!(
            chat_id = %chat_id,
            owner_id = %owner_id,
            expected_chat_type = %chat_type,
            existing_chat_type = %conflicting.chat_type,
            "发现聊天记录类型冲突，尝试更新 chat_type 以修复数据不一致"
        );

        // 尝试更新 chat_type 和 to_id
        // 注意：即使更新失败，我们也会继续处理，确保消息能正常发送
        let update_result = sqlx::query!(
            r#"
            UPDATE im_chat
             SET chat_type = $1, to_id = $2, update_time = $3, version = version + 1
             WHERE chat_id = $4 AND owner_id = $5
             "#,
            chat_type,
            &to_id,
            now,
            &chat_id,
            &owner_id
        )
        .execute(conn)
        .await;

        match update_result {
            Ok(_) => {
                // 更新成功，返回更新后的记录
                info!(
                    chat_id = %chat_id,
                    owner_id = %owner_id,
                    chat_type = %chat_type,
                    "成功更新聊天记录类型"
                );

                let updated_chat = sqlx::query_as!(ImChat,
                    r#"
                    SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                            read_sequence, remark, create_time, update_time, del_flag, version
                     FROM im_chat
                     WHERE chat_id = $1 AND owner_id = $2 AND chat_type = $3 AND (del_flag IS NULL OR del_flag = 1)
                     "#,
                     chat_id,
                     owner_id,
                     chat_type
                )
                .fetch_one(conn)
                .await;

                match updated_chat {
                    Ok(chat) => return Ok(chat),
                    Err(e) => {
                        // 查询更新后的记录失败，但仍然返回冲突的记录，确保消息能正常发送
                        warn!(
                            chat_id = %chat_id,
                            owner_id = %owner_id,
                            error = %e,
                            "查询更新后的聊天记录失败，返回冲突记录以确保消息能正常发送"
                        );
                        // 返回冲突的记录，但修改 chat_type 为期望的值
                        // 这样前端可以正常显示，虽然数据库中的值可能还是旧的
                        return Ok(ImChat {
                            chat_id: conflicting.chat_id,
                            chat_type, // 使用期望的 chat_type
                            owner_id: conflicting.owner_id,
                            to_id, // 使用新的 to_id
                            is_mute: conflicting.is_mute,
                            is_top: conflicting.is_top,
                            sequence: conflicting.sequence,
                            read_sequence: conflicting.read_sequence,
                            remark: conflicting.remark,
                            create_time: conflicting.create_time,
                            update_time: Some(now),
                            del_flag: conflicting.del_flag,
                            version: conflicting.version.map(|v| v + 1),
                        });
                    }
                }
            }
            Err(e) => {
                // 更新失败，但仍然返回冲突的记录，确保消息能正常发送
                // 这样可以避免因为数据不一致导致消息发送失败
                warn!(
                    chat_id = %chat_id,
                    owner_id = %owner_id,
                    error = %e,
                    "更新聊天记录类型失败，返回冲突记录以确保消息能正常发送（数据库中的 chat_type 可能仍然是旧的）"
                );
                // 返回冲突的记录，但修改 chat_type 为期望的值
                // 这样前端可以正常显示，虽然数据库中的值可能还是旧的
                return Ok(ImChat {
                    chat_id: conflicting.chat_id,
                    chat_type, // 使用期望的 chat_type
                    owner_id: conflicting.owner_id,
                    to_id, // 使用新的 to_id
                    is_mute: conflicting.is_mute,
                    is_top: conflicting.is_top,
                    sequence: conflicting.sequence,
                    read_sequence: conflicting.read_sequence,
                    remark: conflicting.remark,
                    create_time: conflicting.create_time,
                    update_time: Some(now),
                    del_flag: conflicting.del_flag,
                    version: conflicting.version.map(|v| v + 1),
                });
            }
        }
    }

    // 创建新会话
    // 如果插入失败（可能是并发插入导致的唯一约束冲突），再次尝试查询
    let insert_result = sqlx::query!(
        r#"
        INSERT INTO im_chat
         (chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence, read_sequence, remark,
          create_time, update_time, del_flag, version)
          VALUES ($1, $2, $3, $4, 0, 0, 0, 0, NULL, $5, $6, 1, 1)
          "#,
        &chat_id,
        chat_type,
        &owner_id,
        &to_id,
        now,
        now
    )
    .execute(conn)
    .await;

    match insert_result {
        Ok(_) => {
            // 插入成功，返回新创建的记录
            Ok(ImChat {
                chat_id,
                chat_type,
                owner_id: Some(owner_id),
                to_id,
                is_mute: 0,
                is_top: 0,
                sequence: Some(0),
                read_sequence: Some(0),
                remark: None,
                create_time: Some(now),
                update_time: Some(now),
                del_flag: Some(1),
                version: Some(1),
            })
        }
        Err(e) => {
            // 插入失败，可能是并发插入导致的唯一约束冲突
            // 检查错误是否是主键/唯一键冲突
            let error_msg = e.to_string();
            let is_duplicate_key = error_msg.contains("Duplicate entry")
                || error_msg.contains("1062")
                || error_msg.contains("UNIQUE constraint");

            if is_duplicate_key {
                // 再次尝试查询，获取已存在的记录
                warn!(chat_id = %chat_id, owner_id = %owner_id, error = %e, "插入聊天记录失败，可能是并发冲突或表结构问题，尝试重新查询");

                // 先尝试精确匹配 (chat_id, owner_id, chat_type)
                let chat = sqlx::query_as!(ImChat,
                    r#"
                    SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                            read_sequence, remark, create_time, update_time, del_flag, version
                     FROM im_chat
                     WHERE chat_id = $1 AND owner_id = $2 AND chat_type = $3 AND (del_flag IS NULL OR del_flag = 1)
                     "#,
                     &chat_id,
                     &owner_id,
                     chat_type
                )
                .fetch_optional(conn)
                .await?;

                if let Some(c) = chat {
                    // 找到了已存在的记录，返回它
                    return Ok(c);
                }

                // 如果精确匹配找不到，可能是表结构问题（主键只有 chat_id）
                // 尝试只按 chat_id 查询，然后更新 owner_id
                warn!(chat_id = %chat_id, owner_id = %owner_id, "精确匹配未找到记录，可能是表结构问题，尝试按 chat_id 查询");

                let existing_chat = sqlx::query_as!(
                    ImChat,
                    r#"
                    SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                            read_sequence, remark, create_time, update_time, del_flag, version
                     FROM im_chat
                     WHERE chat_id = $1 AND (del_flag IS NULL OR del_flag = 1)
                     LIMIT 1
                     "#,
                    chat_id
                )
                .fetch_optional(conn)
                .await?;

                if let Some(existing) = existing_chat {
                    // 如果找到的记录 owner_id 不同，说明表结构有问题（主键只有 chat_id）
                    // 这种情况下，我们无法为同一个 chat_id 创建多条记录
                    // 只能返回错误，提示需要更新表结构
                    if existing.owner_id.as_deref() != Some(&owner_id) {
                        error!(
                            chat_id = %chat_id,
                            owner_id = %owner_id,
                            existing_owner_id = ?existing.owner_id,
                            "表结构问题：主键只有 chat_id，无法为同一 chat_id 创建不同 owner_id 的记录。请执行 fix_im_chat_primary_key.sql 更新表结构"
                        );
                        return Err(AppError::internal("表结构问题"));
                    }
                    // 如果 owner_id 相同，返回现有记录
                    return Ok(existing);
                }
            }

            // 其他类型的错误或找不到记录
            error!(chat_id = %chat_id, owner_id = %owner_id, error = %e, "创建聊天记录失败且无法找到已存在的记录");
            Err(AppError::internal("创建聊天记录失败且无法找到已存在的记录"))
        }
    }
}

/// 获取用户的聊天会话列表
#[allow(dead_code)]
pub async fn get_user_chats(owner_id: &str) -> AppResult<Vec<ImChat>> {
    let conn = db::pool();
    let chats = sqlx::query_as!(
        ImChat,
        r#"
        SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                read_sequence, remark, create_time, update_time, del_flag, version
         FROM im_chat
         WHERE owner_id = $1 AND (del_flag IS NULL OR del_flag = 1)
         ORDER BY is_top DESC, update_time DESC
         "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    Ok(chats)
}

/// 获取用户的聊天会话列表（包含名称信息）
pub async fn get_user_chats_with_names(owner_id: &str) -> AppResult<Vec<ChatWithName>> {
    let conn = db::pool();

    // 先检查用户所在的群组，为没有聊天记录的群组自动创建聊天记录
    // 这样可以确保所有群组都会出现在聊天列表中
    let group_rows = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT g.group_id
        FROM im_group g
        INNER JOIN im_group_member gm ON g.group_id = gm.group_id AND gm.del_flag = 1
        WHERE gm.member_id = $1 AND g.del_flag = 1
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    // 为每个群组创建聊天记录（如果还没有的话）
    for group_id in group_rows {
        let chat_id = format!("group_{}", group_id);
        // 使用 get_or_create_chat 方法，如果已存在则不会重复创建
        if let Err(e) =
            get_or_create_chat(chat_id.clone(), 2, owner_id.to_string(), group_id.clone()).await
        {
            warn!(chat_id = %chat_id, owner_id = %owner_id, group_id = %group_id, error = ?e, "为群组创建聊天记录失败（不影响获取聊天列表）");
        }
    }

    // 使用 LEFT JOIN 关联查询群组表和用户表
    // 对于群组（chat_type = 2），关联 im_group 表获取群组名称和人数
    // 对于单聊（chat_type = 1），关联 users 表获取用户名
    // 注意：对于群聊，如果群组已解散（g.group_id IS NULL），直接在 SQL 中过滤掉
    let rows = sqlx::query!(
        r#"
        SELECT
            c.chat_id, c.chat_type, c.owner_id, c.to_id, c.is_mute, c.is_top,
            c.sequence, c.read_sequence, c.create_time, c.update_time, c.del_flag, c.version,
            CASE
                WHEN c.chat_type = 2 AND g.group_name IS NOT NULL AND g.group_name != '' THEN g.group_name
                WHEN c.chat_type = 1 AND u.name IS NOT NULL AND u.name != '' THEN u.name
                ELSE NULL
            END as name,
            CASE
                WHEN c.chat_type = 2 THEN CAST((
                    SELECT COUNT(*)
                    FROM im_group_member gm
                    WHERE gm.group_id = c.to_id AND gm.del_flag = 1
                ) AS INTEGER)
                ELSE NULL
            END as member_count
        FROM im_chat c
        LEFT JOIN im_group g ON c.chat_type = 2 AND c.to_id = g.group_id AND g.del_flag = 1
        LEFT JOIN users u ON c.chat_type = 1 AND (c.to_id = u.open_id OR c.to_id = u.name) AND (u.status IS NULL OR u.status = 1)
        WHERE c.owner_id = $1
        AND (c.del_flag IS NULL OR c.del_flag = 1)
        AND (c.chat_type != 2 OR g.group_id IS NOT NULL)
        ORDER BY c.is_top DESC, c.update_time DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    let now = time::OffsetDateTime::now_utc();
    let mut chats = Vec::new();

    for row in rows {
        let chat_id: String = row.chat_id;
        let mut chat_type: i32 = row.chat_type;
        let to_id: String = row.to_id;

        // 数据修复：如果 chat_id 以 "single_" 开头但 chat_type = 2，说明数据不一致
        // 自动修复为 chat_type = 1（单聊）
        if chat_id.starts_with("single_") && chat_type == 2 {
            warn!(
                chat_id = %chat_id,
                owner_id = %owner_id,
                to_id = %to_id,
                "发现数据不一致：chat_id 以 single_ 开头但 chat_type = 2，自动修复为单聊"
            );

            // 尝试更新数据库中的 chat_type
            let update_result = sqlx::query!(
                r#"
                UPDATE im_chat
                 SET chat_type = 1, update_time = $1, version = version + 1
                 WHERE chat_id = $2 AND owner_id = $3 AND chat_type = 2
                 "#,
                now,
                &chat_id,
                owner_id
            )
            .execute(conn)
            .await;

            match update_result {
                Ok(_) => {
                    info!(
                        chat_id = %chat_id,
                        owner_id = %owner_id,
                        "成功修复聊天记录类型：从群聊改为单聊"
                    );
                    chat_type = 1; // 使用修复后的类型
                }
                Err(e) => {
                    warn!(
                        chat_id = %chat_id,
                        owner_id = %owner_id,
                        error = %e,
                        "修复聊天记录类型失败，但会在返回时使用正确的类型"
                    );
                    chat_type = 1; // 即使更新失败，也使用正确的类型返回
                }
            }
        }

        // 数据修复：如果 chat_id 以 "group_" 开头但 chat_type = 1，说明数据不一致
        // 自动修复为 chat_type = 2（群聊）
        if chat_id.starts_with("group_") && chat_type == 1 {
            warn!(
                chat_id = %chat_id,
                owner_id = %owner_id,
                to_id = %to_id,
                "发现数据不一致：chat_id 以 group_ 开头但 chat_type = 1，自动修复为群聊"
            );

            // 尝试更新数据库中的 chat_type
            let update_result = sqlx::query!(
                r#"
                UPDATE im_chat
                 SET chat_type = 2, update_time = $1, version = version + 1
                 WHERE chat_id = $2 AND owner_id = $3 AND chat_type = 1
                 "#,
                now,
                &chat_id,
                owner_id
            )
            .execute(conn)
            .await;

            match update_result {
                Ok(_) => {
                    info!(
                        chat_id = %chat_id,
                        owner_id = %owner_id,
                        "成功修复聊天记录类型：从单聊改为群聊"
                    );
                    chat_type = 2; // 使用修复后的类型
                }
                Err(e) => {
                    warn!(
                        chat_id = %chat_id,
                        owner_id = %owner_id,
                        error = %e,
                        "修复聊天记录类型失败，但会在返回时使用正确的类型"
                    );
                    chat_type = 2; // 即使更新失败，也使用正确的类型返回
                }
            }
        }

        chats.push(ChatWithName {
            chat_id,
            chat_type,
            owner_id: Some(row.owner_id),
            to_id,
            is_mute: row.is_mute,
            is_top: row.is_top,
            sequence: row.sequence,
            read_sequence: row.read_sequence,
            create_time: row.create_time,
            update_time: row.update_time,
            del_flag: row.del_flag,
            version: row.version,
            name: row.name,
            member_count: row.member_count,
        });
    }

    Ok(chats)
}

/// 更新会话序列号
#[allow(dead_code)]
pub async fn update_chat_sequence(chat_id: &str, sequence: i64) -> AppResult<()> {
    let conn = db::pool();

    let now = time::OffsetDateTime::now_utc();

    sqlx::query!(
        r#"UPDATE im_chat
         SET sequence = $1, update_time = $2, version = version + 1
         WHERE chat_id = $3"#,
        sequence,
        now,
        chat_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 更新已读序列号
/// 更新群聊备注（仅自己可见）
pub async fn update_chat_remark(
    chat_id: &str,
    owner_id: &str,
    remark: Option<String>,
) -> AppResult<()> {
    let conn = db::pool();

    let now = time::OffsetDateTime::now_utc();

    sqlx::query!(
        r#"
        UPDATE im_chat
         SET remark = $1, update_time = $2, version = version + 1
         WHERE chat_id = $3 AND owner_id = $4
         "#,
        remark,
        now,
        chat_id,
        owner_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_read_sequence(chat_id: &str, read_sequence: i64) -> AppResult<()> {
    let conn = db::pool();

    sqlx::query!(
        r#"
        UPDATE im_chat
         SET read_sequence = $1, update_time = $2, version = version + 1
         WHERE chat_id = $3
         "#,
        read_sequence,
        time::OffsetDateTime::now_utc(),
        chat_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 设置会话置顶
pub async fn set_chat_top(chat_id: &str, is_top: i16) -> AppResult<()> {
    let conn = db::pool();

    sqlx::query!(
        r#"
        UPDATE im_chat
         SET is_top = $1, update_time = $2, version = version + 1
         WHERE chat_id = $3
         "#,
        is_top,
        time::OffsetDateTime::now_utc(),
        chat_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 设置会话免打扰
pub async fn set_chat_mute(chat_id: &str, is_mute: i16) -> AppResult<()> {
    let conn = db::pool();

    sqlx::query!(
        r#"
        UPDATE im_chat
         SET is_mute = $1, update_time = $2, version = version + 1
         WHERE chat_id = $3
         "#,
        is_mute,
        time::OffsetDateTime::now_utc(),
        chat_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 删除聊天会话（软删除）
/// 同时删除相关的消息记录
pub async fn delete_chat(chat_id: &str, owner_id: &str) -> AppResult<()> {
    let conn = db::pool();

    // 先获取聊天信息，获取 to_id（对方用户ID）
    let chat = sqlx::query_as!(
        ImChat,
        r#"
        SELECT chat_id, chat_type, owner_id, to_id, is_mute, is_top, sequence,
                read_sequence, remark, create_time, update_time, del_flag, version
         FROM im_chat
         WHERE chat_id = $1 AND owner_id = $2
         "#,
        chat_id,
        owner_id
    )
    .fetch_optional(conn)
    .await?;

    let now = time::OffsetDateTime::now_utc();
    // 删除聊天会话（软删除）
    sqlx::query!(
        r#"UPDATE im_chat
         SET del_flag = 0, update_time = $1, version = version + 1
         WHERE chat_id = $2 AND owner_id = $3
        "#,
        now,
        chat_id,
        owner_id
    )
    .execute(conn)
    .await?;

    // 如果找到了聊天记录，同时删除相关的消息记录
    if let Some(chat) = chat {
        // 删除单聊消息（软删除，设置 del_flag = 0）
        // 删除所有与 owner_id 和 to_id 相关的消息（双向）
        sqlx::query!(
            r#"
            UPDATE im_single_message
             SET del_flag = 0, update_time = $1, version = version + 1
             WHERE ((from_id = $2 AND to_id = $3) OR (from_id = $3 AND to_id = $2))
             AND del_flag = 1
             "#,
            now,
            owner_id,
            chat.to_id
        )
        .execute(conn)
        .await?;
    }

    Ok(())
}

/// 获取未读消息统计
/// 改进：不仅查询 im_chat 表，还直接查询消息表，确保即使没有聊天记录也能获取离线消息
pub async fn get_unread_message_stats(owner_id: &str) -> AppResult<serde_json::Value> {
    let conn = db::pool();

    // 首先，从 im_chat 表获取有未读消息的聊天会话（传统方式）
    let chat_rows = sqlx::query!(
        r#"
        SELECT
            c.chat_id,
            c.chat_type,
            c.to_id,
            c.sequence,
            c.read_sequence,
            CASE
                WHEN c.chat_type = 2 AND g.group_name IS NOT NULL AND g.group_name != '' THEN g.group_name
                WHEN c.chat_type = 1 AND u.name IS NOT NULL AND u.name != '' THEN u.name
                ELSE c.to_id
            END as name
        FROM im_chat c
        LEFT JOIN im_group g ON c.chat_type = 2 AND c.to_id = g.group_id AND g.del_flag = 1
        LEFT JOIN users u ON c.chat_type = 1 AND (c.to_id = u.open_id OR c.to_id = u.name) AND (u.status IS NULL OR u.status = 1)
        WHERE c.owner_id = $1
        AND (c.del_flag IS NULL OR c.del_flag = 1)
        AND (c.chat_type != 2 OR g.group_id IS NOT NULL)
        AND c.sequence IS NOT NULL
        AND c.read_sequence IS NOT NULL
        AND c.sequence > c.read_sequence
        ORDER BY c.sequence DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    // 使用 HashMap 存储未读聊天信息，key 为 (chat_type, to_id)
    let mut unread_chats_map: HashMap<(i32, String), (i64, String, Option<String>)> =
        HashMap::new();

    // 处理从 im_chat 表获取的未读消息
    for row in chat_rows {
        let sequence: Option<i64> = row.sequence;
        let read_sequence: Option<i64> = row.read_sequence;
        let chat_type: i32 = row.chat_type;
        let to_id: String = row.to_id;
        let name: Option<String> = row.name;

        if let (Some(seq), Some(read_seq)) = (sequence, read_sequence) {
            let unread_count = (seq - read_seq).max(0);
            if unread_count > 0 {
                let key = (chat_type, to_id.clone());
                unread_chats_map.insert(key, (unread_count, to_id, name));
            }
        }
    }

    // 然后，直接查询 im_single_message 表，找出所有未读的单聊消息
    // 这样可以确保即使没有聊天记录，也能获取离线消息
    // 重要：排除自己发送给自己的消息（from_id != to_id）
    let single_message_rows = sqlx::query!(
        r#"
        SELECT
            m.from_id,
            m.to_id,
            COUNT(*) "unread_count!: i64",
            MAX(m.message_time) as latest_message_time,
            u.name as from_name
        FROM im_single_message m
        LEFT JOIN users u ON m.from_id = u.open_id OR m.from_id = u.name
        WHERE m.to_id = $1
        AND m.from_id != m.to_id
        AND m.read_status = 0
        AND m.del_flag = 1
        GROUP BY m.from_id, m.to_id, u.name
        ORDER BY latest_message_time DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    // 处理从 im_single_message 表获取的未读消息
    for row in single_message_rows {
        let from_id: String = row.from_id;
        let _to_id: String = row.to_id; // to_id 总是等于 owner_id，这里保留用于调试
        let unread_count: i64 = row.unread_count;
        let from_name: Option<String> = Some(row.from_name);

        if unread_count > 0 {
            let key = (1, from_id.clone()); // chat_type = 1 (单聊), to_id = from_id (对方用户ID)
            // 如果已存在，取较大的未读数量
            if let Some((existing_count, _, _)) = unread_chats_map.get(&key) {
                if unread_count > *existing_count {
                    unread_chats_map.insert(key, (unread_count, from_id, from_name));
                }
            } else {
                unread_chats_map.insert(key, (unread_count, from_id, from_name));
            }
        }
    }

    // 查询群聊消息（3人及以上）：从 im_group_message 表查询
    // 注意：2人聊天的消息已经在 im_single_message 表中统计过了
    let group_message_rows = sqlx::query!(
        r#"
        SELECT
            gm.group_id,
            COUNT(*) "unread_count!: i64",
            MAX(gm.message_time) as latest_message_time,
            g.group_name,
            (SELECT COUNT(*) FROM im_group_member gm3 WHERE gm3.group_id = gm.group_id AND gm3.del_flag = 1) as member_count
        FROM im_group_message gm
        INNER JOIN im_group_member gm2 ON gm.group_id = gm2.group_id AND gm2.del_flag = 1
        INNER JOIN im_group g ON gm.group_id = g.group_id AND g.del_flag = 1
        WHERE gm2.member_id = $1
        AND gm.del_flag = 1
        AND (SELECT COUNT(*) FROM im_group_member gm3 WHERE gm3.group_id = gm.group_id AND gm3.del_flag = 1) >= 3
        -- 注意：群消息已读状态现在使用 Redis，这里暂时不检查已读状态
        -- 未读消息统计应该通过 Redis 查询，或者使用 im_chat 表的 read_sequence
        GROUP BY gm.group_id, g.group_name
        ORDER BY latest_message_time DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    // 处理从 im_group_message 表获取的未读消息（仅3人及以上的群聊）
    for row in group_message_rows {
        let group_id: String = row.group_id;
        let unread_count: i64 = row.unread_count;
        let group_name: Option<String> = Some(row.group_name);
        let member_count: Option<i64> = row.member_count;

        // 确保只处理3人及以上的群聊
        if unread_count > 0 && member_count.map(|c| c >= 3).unwrap_or(false) {
            let key = (2, group_id.clone()); // chat_type = 2 (群聊), to_id = group_id
            // 如果已存在，取较大的未读数量
            if let Some((existing_count, _, _)) = unread_chats_map.get(&key) {
                if unread_count > *existing_count {
                    unread_chats_map.insert(key, (unread_count, group_id, group_name));
                }
            } else {
                unread_chats_map.insert(key, (unread_count, group_id, group_name));
            }
        }
    }

    let mut total_unread: i64 = 0;
    let mut single_chat_unread: i64 = 0;
    let mut group_chat_unread: i64 = 0;
    let mut unread_chats: Vec<serde_json::Value> = Vec::new();

    for ((chat_type, to_id), (unread_count, _, name)) in unread_chats_map {
        total_unread += unread_count;

        if chat_type == 1 {
            single_chat_unread += unread_count;
        } else if chat_type == 2 {
            group_chat_unread += unread_count;
        }

        // 生成 chat_id
        let chat_id = if chat_type == 1 {
            // 单聊：使用排序后的用户ID
            let (min_id, max_id) = if owner_id < to_id.as_str() {
                (owner_id, to_id.as_str())
            } else {
                (to_id.as_str(), owner_id)
            };
            format!("single_{}_{}", min_id, max_id)
        } else {
            // 群聊：使用 group_id
            format!("group_{}", to_id)
        };

        unread_chats.push(json!({
            "chat_id": chat_id,
            "chat_type": chat_type,
            "to_id": to_id,
            "name": name.unwrap_or_else(|| to_id.clone()),
            "unread_count": unread_count,
        }));
    }

    // 按最新消息时间排序
    unread_chats.sort_by(|a, b| {
        let a_count = a.get("unread_count").and_then(|v| v.as_i64()).unwrap_or(0);
        let b_count = b.get("unread_count").and_then(|v| v.as_i64()).unwrap_or(0);
        b_count.cmp(&a_count) // 降序排列
    });

    Ok(json!({
        "total_unread": total_unread,
        "single_chat_unread": single_chat_unread,
        "group_chat_unread": group_chat_unread,
        "unread_chats": unread_chats,
    }))
}
