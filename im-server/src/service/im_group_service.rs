use crate::dto::UpdateGroupRequest;
use crate::prelude::*;
use crate::{db, models::ImGroup, models::ImGroupMember};
use time::OffsetDateTime;
use tracing::{error, warn};

pub async fn create_group(group: ImGroup) -> AppResult<()> {
    let conn = db::pool();
    group.validate().map_err(|e| {
        warn!("Failed to validate group request: {}", e);
        AppError::public(e)
    })?;

    let now = OffsetDateTime::now_utc();

    let result = sqlx::query!(
        r#"
        INSERT INTO im_group
         (group_id, owner_id, group_type, group_name, mute, apply_join_type, avatar,
          max_member_count, introduction, notification, status, sequence, create_time,
          update_time, extra, version, del_flag, verifier)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, 0, $11, $12, $13, 1, 1, $14)
         "#,
        &group.group_id,
        &group.owner_id,
        group.group_type,
        &group.group_name,
        group.mute,
        group.apply_join_type,
        group.avatar,
        group.max_member_count,
        group.introduction,
        group.notification,
        now,
        now,
        group.extra,
        group.verifier
    )
    .execute(conn)
    .await;

    match result {
        Ok(_) => {
            // 添加群主为成员
            if let Err(e) = add_group_member(&group.group_id, &group.owner_id, 2, None).await {
                error!("添加群主为成员失败: {:?}", e);
                return Err(e);
            }
            Ok(())
        }
        Err(e) => {
            error!("创建群组数据库错误: {:?}", e);
            // 检查是否是重复键错误
            // if let sqlx::Error::Database(db_err) = &e {
            //     if db_err.message().contains("Duplicate entry") || db_err.message().contains("PRIMARY") {
            //         warn!("群组ID已存在: {}", group.group_id);
            //         return Err(ErrorCode::InvalidInput);
            //     }
            // }
            Err(AppError::internal(""))
        }
    }
}

/// 获取群组信息
pub async fn get_group(group_id: &str) -> AppResult<ImGroup> {
    let conn = db::pool();
    let group = sqlx::query_as!(
        ImGroup,
        r#"
        SELECT group_id, owner_id, group_type, group_name, mute, apply_join_type, avatar,
                max_member_count, introduction, notification, status, sequence, create_time,
                update_time, extra, version, del_flag, verifier,
                (SELECT COUNT(*) FROM im_group_member gm WHERE gm.group_id = im_group.group_id AND gm.del_flag = 1) as member_count
         FROM im_group
         WHERE group_id = $1 AND del_flag = 1
         "#,
         group_id
    )
    .fetch_optional(conn)
    .await?;

    match group {
        Some(g) => Ok(g),
        None => Err(AppError::not_found(group_id)),
    }
}

pub async fn add_group_member(
    group_id: &str,
    member_id: &str,
    role: i32,
    alias: Option<String>,
) -> AppResult<()> {
    let conn = db::pool();
    let group_member_id = format!("{}_{}", group_id, member_id);

    if group_member_id.len() > 100 {
        warn!(
            "群成员ID长度超过限制: {} > 100, group_id={}, member_id={}",
            group_member_id.len(),
            group_id,
            member_id
        );
        return Err(AppError::internal("群成员ID长度超过限制"));
    }
    let now = OffsetDateTime::now_utc();
    let result = sqlx::query!(
        r#"
        INSERT INTO im_group_member
        (group_member_id, group_id, member_id, role, mute, alias, join_time, del_flag,
         create_time, update_time, version)
         VALUES ($1, $2, $3, $4, 1, $5, $6, 1, $7, $8, 1)
         ON CONFLICT (group_member_id) DO UPDATE SET
         role = EXCLUDED.role,
         alias = EXCLUDED.alias,
         del_flag = 1,
         update_time = $9,
         version = im_group_member.version + 1
        "#,
        group_member_id,
        group_id,
        member_id,
        role,
        alias,
        now,
        now,
        now,
        now,
    )
    .execute(conn)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("添加群成员数据库错误: {:?}", e);
            Err(AppError::internal(format!("添加群成员数据库错误: {:?}", e)))
        }
    }
}

