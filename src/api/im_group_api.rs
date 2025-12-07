use crate::db;
use crate::dto::AddGroupMemberRequest;
use crate::dto::CreateGroupRequest;
use crate::dto::UpdateGroupRequest;
use crate::dto::UpdateMemberAliasRequest;
use crate::dto::UpdateMemberRoleRequest;
use crate::models::ChatMessage;
use crate::models::ImGroup;
use crate::models::ImGroupMember;
use crate::models::ImGroupMessage;
use crate::models::User;
use crate::prelude::*;
use crate::service::im_friendship_service;
use crate::service::im_group_service;
use crate::service::im_message_service;
use crate::service::user_service;
use crate::utils::subcription::SubscriptionService;
use salvo::oapi::endpoint;
use salvo::oapi::extract::{JsonBody, PathParam};
use salvo::prelude::*;
use std::sync::Arc;
use time::OffsetDateTime;
use ulid::Ulid;

/// 创建群组
#[endpoint(tags("im_group"))]
pub async fn create_group(
    req: JsonBody<CreateGroupRequest>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();

        // 验证请求参数
        if req.group_id.is_empty() {
            warn!("群组ID为空");
            return Err(AppError::public("群组ID不能为空"));
        }
        if req.group_name.trim().is_empty() {
            warn!("群组名称为空");
            return Err(AppError::public("群组名称不能为空"));
        }

        error!("todo 未完成");

        json_ok(MyResponse::error_with_code(-1, "功能尚未完成"))
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 获得用户的群组列表
#[endpoint(tags("im_group"))]
pub async fn get_user_groups(depot: &mut Depot) -> JsonResult<MyResponse<Vec<ImGroup>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let owner_id = from_user.open_id.clone();
        info!("获取用户群组列表: user_id={}", owner_id);

        match im_group_service::get_user_groups(&owner_id).await {
            Ok(groups) => {
                info!(
                    "成功获取用户群组列表: owner_id={}, count={}",
                    owner_id,
                    groups.len()
                );
                json_ok(MyResponse::success_with_data("Ok", groups))
            }
            Err(e) => {
                warn!("获取用户群组列表失败: owner_id={}, error={:?}", owner_id, e);
                Err(e)
            }
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 获取群组列表
#[endpoint(tags("im_group"))]
pub async fn get_group(group_id: PathParam<String>) -> JsonResult<MyResponse<ImGroup>> {
    let group = im_group_service::get_group(&group_id).await?;
    json_ok(MyResponse::success_with_data("Ok", group))
}

/// 获取群组成员列表
#[endpoint(tags("im_group"))]
pub async fn get_group_members(
    group_id: PathParam<String>,
) -> JsonResult<MyResponse<Vec<ImGroupMember>>> {
    let members = im_group_service::get_group_members(&group_id).await?;
    json_ok(MyResponse::success_with_data("Ok", members))
}

/// 添加群组成员
#[endpoint(tags("im_group"))]
pub async fn add_group_member(
    group_id: PathParam<String>,
    member_id: PathParam<String>,
    req: JsonBody<AddGroupMemberRequest>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let group_id = group_id.into_inner();
        let member_id = member_id.into_inner();

        // 将 member_id 转换为 open_id（支持用户名、手机号、open_id、snowflake_id）
        let to_user = match user_service::get_by_name(&member_id).await {
            Ok(user) => Ok(user),
            Err(_) => match user_service::get_by_phone(&member_id).await {
                Ok(user) => Ok(user),
                Err(_) => user_service::get_by_open_id(&member_id).await,
            },
        };
        let from_id = from_user.open_id.clone();
        let to_user = match to_user {
            Ok(user) => user,
            Err(_) => {
                warn!("无法找到成员用户: member_id={}", member_id);
                return Err(AppError::not_found(
                    "未找到该用户，请确认用户ID或用户名是否正确",
                ));
            }
        };

        let to_id = to_user.open_id.clone();
        info!(
            "添加群成员: from_id={}, member_id={}, to_id={}",
            from_id, member_id, to_id
        );

        // 检查是否是好友，只有好友才能直接拉入群组
        match im_friendship_service::is_friend(&from_id, &to_id).await {
            Ok(true) => {
                info!("用户 {} 和 {} 是好友，可以直接添加到群组", from_id, to_id);
            }
            Ok(false) => {
                warn!("用户 {} 和 {} 不是好友，无法添加到群组", from_id, to_id);
                return Err(AppError::public("只能添加好友到群组"));
            }
            Err(e) => {
                warn!("检查好友关系失败: {:?}", e);
                return Err(AppError::public("检查好友关系失败"));
            }
        }

        // 在添加成员前，检查当前成员数
        let current_members = match im_group_service::get_group_members(&group_id).await {
            Ok(members) => members,
            Err(_) => {
                // 如果获取成员失败，可能是群组不存在，继续尝试添加（可能是新群组）
                vec![]
            }
        };

        let current_member_count = current_members.len();
        // 添加成员后，成员数会变成 current_member_count + 1
        let will_have_more_than_2_members = current_member_count >= 2;

        info!(
            "添加成员前: group_id={}, current_member_count={}, will_have_more_than_2_members={}",
            group_id, current_member_count, will_have_more_than_2_members
        );

        // 如果成员数大于2（即3人及以上），需要确保group_id存在且唯一
        let final_group_id = if will_have_more_than_2_members {
            // 检查群组是否存在
            match im_group_service::get_group(&group_id).await {
                Ok(_) => {
                    // 群组已存在，使用现有group_id
                    info!("群组已存在，使用现有group_id: {}", group_id);
                    group_id
                }
                Err(_) => {
                    // 群组不存在，创建一个新的唯一group_id
                    let new_group_id = Ulid::new().to_string();
                    info!(
                        "群组不存在，创建新的唯一group_id: {} -> {}",
                        group_id, new_group_id
                    );

                    let now = OffsetDateTime::now_utc();

                    // 获取群主（从现有成员中找角色最高的，或者使用第一个成员）
                    let owner_id = current_members
                        .iter()
                        .find(|m| m.role == 2)
                        .map(|m| m.member_id.clone())
                        .or_else(|| current_members.first().map(|m| m.member_id.clone()))
                        .unwrap_or_else(|| from_id.clone());

                    let new_group = ImGroup {
                        group_id: new_group_id.clone(),
                        owner_id: owner_id.clone(),
                        group_type: 1,               // 私有群
                        group_name: format!("群聊"), // 默认名称，前端可以修改
                        mute: Some(0),
                        apply_join_type: 1,
                        avatar: None,
                        max_member_count: None,
                        introduction: None,
                        notification: None,
                        status: Some(1),
                        sequence: Some(0),
                        create_time: Some(now),
                        update_time: Some(now),
                        extra: None,
                        version: Some(1),
                        del_flag: 1,
                        verifier: None,
                        member_count: None,
                    };

                    if let Err(e) = im_group_service::create_group(new_group).await {
                        warn!("创建群组失败: {:?}", e);
                        // 如果创建失败，继续使用原group_id（可能是临时ID）
                        group_id
                    } else {
                        info!("成功创建新群组: {}", new_group_id);

                        // 如果原group_id已有成员，需要更新这些成员的group_id到新的group_id
                        if !current_members.is_empty() {
                            // 更新所有现有成员的group_id（使用group_service的pool）
                            // 注意：需要更新group_member_id和group_id
                            for member in &current_members {
                                // 先删除旧的成员记录，然后插入新的（或者更新）
                                // 使用group_service的方法来更新
                                if let Err(e) = im_group_service::add_group_member(
                                    &new_group_id,
                                    &member.member_id,
                                    member.role,
                                    member.alias.clone(),
                                )
                                .await
                                {
                                    warn!(
                                        "迁移成员到新group_id失败: member_id={}, error={:?}",
                                        member.member_id, e
                                    );
                                } else {
                                    info!(
                                        "成功迁移成员到新group_id: {} -> {}, member_id={}",
                                        group_id, new_group_id, member.member_id
                                    );
                                }
                            }
                        }

                        new_group_id
                    }
                }
            }
        } else {
            group_id
        };

        let role = req.role.unwrap_or(0);

        let _ =
            im_group_service::add_group_member(&final_group_id, &to_id, role, req.alias).await?;

        info!("成功将用户 {} 添加到群组 {}", member_id, final_group_id);

        // 为新加入的成员创建聊天记录（如果还没有的话）
        // 这样即使没有发送过消息，群组也会出现在聊天列表中

        error!("todo 未完成");

        json_ok(MyResponse::error_with_code(-1, "功能尚未完成"))
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 移除群组成员
#[endpoint(tags("im_group"))]
pub async fn remove_group_member(
    group_id: PathParam<String>,
    member_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let member_id = member_id.into_inner();

        let operator_id = from_user.open_id.clone();
        info!(
            "移除群成员请求: group_id={}, member_id={}, operator_id={}",
            group_id, member_id, operator_id
        );

        let _ = im_group_service::remove_group_member(&group_id, &member_id, &operator_id).await?;

        json_ok(MyResponse::success_with_msg("Ok"))
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 更新群组成员角色
#[endpoint(tags("im_group"))]
pub async fn update_member_role(
    req: JsonBody<UpdateMemberRoleRequest>,
    group_id: PathParam<String>,
    member_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let group_id = group_id.into_inner();
        let member_id = member_id.into_inner();
        let operator_id = from_user.open_id.clone();

        info!(
            "更新群成员角色请求: group_id={}, member_id={}, role={}, operator_id={}",
            group_id, member_id, req.role, operator_id
        );

        match im_group_service::update_member_role(&group_id, &member_id, req.role, &operator_id)
            .await
        {
            Ok(_) => {
                let role_name = match req.role {
                    0 => "普通成员",
                    1 => "管理员",
                    2 => "群主",
                    _ => "未知",
                };
                info!(
                    "成功更新群成员角色: group_id={}, member_id={}, role={}",
                    group_id, member_id, role_name
                );
                json_ok(MyResponse::success_with_msg(f!("已设置为{}", role_name)))
            }
            Err(e) => {
                warn!(
                    "更新群成员角色失败: group_id={}, member_id={}, role={}, operator_id={}, error={:?}",
                    group_id, member_id, req.role, operator_id, e
                );
                Err(AppError::internal("更新群成员角色失败"))
            }
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 更新成员昵称
#[endpoint(tags("im_group"))]
pub async fn update_member_alias(
    req: JsonBody<UpdateMemberAliasRequest>,
    group_id: PathParam<String>,
    member_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let group_id = group_id.into_inner();
        let member_id = member_id.into_inner();
        let operator_id = from_user.open_id.clone();

        // 验证：只能修改自己的别名
        if member_id != operator_id {
            warn!(
                "用户 {} 尝试修改其他成员的别名: group_id={}, member_id={}",
                operator_id, group_id, member_id
            );
            return Err(AppError::public("只能修改自己的群昵称"));
        }

        info!(
            "更新群成员别名请求: group_id={}, member_id={}, alias={:?}",
            group_id, member_id, req.alias
        );

        match im_group_service::update_member_alias(&group_id, &member_id, req.alias.clone()).await
        {
            Ok(_) => {
                info!(
                    "成功更新群成员别名: group_id={}, member_id={}, alias={:?}",
                    group_id, member_id, req.alias
                );
                json_ok(MyResponse::success_with_msg("Ok"))
            }
            Err(e) => {
                warn!(
                    "更新群成员别名失败: group_id={}, member_id={}, alias={:?}, error={:?}",
                    group_id, member_id, req.alias, e
                );
                Err(e)
            }
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 移除群组
#[endpoint(tags("im_group"))]
pub async fn delete_group(
    group_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let owner_id = from_user.open_id.clone();
        info!("删除群组请求: group_id={}, owner_id={}", group_id, owner_id);

        let _ = im_group_service::delete_group(&group_id, &owner_id).await?;

        info!("成功删除群组: group_id={}, owner_id={}", group_id, owner_id);
        json_ok(MyResponse::success_with_msg("Ok"))
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 解散群组
#[endpoint(tags("im_group"))]
pub async fn dissolve_group(
    group_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let owner_id = from_user.open_id.clone();
        let subscription_service = depot
            .obtain::<Arc<SubscriptionService>>()
            .map_err(|_| AppError::internal("SubscriptionService not found"))?;
        info!(
            "解散群组请求: group_id={}, owner_id='{}'",
            group_id, owner_id
        );

        // 在解散前获取群组名称
        let group_name = match im_group_service::get_group(&group_id).await {
            Ok(g) => g.group_name,
            Err(_) => group_id.clone(),
        };

        let members = im_group_service::dissolve_group(&group_id, &owner_id).await?;
        info!("群组 {} 已解散", group_name);

        let message_id = Ulid::new().to_string();
        let system_message = f!(
            r#"{{"type":"group_dissolved","group_id":"{}","group_name":"{}","owner_id":"{}"}}"#,
            group_id,
            group_name,
            owner_id
        );

        // 保存系统消息到数据库
        let normalized_group_id = if group_id.starts_with("group_") {
            group_id.clone()
        } else {
            f!("group_{}", group_id)
        };
        let now = OffsetDateTime::now_utc();
        let group_message = ImGroupMessage {
            message_id: message_id.clone(),
            group_id: normalized_group_id.clone(),
            from_id: "system".to_string(),
            message_body: system_message.clone(),
            message_time: now,
            message_content_type: 100, // 系统消息类型
            extra: None,
            del_flag: 1,
            sequence: Some(now.unix_timestamp() * 1000),
            message_random: Some(Ulid::new().to_string()),
            create_time: now,
            update_time: Some(now),
            version: Some(1),
            reply_to: None,
        };

        if let Err(e) = im_message_service::save_group_message(group_message).await {
            warn!(
                "保存群组解散系统消息失败: group_id={}, error={:?}",
                group_id, e
            );
        }

        // 去重：使用 HashSet 确保每个 member_id 只处理一次
        let mut processed_member_ids = std::collections::HashSet::new();
        // 为每个群成员推送系统消息
        for member in &members {
            let member_id_str = &member.member_id;

            // 获取成员用户信息
            let member_user = match user_service::get_by_open_id(member_id_str).await {
                Ok(user) => user,
                Err(_) => match user_service::get_by_name(member_id_str).await {
                    Ok(user) => user,
                    Err(_) => {
                        warn!(member_id = %member_id_str, "无法找到群成员用户，跳过推送");
                        continue;
                    }
                },
            };

            let member_open_id = member_user.open_id.clone();

            // 如果已经处理过这个成员，跳过（去重）
            if !processed_member_ids.insert(member_open_id.clone()) {
                continue;
            }

            // 构建系统消息
            let chat_message = ChatMessage {
                message_id: message_id.clone(),
                from_user_id: "system".to_string(),
                to_user_id: normalized_group_id.clone(),
                message: system_message.clone(),
                timestamp_ms: now.unix_timestamp() * 1000,
                file_url: None,
                file_name: None,
                file_type: None,
                chat_type: Some(2), // 群聊
            };
            let conn = db::pool();
            // 从数据库查询订阅ID并同步到内存（如果内存中没有）
            let subscription_ids = {
                let mut ids = subscription_service.get_subscription_ids(member_user.id);
                if ids.is_empty() {
                    // 如果内存中没有，从数据库查询（只查询最近24小时内创建的订阅，过滤掉已不在线的用户）
                    if let Ok(db_subscriptions) = sqlx::query_scalar!(
                        r#"SELECT subscription_id FROM subscriptions
                         WHERE user_id = $1
                         AND created_at >= NOW() - INTERVAL '24 HOURS'
                         ORDER BY created_at DESC"#,
                        member_user.id
                    )
                    .fetch_all(conn)
                    .await
                    {
                        for sub_id in &db_subscriptions {
                            subscription_service
                                .add_subscription_id(sub_id.clone(), member_user.id);
                        }
                        ids = subscription_service.get_subscription_ids(member_user.id);
                    }
                }
                ids
            };
            error!("MQTT 待完成")
        }
        json_ok(MyResponse::success_with_msg("MQTT 待完成"))
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 更新群组信息
#[endpoint(tags("im_group"))]
pub async fn update_group(
    group_id: PathParam<String>,
    req: JsonBody<UpdateGroupRequest>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let req = req.into_inner();

        let owner_id = from_user.open_id.clone();
        info!(
            "更新群组信息请求: group_id={}, owner_id={}",
            group_id, owner_id
        );

        match im_group_service::update_group(&group_id, &owner_id, &req).await {
            Ok(_) => {
                info!(
                    "成功更新群组信息: group_id={}, owner_id={}",
                    group_id, owner_id
                );
                json_ok(MyResponse::success_with_msg("Ok"))
            }
            Err(e) => Err(e.into()),
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}
