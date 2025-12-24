use crate::{db, prelude::*};

use crate::models::{ImFriendship, ImFriendshipRequest};

use time::OffsetDateTime;

pub async fn is_friend(owner_id: &str, to_id: &str) -> AppResult<bool> {
    let conn = db::pool();
    debug!("检查好友关系: owner_id={}, to_id={}", owner_id, to_id);

    // 先尝试直接匹配（检查双向关系）
    let result = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*) "count!: i64" FROM im_friendship
            WHERE ((owner_id = $1 AND to_id = $2) OR (owner_id = $2 AND to_id = $1))
            AND (del_flag IS NULL OR del_flag = 1)
            AND (black IS NULL OR black = 1)
        "#,
        owner_id,
        to_id
    )
    .fetch_one(conn)
    .await
    .inspect_err(|e| {
        warn!(
            "检查好友关系失败: owner_id={}, to_id={}, error={:?}",
            owner_id, to_id, e
        );
    })?;

    let is_friend = result > 0;
    debug!(
        "好友关系检查结果（直接匹配）: owner_id={}, to_id={}, is_friend={}",
        owner_id, to_id, is_friend
    );

    // 如果直接匹配失败，可能是ID格式不一致（比如一个是open_id，一个是用户名）
    // 尝试通过用户表查找可能的匹配，然后使用所有可能的标识组合查询
    if !is_friend {
        // 使用一个查询同时查找两个用户的所有标识
        let user_row = sqlx::query!(
            r#"SELECT
                u1.open_id as owner_open_id, u1.name as owner_name, u1.phone as owner_phone,
                u2.open_id as to_open_id, u2.name as to_name, u2.phone as to_phone
             FROM users u1, users u2
             WHERE (u1.open_id = $1 OR u1.name = $1 OR u1.phone = $1)
             AND (u2.open_id = $2 OR u2.name = $2 OR u2.phone = $2)
             AND (u1.status IS NULL OR u1.status = 1)
             AND (u2.status IS NULL OR u2.status = 1)
             LIMIT 1"#,
            owner_id,
            to_id
        )
        .fetch_optional(conn)
        .await
        .ok()
        .flatten();

        if let Some(row) = user_row {
            // 如果找到了匹配的用户，尝试再次检查好友关系
            let owner_open_id = row.owner_open_id;
            let to_open_id = row.to_open_id;
            let owner_name = row.owner_name;
            let to_name = row.to_name;
            let owner_phone = row.owner_phone;
            let to_phone = row.to_phone;

            let mut owner_ids: Vec<String> = Vec::new();
            if owner_open_id != owner_id {
                owner_ids.push(owner_open_id);
            }
            if owner_name != owner_id && !owner_ids.contains(&owner_name) {
                owner_ids.push(owner_name.clone());
            }

            if let Some(ref id) = owner_phone {
                if id != owner_id && !owner_ids.contains(id) {
                    owner_ids.push(id.clone());
                }
            }

            let mut to_ids: Vec<String> = Vec::new();
            if to_open_id != owner_id {
                to_ids.push(to_open_id);
            }
            if to_name != owner_id && !to_ids.contains(&to_name) {
                to_ids.push(to_name.clone());
            }

            if let Some(ref id) = to_phone {
                if id != owner_id && !to_ids.contains(id) {
                    to_ids.push(id.clone());
                }
            }

            // 尝试所有可能的组合（包括原始ID）
            let all_owner_ids: Vec<&str> = {
                let mut ids = vec![owner_id];
                ids.extend(owner_ids.iter().map(|s| s.as_str()));
                ids
            };

            let all_to_ids: Vec<&str> = {
                let mut ids = vec![to_id];
                ids.extend(to_ids.iter().map(|s| s.as_str()));
                ids
            };

            // 尝试所有可能的组合
            for oid in &all_owner_ids {
                for tid in &all_to_ids {
                    if *oid == owner_id && *tid == to_id {
                        continue; // 已经检查过了
                    }

                    let check_result = sqlx::query_scalar!(
                        r#"SELECT COUNT(*) "count!: i64" FROM im_friendship
                         WHERE ((owner_id = $1 AND to_id = $2) OR (owner_id = $2 AND to_id = $1))
                         AND (del_flag IS NULL OR del_flag = 1)
                         AND (black IS NULL OR black = 1)"#,
                        oid,
                        tid
                    )
                    .fetch_optional(conn)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(0);

                    if check_result > 0 {
                        debug!(
                            "好友关系检查结果（通过ID转换匹配）: owner_id={}, to_id={}, matched_owner={}, matched_to={}, is_friend=true",
                            owner_id, to_id, oid, tid
                        );
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(is_friend)
}

/// 获取好友列表
/// 支持通过用户名、手机号或 open_id 查询
pub async fn get_friends(owner_id: &str) -> AppResult<Vec<ImFriendship>> {
    let conn = db::pool();
    debug!("查询好友列表: owner_id={}", owner_id);

    // 首先尝试直接匹配
    let friends = sqlx::query_as!(
        ImFriendship,
        r#"
            SELECT owner_id, to_id, remark, del_flag, black, create_time, update_time,
            sequence, black_sequence, add_source, extra, version
            FROM im_friendship
            WHERE owner_id = $1
            AND (del_flag IS NULL OR del_flag = 1)
            AND (black IS NULL OR black = 1)
            ORDER BY sequence DESC, create_time DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await?;

    if !friends.is_empty() {
        info!(
            "查询好友列表结果（直接匹配）: owner_id={}, count={}",
            owner_id,
            friends.len()
        );
        return Ok(friends);
    }

    // 如果直接匹配没有结果，尝试通过用户表查找可能的 owner_id 格式
    // 先尝试作为用户名查找
    let user_by_name = sqlx::query_scalar!(
        r#"
            SELECT open_id FROM users WHERE name = $1
            AND (status IS NULL OR status = 1)
            LIMIT 1
        "#,
        owner_id
    )
    .fetch_optional(conn)
    .await
    .ok()
    .flatten();

    if let Some(open_id) = user_by_name {
        if open_id != owner_id {
            debug!("尝试使用 open_id 查询好友列表: open_id={}", open_id);

            let friends = sqlx::query_as!(
                ImFriendship,
                r#"
                    SELECT owner_id, to_id, remark, del_flag, black, create_time, update_time,
                    sequence, black_sequence, add_source, extra, version
                    FROM im_friendship
                    WHERE owner_id = $1
                    AND (del_flag IS NULL OR del_flag = 1)
                    AND (black IS NULL OR black = 1)
                    ORDER BY sequence DESC, create_time DESC
                "#,
                open_id
            )
            .fetch_all(conn)
            .await
            .ok();

            if let Some(friends) = friends {
                if !friends.is_empty() {
                    warn!(
                        "使用 open_id 查询到好友列表: owner_id={}, open_id={}, count={}",
                        owner_id,
                        open_id,
                        friends.len()
                    );
                    return Ok(friends);
                }
            }
        }
    }

    // 尝试作为 open_id 查找对应的用户名
    let user_by_open_id = sqlx::query_scalar!(
        r#"
            SELECT name FROM users WHERE open_id = $1
            AND (status IS NULL OR status = 1) LIMIT 1
        "#,
        owner_id
    )
    .fetch_optional(conn)
    .await
    .ok()
    .flatten();

    if let Some(name) = user_by_open_id {
        if name != owner_id && !name.is_empty() {
            debug!("尝试使用用户名查询好友列表: name={}", name);

            let friends = sqlx::query_as!(
                ImFriendship,
                r#"
                    SELECT owner_id, to_id, remark, del_flag, black, create_time, update_time,
                    sequence, black_sequence, add_source, extra, version
                    FROM im_friendship
                    WHERE owner_id = $1
                    AND (del_flag IS NULL OR del_flag = 1)
                    AND (black IS NULL OR black = 1)
                    ORDER BY sequence DESC, create_time DESC
                "#,
                name
            )
            .fetch_all(conn)
            .await
            .ok();

            if let Some(friends) = friends {
                if !friends.is_empty() {
                    warn!(
                        "使用用户名查询到好友列表: owner_id={}, name={}, count={}",
                        owner_id,
                        name,
                        friends.len()
                    );
                    return Ok(friends);
                }
            }
        }
    }

    info!("查询好友列表结果: owner_id={}, count=0", owner_id);
    Ok(friends)
}

/// 添加好友（双向关系）
pub async fn add_friend(
    owner_id: String,
    to_id: String,
    add_source: Option<String>,
    remark: Option<String>,
) -> AppResult<()> {
    let conn = db::pool();
    if owner_id == to_id {
        return Err(AppError::public("不能添加自己为好友"));
    }

    // 检查是否已经是双向好友（两个方向都检查）
    let is_owner_to_friend = is_friend(&owner_id, &to_id).await?;
    let is_friend_to_owner = is_friend(&to_id, &owner_id).await?;

    // 如果两个方向都是好友，说明已经是双向好友关系
    if is_owner_to_friend && is_friend_to_owner {
        return Err(AppError::public("已经是好友"));
    }

    let timestamp = OffsetDateTime::now_utc();

    let mut tx = conn.begin().await?;

    sqlx::query!(
        r#"
            INSERT INTO im_friendship
            (owner_id, to_id, remark, del_flag, black, create_time, update_time, sequence, add_source, version)
            VALUES ($1, $2, $3, 1, 1, $4, $4, $5, $6, 1)
            ON CONFLICT (owner_id, to_id) DO UPDATE SET
            del_flag = 1,
            black = 1,
            remark = EXCLUDED.remark,
            update_time = $4,
            version = im_friendship.version + 1
         "#,
         owner_id,
         to_id,
         remark,
         timestamp,
         timestamp.unix_timestamp() * 1000,
         add_source,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
            INSERT INTO im_friendship
            (owner_id, to_id, remark, del_flag, black, create_time, update_time, sequence, add_source, version)
            VALUES ($1, $2, $3, 1, 1, $4, $4, $5, $6, 1)
            ON CONFLICT (owner_id, to_id) DO UPDATE SET
            del_flag = 1,
            black = 1,
            remark = EXCLUDED.remark,
            update_time = $4,
            version = im_friendship.version + 1
        "#,
        to_id,
        owner_id,
        remark,
        timestamp,
        timestamp.unix_timestamp() * 1000,
        add_source,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// 删除好友
pub async fn remove_friend(owner_id: &str, to_id: &str) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    let existing_count = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*) as "count!: i64" FROM im_friendship
            WHERE ((owner_id = $1 AND to_id = $2) OR (owner_id = $2 AND to_id = $1))
            AND (del_flag IS NULL OR del_flag = 1)
         "#,
        owner_id,
        to_id
    )
    .fetch_one(conn)
    .await?;

    if existing_count == 0 {
        warn!("好友关系不存在: owner_id={}, to_id={}", owner_id, to_id);
        return Err(AppError::not_found("好友不存在"));
    }

    // 软删除双向关系
    let result = sqlx::query!(
        r#"
            UPDATE im_friendship
            SET del_flag = 0, update_time = $1, version = version + 1
            WHERE ((owner_id = $2 AND to_id = $3) OR (owner_id = $3 AND to_id = $2))
            AND (del_flag IS NULL OR del_flag = 1)
         "#,
        now,
        owner_id,
        to_id
    )
    .execute(conn)
    .await
    .inspect_err(|e| {
        warn!(
            "删除好友关系失败: owner_id={}, to_id={}, error={:?}",
            owner_id, to_id, e
        );
    })?;

    if result.rows_affected() == 0 {
        warn!(
            "删除好友关系时没有更新任何记录: owner_id={}, to_id={}",
            owner_id, to_id
        );
        return Err(AppError::not_found("好友不存在"));
    }
    info!(
        "成功删除好友关系: owner_id={}, to_id={}, rows_affected={}",
        owner_id,
        to_id,
        result.rows_affected()
    );
    Ok(())
}

pub async fn update_remark(owner_id: &str, to_id: &str, remark: Option<String>) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();
    sqlx::query!(
        r#"
            UPDATE im_friendship
            SET remark = $1, update_time = $2, version = version + 1
            WHERE owner_id = $3 AND to_id = $4
            AND (del_flag IS NULL OR del_flag = 1)
        "#,
        remark,
        now,
        owner_id,
        to_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// 拉黑/取消拉黑好友（切换状态）
pub async fn black_friend(owner_id: &str, to_id: &str) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    // 先查询当前状态
    let current_black = sqlx::query_scalar!(
        r#"
            SELECT black FROM im_friendship WHERE owner_id = $1 AND to_id = $2
        "#,
        owner_id,
        to_id
    )
    .fetch_optional(conn)
    .await?
    .flatten();

    let new_black = if current_black == Some(2) {
        // 如果已经拉黑，则取消拉黑
        1
    } else {
        // 否则拉黑
        2
    };

    sqlx::query!(
        r#"
            UPDATE im_friendship
            SET black = $1, black_sequence = $2, update_time = $3, version = version + 1
            WHERE owner_id = $4 AND to_id = $5
         "#,
        new_black,
        if new_black == 2 {
            Some(OffsetDateTime::now_utc().unix_timestamp() * 1000)
        } else {
            None
        },
        now,
        owner_id,
        to_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 创建好友请求
pub async fn create_friendship_request(request: ImFriendshipRequest) -> AppResult<()> {
    let conn = db::pool();

    request.validate().map_err(|e| {
        warn!("Failed to validate friendship request: {}", e);
        AppError::public(e)
    })?;

    if is_friend(&request.from_id, &request.to_id).await? {
        return Err(AppError::public("不能重复添加好友"));
    }

    // 检查是否已经有待处理的好友请求（只检查待处理的，已拒绝的可以重新发送）
    let existing_requests = get_friendship_requests(&request.to_id, Some(0)).await?;
    if existing_requests
        .iter()
        .any(|r| r.from_id == request.from_id && r.approve_status == Some(0))
    {
        warn!(
            "已经存在待处理的好友请求: from_id={}, to_id={}",
            request.from_id, request.to_id
        );
        return Err(AppError::public("已存在待处理的好友请求"));
    }

    // 如果之前有被拒绝的请求，先删除它，然后创建新请求
    // 这样可以避免重复键冲突，同时允许重新发送被拒绝的请求
    sqlx::query!(
        r#"
            DELETE FROM im_friendship_request
            WHERE from_id = $1 AND to_id = $2 AND approve_status = 2
        "#,
        request.from_id,
        request.to_id
    )
    .execute(conn)
    .await
    .ok(); // 忽略删除错误（可能没有旧记录）

    let timestamp = OffsetDateTime::now_utc();

    let result = sqlx::query!(
        r#"
            INSERT INTO im_friendship_request
            (id, from_id, to_id, remark, read_status, add_source, message, approve_status,
            create_time, update_time, sequence, del_flag, version)
            VALUES ($1, $2, $3, $4, 0, $5, $6, 0, $7, $8, $9, 1, 1)
            ON CONFLICT (id) DO UPDATE SET
            approve_status = 0,
            update_time = $8,
            version = im_friendship_request.version + 1
        "#,
        request.id,
        request.from_id,
        request.to_id,
        request.remark,
        request.add_source,
        request.message,
        timestamp,
        timestamp,
        timestamp.unix_timestamp() * 1000,
    )
    .execute(conn)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            error!(
                "创建好友请求数据库错误: request_id={}, from_id={}, to_id={}, error={:?}",
                request.id, request.from_id, request.to_id, e
            );
            // 检查是否是外键约束错误
            if let sqlx::Error::Database(db_err) = &e {
                let error_msg = db_err.message();
                if error_msg.contains("foreign key constraint") || error_msg.contains("FOREIGN KEY")
                {
                    warn!(
                        "外键约束错误: 发送者 {} 或接收者 {} 可能不存在",
                        request.from_id, request.to_id
                    );
                }
                if error_msg.contains("Duplicate entry") || error_msg.contains("PRIMARY") {
                    warn!(
                        "好友请求记录已存在: request_id={}, from_id={}, to_id={}",
                        request.id, request.from_id, request.to_id
                    );
                    // 对于重复键，ON DUPLICATE KEY UPDATE 应该已经处理，但如果还是失败，返回错误
                }
                if error_msg.contains("Data too long") || error_msg.contains("too long") {
                    warn!(
                        "数据长度超过限制: request_id={}, from_id={}, to_id={}",
                        request.id, request.from_id, request.to_id
                    );
                }
            }

            Err(AppError::from(e))
        }
    }
}

/// 获取好友请求列表
pub async fn get_friendship_requests(
    to_id: &str,
    approve_status: Option<i32>,
) -> AppResult<Vec<ImFriendshipRequest>> {
    let conn = db::pool();

    let requests = if let Some(status) = approve_status {
        sqlx::query_as!(
            ImFriendshipRequest,
            r#"
                SELECT id, from_id, to_id, remark, read_status, add_source, message,
                    approve_status, create_time, update_time, sequence, del_flag, version
                FROM im_friendship_request
                WHERE to_id = $1
                AND (del_flag IS NULL OR del_flag = 1)
                AND approve_status = $2
                ORDER BY create_time DESC
            "#,
            to_id,
            status
        )
        .fetch_all(conn)
        .await?
    } else {
        sqlx::query_as!(
            ImFriendshipRequest,
            r#"
                SELECT id, from_id, to_id, remark, read_status, add_source, message,
                    approve_status, create_time, update_time, sequence, del_flag, version
                FROM im_friendship_request
                WHERE to_id = $1
                AND (del_flag IS NULL OR del_flag = 1)
                ORDER BY create_time DESC
            "#,
            to_id
        )
        .fetch_all(conn)
        .await?
    };

    Ok(requests)
}

/// 处理好友请求（同意或拒绝）
pub async fn handle_friendship_request(request_id: &str, approve_status: i32) -> AppResult<()> {
    let conn = db::pool();
    let timestamp = OffsetDateTime::now_utc();
    let request = sqlx::query_as!(
        ImFriendshipRequest,
        r#"
            UPDATE im_friendship_request
            SET approve_status = $1, update_time = $2, version = version + 1
            WHERE id = $3
            RETURNING *
         "#,
        approve_status,
        timestamp,
        request_id
    )
    .fetch_optional(conn)
    .await?;

    if approve_status == 1 {
        if let Some(req) = request {
            add_friend(req.from_id, req.to_id, req.add_source, req.remark).await?;
        }
    }

    Ok(())
}