/// 获取群成员列表
pub async fn get_group_members(group_id: &str) -> AppResult<Vec<ImGroupMember>> {
    let conn = db::pool();
    // 获取所有群成员记录（可能包含重复的 member_id）
    // 在应用层进行去重，确保每个 member_id 只返回一条记录
    let all_members = sqlx::query_as!(
        ImGroupMember,
        r#"
        SELECT group_member_id, group_id, member_id, role, speak_date, mute, alias,
                join_time, leave_time, join_type, extra, del_flag, create_time, update_time, version
         FROM im_group_member
         WHERE group_id = $1 AND del_flag = 1
         ORDER BY role DESC, update_time DESC, join_time ASC
         "#,
        group_id
    )
    .fetch_all(conn)
    .await?;

    // 使用 HashMap 去重，保留每个 member_id 的第一条记录（已按 update_time DESC 排序，所以是最新的）
    use std::collections::HashMap;
    let mut unique_members = HashMap::new();
    for member in all_members {
        unique_members
            .entry(member.member_id.clone())
            .or_insert(member);
    }

    // 转换为 Vec 并重新排序
    let mut members: Vec<ImGroupMember> = unique_members.into_values().collect();
    members.sort_by(|a, b| {
        // 先按角色排序（角色高的在前），然后按加入时间排序
        match b.role.cmp(&a.role) {
            std::cmp::Ordering::Equal => a
                .join_time
                .unwrap_or(OffsetDateTime::UNIX_EPOCH)
                .cmp(&b.join_time.unwrap_or(OffsetDateTime::UNIX_EPOCH)),
            other => other,
        }
    });

    Ok(members)
}

/// 移除群成员（只有群主和管理员可以移除成员）
pub async fn remove_group_member(
    group_id: &str,
    member_id: &str,
    operator_id: &str,
) -> AppResult<()> {
    let now = OffsetDateTime::now_utc();

    // 验证群组是否存在
    let group = match get_group(group_id).await {
        Ok(g) => g,
        Err(e) => {
            warn!("群组不存在: {}", group_id);
            return Err(e);
        }
    };

    // 验证操作者权限：只有群主或管理员可以移除成员
    let members = match get_group_members(group_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!("获取群成员列表失败: group_id={}, error={:?}", group_id, e);
            return Err(e);
        }
    };

    // 查找操作者的成员信息
    let operator_member = members.iter().find(|m| m.member_id == operator_id);
    let is_owner = group.owner_id.trim() == operator_id.trim();
    let is_admin = operator_member.map(|m| m.role == 1).unwrap_or(false);

    if !is_owner && !is_admin {
        warn!(
            "用户 {} 不是群主或管理员，无法移除成员: group_id={}",
            operator_id, group_id
        );
        return Err(AppError::internal(f!(
            "用户 {} 不是群主或管理员，无法移除成员: group_id={}",
            operator_id,
            group_id
        )));
    }

    // 不能移除群主
    if member_id.trim() == group.owner_id.trim() {
        warn!(
            "不能移除群主: group_id={}, member_id={}",
            group_id, member_id
        );
        return Err(AppError::internal(f!(
            "不能移除群主: group_id={}, member_id={}",
            group_id,
            member_id
        )));
    }

    // 查找要移除的成员
    let target_member = members.iter().find(|m| m.member_id == member_id);
    if target_member.is_none() {
        warn!(
            "要移除的成员不存在: group_id={}, member_id={}",
            group_id, member_id
        );
        return Err(AppError::not_found(f!(
            "要移除的成员不存在: group_id={}, member_id={}",
            group_id,
            member_id
        )));
    }

    // 管理员不能移除其他管理员（只有群主可以）
    if !is_owner {
        if let Some(target) = target_member {
            if target.role == 1 {
                warn!(
                    "管理员不能移除其他管理员: group_id={}, operator_id={}, member_id={}",
                    group_id, operator_id, member_id
                );
                return Err(AppError::internal(f!(
                    "管理员不能移除其他管理员: group_id={}, operator_id={}, member_id={}",
                    group_id,
                    operator_id,
                    member_id
                )));
            }
        }
    }

    let conn = db::pool();

    // 执行删除
    sqlx::query!(
        r#"
        UPDATE im_group_member
         SET del_flag = 0, leave_time = $1, update_time = $1, version = version + 1
         WHERE group_id = $2 AND member_id = $3 AND del_flag = 1
         "#,
        now,
        group_id,
        member_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 更新群成员角色（设置/取消管理员）
pub async fn update_member_role(
    group_id: &str,
    member_id: &str,
    role: i32,
    operator_id: &str,
) -> AppResult<()> {
    let now = OffsetDateTime::now_utc();

    // 验证操作者权限（只有群主可以设置/取消管理员）
    let group = match get_group(group_id).await {
        Ok(g) => g,
        Err(e) => {
            warn!("群组不存在: {}", group_id);
            return Err(e);
        }
    };

    if group.owner_id != operator_id {
        warn!(
            "用户 {} 不是群组 {} 的群主，无法修改成员角色",
            operator_id, group_id
        );
        return Err(AppError::public(f!(
            "用户 {} 不是群组 {} 的群主，无法修改成员角色",
            operator_id,
            group_id
        )));
    }

    // 验证角色值（0=普通成员，1=管理员，2=群主）
    if role < 0 || role > 2 {
        warn!("无效的角色值: {}", role);
        return Err(AppError::public(f!("无效的角色值: {}", role)));
    }

    // 不能修改群主的角色
    if member_id == group.owner_id && role != 2 {
        warn!("不能修改群主的角色");
        return Err(AppError::public("不能修改群主的角色"));
    }

    let conn = db::pool();
    // 更新成员角色
    sqlx::query!(
        r#"UPDATE im_group_member
         SET role = $1, update_time = $2, version = version + 1
         WHERE group_id = $3 AND member_id = $4 AND del_flag = 1"#,
        role,
        now,
        group_id,
        member_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 删除群组（硬删除，只有群主可以删除）
pub async fn delete_group(group_id: &str, owner_id: &str) -> AppResult<()> {
    // 首先验证是否是群主
    let group = match get_group(group_id).await {
        Ok(g) => g,
        Err(e) => {
            warn!("群组不存在: {}", group_id);
            return Err(e);
        }
    };

    // 去除空格进行比较（更宽松的匹配）
    let group_owner_id_trimmed = group.owner_id.trim();
    let owner_id_trimmed = owner_id.trim();

    if group_owner_id_trimmed != owner_id_trimmed {
        warn!(
            "用户不是群主，无法删除群组: group_id={}, group_owner_id='{}', current_owner_id='{}'",
            group_id, group_owner_id_trimmed, owner_id_trimmed
        );
        return Err(AppError::public("用户不是群主，无法删除群组"));
    }

    let conn = db::pool();
    // 先删除所有群成员（硬删除）
    sqlx::query!(
        r#"
        DELETE FROM im_group_member WHERE group_id = $1
        "#,
        group_id,
    )
    .execute(conn)
    .await?;

    // 删除群组（硬删除）
    sqlx::query!(
        r#"
        DELETE FROM im_group WHERE group_id = $1
        "#,
        group_id,
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 解散群组（只有群主可以解散）
/// 返回解散前的成员列表，用于发送系统消息
pub async fn dissolve_group(group_id: &str, owner_id: &str) -> AppResult<Vec<ImGroupMember>> {
    let now = OffsetDateTime::now_utc();
    let conn = db::pool();
    // 首先检查群组是否存在（无论 del_flag 状态）
    let group = sqlx::query_as!(
        ImGroup,
        r#"
        SELECT group_id, owner_id, group_type, group_name, mute, apply_join_type, avatar,
                max_member_count, introduction, notification, status, sequence, create_time,
                update_time, extra, version, del_flag, verifier,
                (SELECT COUNT(*) FROM im_group_member gm WHERE gm.group_id = im_group.group_id AND gm.del_flag = 1) as member_count
         FROM im_group
         WHERE group_id = $1
         "#,
        group_id
    )
    .fetch_optional(conn)
    .await?;

    let group = match group {
        Some(g) => g,
        None => {
            warn!("群组不存在: {}", group_id);
            return Err(AppError::not_found(group_id));
        }
    };

    // 如果群组已经解散（del_flag = 0），直接返回成功（幂等操作）
    if group.del_flag == 0 {
        info!("群组已经解散，无需重复操作: group_id={}", group_id);
        return Ok(vec![]);
    }

    // 验证是否是群主
    let group_owner_id_trimmed = group.owner_id.trim();
    let owner_id_trimmed = owner_id.trim();

    if group_owner_id_trimmed != owner_id_trimmed {
        warn!(
            "用户不是群主，无法解散群组: group_id={}, group_owner_id='{}', current_owner_id='{}', group_owner_id_len={}, owner_id_len={}",
            group_id,
            group_owner_id_trimmed,
            owner_id_trimmed,
            group_owner_id_trimmed.len(),
            owner_id_trimmed.len()
        );
        return Err(AppError::public("用户不是群主，无法解散群组"));
    }

    // 在解散前获取所有成员列表（用于发送系统消息）
    let members = match get_group_members(group_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!("获取群成员列表失败: group_id={}, error={:?}", group_id, e);
            vec![] // 即使获取成员失败，也继续解散群组
        }
    };

    // 软删除群组（设置 del_flag = 0）
    sqlx::query!(
        r#"UPDATE im_group
         SET del_flag = 0, update_time = $1, version = version + 1
         WHERE group_id = $2"#,
        now,
        group_id
    )
    .execute(conn)
    .await?;

    // 同时软删除所有群成员（设置 del_flag = 0）
    sqlx::query!(
        r#"UPDATE im_group_member
         SET del_flag = 0, leave_time = $1, update_time = $2, version = version + 1
         WHERE group_id = $3"#,
        now,
        now,
        group_id
    )
    .execute(conn)
    .await?;

    // 返回解散前的成员列表
    Ok(members)
}

/// 更新群成员别名（我在本群的昵称）
pub async fn update_member_alias(
    group_id: &str,
    member_id: &str,
    alias: Option<String>,
) -> AppResult<()> {
    let now = OffsetDateTime::now_utc();

    // 验证成员是否存在
    let member = match get_group_members(group_id).await {
        Ok(members) => members.iter().find(|m| m.member_id == member_id).cloned(),
        Err(e) => {
            warn!("获取群成员列表失败: group_id={}, error={:?}", group_id, e);
            return Err(e);
        }
    };

    if member.is_none() {
        warn!(
            "群成员不存在: group_id={}, member_id={}",
            group_id, member_id
        );
        return Err(AppError::public("群成员不存在"));
    }

    let conn = db::pool();
    // 更新成员别名
    sqlx::query!(
        r#"UPDATE im_group_member
         SET alias = $1, update_time = $2, version = version + 1
         WHERE group_id = $3 AND member_id = $4 AND del_flag = 1"#,
        alias,
        now,
        group_id,
        member_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// 更新群组信息（只有群主可以更新）
pub async fn update_group(
    group_id: &str,
    owner_id: &str,
    req: &UpdateGroupRequest,
) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    // 验证是否是群主
    let group = match get_group(group_id).await {
        Ok(g) => g,
        Err(e) => {
            warn!("群组不存在: {}", group_id);
            return Err(e);
        }
    };

    // 去除空格进行比较（更宽松的匹配）
    let group_owner_id_trimmed = group.owner_id.trim();
    let owner_id_trimmed = owner_id.trim();

    if group_owner_id_trimmed != owner_id_trimmed {
        warn!(
            "用户不是群主，无法更新群组信息: group_id={}, group_owner_id='{}', current_owner_id='{}'",
            group_id, group_owner_id_trimmed, owner_id_trimmed
        );
        return Err(AppError::public("用户不是群主，无法更新群组信息"));
    }

    // 使用 QueryBuilder 构建动态SQL，完全手动控制逗号
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE im_group SET ");
    let mut has_update = false;
    let mut need_comma = false;

    if let Some(ref group_name) = req.group_name {
        if group_name.trim().is_empty() {
            warn!("群组名称不能为空");
            return Err(AppError::public(f!("群组名称不能为空")));
        }
        if group_name.len() > 100 {
            warn!("群组名称长度超过限制: {} > 100", group_name.len());
            return Err(AppError::public(f!(
                "群组名称长度超过限制: {} > 100",
                group_name.len()
            )));
        }
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("group_name = ");
        query_builder.push_bind(group_name.trim().to_string());
        need_comma = true;
        has_update = true;
    }
    if let Some(ref introduction) = req.introduction {
        if introduction.len() > 100 {
            warn!("群组简介长度超过限制: {} > 100", introduction.len());
            return Err(AppError::public(f!(
                "群组简介长度超过限制: {} > 100",
                introduction.len()
            )));
        }
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("introduction = ");
        query_builder.push_bind(introduction.trim().to_string());
        need_comma = true;
        has_update = true;
    }
    if let Some(ref avatar) = req.avatar {
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("avatar = ");
        query_builder.push_bind(avatar.trim().to_string());
        need_comma = true;
        has_update = true;
    }
    if let Some(ref notification) = req.notification {
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("notification = ");
        query_builder.push_bind(notification.trim().to_string());
        need_comma = true;
        has_update = true;
    }
    if let Some(apply_join_type) = req.apply_join_type {
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("apply_join_type = ");
        query_builder.push_bind(apply_join_type);
        need_comma = true;
        has_update = true;
    }
    if let Some(max_member_count) = req.max_member_count {
        if max_member_count < 1 {
            warn!("最大成员数必须大于0");
            return Err(AppError::public(f!("最大成员数必须大于0")));
        }
        if need_comma {
            query_builder.push(", ");
        }
        query_builder.push("max_member_count = ");
        query_builder.push_bind(max_member_count);
        need_comma = true;
        has_update = true;
    }

    if !has_update {
        warn!("没有需要更新的字段");
        return Err(AppError::public(f!("没有需要更新的字段")));
    }

    // 添加 update_time 和 version（这些总是需要更新的）
    if need_comma {
        query_builder.push(", ");
    }
    query_builder.push("update_time = ");
    query_builder.push_bind(now);
    query_builder.push(", version = version + 1");

    query_builder.push(" WHERE group_id = ");
    query_builder.push_bind(group_id);
    query_builder.push(" AND del_flag = 1");

    query_builder.build().execute(conn).await?;

    Ok(())
}

/// 获取用户所在的群组列表
/// 注意：只有3人及以上的群组才会在 im_group 表中有记录
/// 2人聊天不会创建群组记录，所以这里只返回真正的群组（3人及以上）
pub async fn get_user_groups(user_id: &str) -> AppResult<Vec<ImGroup>> {
    let conn = db::pool();

    let groups = sqlx::query_as!(
        ImGroup,
        r#"SELECT
            g.group_id,
            g.owner_id,
            g.group_type,
            g.group_name,
            g.mute,
            g.apply_join_type,
            g.avatar,
            g.max_member_count,
            g.introduction,
            g.notification,
            g.status,
            g.sequence,
            g.create_time,
            g.update_time,
            g.extra,
            g.version,
            g.del_flag,
            g.verifier,
            -- 计算群组成员数量
            (
                SELECT COUNT(*)
                FROM im_group_member gm2
                WHERE gm2.group_id = g.group_id
                AND gm2.del_flag = 1
            ) as "member_count!"
        FROM im_group g
        -- 先检查用户是否是该群的成员
        WHERE EXISTS (
            SELECT 1
            FROM im_group_member gm
            WHERE gm.group_id = g.group_id
            AND gm.member_id = $1
            AND gm.del_flag = 1
        )
        -- 群组本身必须有效
        AND g.del_flag = 1
        -- 确保是真正的群组（3人及以上）
        AND (
            SELECT COUNT(*)
            FROM im_group_member gm3
            WHERE gm3.group_id = g.group_id
            AND gm3.del_flag = 1
        ) >= 3
        ORDER BY g.update_time DESC"#,
        user_id
    )
    .fetch_all(conn)
    .await?;

    Ok(groups)
}
