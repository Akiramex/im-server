use salvo::{
    oapi::extract::{PathParam, QueryParam},
    prelude::*,
};

use crate::{
    db,
    dto::ImGroupMessageStatus,
    models::{ChatMessage, ImSingleMessage, User},
    mqtt,
    prelude::*,
    service::{im_chat_service, im_group_service, im_message_service, user_service},
    utils::{self, RedisClient, subcription::SubscriptionService},
};
use salvo::{
    Depot,
    oapi::{ToSchema, endpoint, extract::JsonBody},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{error, info, warn};
use ulid::Ulid;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SendSingleMessageRequest {
    pub from_id: String,
    pub to_id: String,
    pub message_body: String,
    pub message_content_type: i32,
    pub extra: Option<String>,
    pub reply_to: Option<String>,
}

/// 发送单聊信息
#[endpoint(tags("im_message"))]
pub async fn send_single_message(
    req: JsonBody<SendSingleMessageRequest>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(_user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let conn = db::pool();
        let subscription_service = depot
            .obtain::<Arc<SubscriptionService>>()
            .map_err(|_| AppError::internal("SubscriptionService not found"))?;
        let publisher = mqtt::get_mqtt_publisher();
        // 验证请求参数
        if req.from_id.is_empty() || req.to_id.is_empty() {
            return Err(AppError::public("from_id 和 to_id 不能为空"));
        }

        if req.message_body.is_empty() {
            return Err(AppError::public("消息内容不能为空"));
        }

        // 先获取发送者和接收者的 open_id，确保统一使用 open_id
        // 发送者：优先使用 open_id 查找，如果失败则尝试作为用户名查找
        let from_user = match user_service::get_by_open_id(&req.from_id).await {
            Ok(user) => user,
            Err(_) => {
                // 作为用户名查找
                match user_service::get_by_name(&req.from_id).await {
                    Ok(user) => user,
                    Err(_) => {
                        warn!(from_id = %req.from_id, "无法找到发送者用户");
                        return Err(AppError::not_found("发送者用户不存在"));
                    }
                }
            }
        };

        // 接收者：优先使用 open_id 查找，如果失败则尝试作为用户名查找
        let to_user = match user_service::get_by_open_id(&req.to_id).await {
            Ok(user) => user,
            Err(_) => {
                // 作为用户名查找
                match user_service::get_by_name(&req.to_id).await {
                    Ok(user) => user,
                    Err(_) => {
                        warn!(to_id = %req.to_id, "无法找到接收者用户");
                        return Err(AppError::not_found("接收者用户不存在"));
                    }
                }
            }
        };

        // 统一使用 open_id 作为消息的 from_id 和 to_id
        let from_open_id = from_user.open_id.clone();
        let to_open_id = to_user.open_id.clone();
        let now = OffsetDateTime::now_utc();
        let message_id = Ulid::new().to_string();
        let message = ImSingleMessage {
            message_id: message_id.clone(),
            from_id: from_open_id.clone(),
            to_id: to_open_id.clone(),
            message_body: req.message_body.clone(),
            message_time: OffsetDateTime::now_utc(),
            message_content_type: req.message_content_type,
            read_status: 0,
            extra: req.extra.clone(),
            del_flag: 1,
            sequence: now.unix_timestamp() * 1000, // 使用时间戳作为序列号
            message_random: Some(Ulid::new().to_string()),
            create_time: Some(now),
            update_time: Some(now),
            version: Some(1),
            reply_to: req.reply_to.clone(),
            to_type: Some("User".to_string()),
            file_url: None,
            file_name: None,
            file_type: None,
        };

        match im_message_service::save_single_message(message).await {
            Ok(_) => {
                // 解析extra字段获取文件信息
                let mut file_url = None;
                let mut file_name = None;
                let mut file_type = None;

                if let Some(extra_str) = &req.extra {
                    if let Ok(extra_json) = serde_json::from_str::<serde_json::Value>(extra_str) {
                        file_url = extra_json
                            .get("file_url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        file_name = extra_json
                            .get("file_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        file_type = extra_json
                            .get("file_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }

                let to_mqtt_id = to_user.open_id.clone();
                // 将ImSingleMessage转换为ChatMessage格式用于MQTT推送
                // 使用 open_id 作为 from_user_id 和 to_user_id，确保ID格式一致
                let chat_message = ChatMessage {
                    message_id: message_id.clone(),
                    from_user_id: from_open_id.clone(),
                    to_user_id: to_open_id.clone(),
                    message: req.message_body.clone(),
                    timestamp_ms: now.unix_timestamp() * 1000,
                    file_url,
                    file_name,
                    file_type,
                    chat_type: Some(1),
                };

                // 从数据库查询订阅ID并同步到内存（如果内存中没有）
                let subscription_ids = {
                    let mut ids = subscription_service.get_subscription_ids(to_user.id);
                    if ids.is_empty() {
                        // 如果内存中没有，从数据库查询（只查询最近24小时内创建的订阅，过滤掉已不在线的用户）
                        if let Ok(db_subscriptions) = sqlx::query_scalar!(
                            r#"
                                SELECT subscription_id FROM subscriptions
                                WHERE user_id = $1
                                AND created_at >= NOW() - INTERVAL '24 HOURS'
                                ORDER BY created_at DESC
                            "#,
                            to_user.id
                        )
                        .fetch_all(conn)
                        .await
                        {
                            for sub_id in &db_subscriptions {
                                subscription_service
                                    .add_subscription_id(sub_id.clone(), to_user.id);
                            }
                            ids = subscription_service.get_subscription_ids(to_user.id);
                        }
                    }
                    ids
                };

                // 判断用户是否在线
                let is_online = !subscription_ids.is_empty();
                let is_call_invite = req.message_content_type == 4;

                // 重要：对于通话邀请消息（message_content_type === 4），如果用户不在线，只存储到数据库，不推送
                // 因为通话邀请是实时消息，过期后没有意义，不应该在用户上线后弹出
                if is_call_invite && !is_online {
                    info!(
                        to_id = %req.to_id,
                        to_open_id = %to_open_id,
                        user_db_id = to_user.id,
                        to_mqtt_id = %to_mqtt_id,
                        message_id = %message_id,
                        message_content_type = 4,
                        "语音/视频呼叫消息，用户不在线，只存储到数据库，不推送（通话邀请是实时消息，过期后无意义）"
                    );
                    // 只存储到数据库，不通过 MQTT 推送，也不存储到 Redis
                    return json_ok(MyResponse::success_with_msg("Ok"));
                }

                // 对于普通消息或在线用户的通话邀请，正常处理：
                // 1. 消息已保存到数据库（上面已完成）
                // 2. 通过 MQTT 发布消息（broker 会自动处理离线消息，使用 QoS 1 和 clean_session=false）
                // 这样即使 MQTT 推送失败或用户不在线，消息也不会丢失
                // 注意：broker 只有在客户端已经订阅过 topic 的情况下才会存储离线消息
                // 如果用户从未连接过，broker 不会存储消息，但消息已保存到数据库，用户重连后可以从数据库获取
                let topic = utils::mqtt_user_topic(&to_mqtt_id.to_string());
                info!(
                    to_id = %req.to_id,
                    user_db_id = to_user.id,
                    to_mqtt_id = %to_mqtt_id,
                    has_subscription = is_online,
                    subscription_count = subscription_ids.len(),
                    %topic,
                    message_id = %message_id,
                    is_call_invite = is_call_invite,
                    "消息已保存到数据库，准备通过MQTT发布"
                );

                // 添加调试日志，确认 chat_type 是否正确设置
                info!(
                    to_id = %req.to_id,
                    to_mqtt_id = %to_mqtt_id,
                    %topic,
                    message_id = %message_id,
                    chat_type = ?chat_message.chat_type,
                    from_user_id = %chat_message.from_user_id,
                    to_user_id = %chat_message.to_user_id,
                    "准备编码并发布MQTT消息（单聊）"
                );

                match utils::encode_message(&chat_message) {
                    Ok(payload) => {
                        // 尝试解析 payload 以确认 chat_type 是否被正确序列化
                        if let Ok(decoded) = serde_json::from_slice::<serde_json::Value>(&payload) {
                            info!(
                                to_id = %req.to_id,
                                message_id = %message_id,
                                chat_type_in_payload = ?decoded.get("chat_type"),
                                "消息编码成功，chat_type 检查"
                            );
                        }
                        info!(
                            to_id = %req.to_id,
                            to_mqtt_id = %to_mqtt_id,
                            %topic,
                            message_id = %message_id,
                            payload_len = payload.len(),
                            "准备发布MQTT消息"
                        );

                        let mqtt_publish_result = publisher.publish(&topic, payload.clone()).await;

                        // 混合方案：MQTT + Redis 离线消息
                        // 1. MQTT 处理短期离线（用户曾经连接过，broker 会自动存储）
                        // 2. Redis 处理长期离线或从未连接的用户（作为备份）
                        // 重要：对于语音/视频呼叫消息（message_content_type === 4），如果用户不在线，不存储到 Redis
                        // 因为通话邀请是实时消息，过期后没有意义
                        // 注意：这里 is_online 一定是 true（因为离线用户的通话邀请已经在上面处理了）
                        let should_store_to_redis = if is_call_invite && !is_online {
                            // 语音/视频呼叫消息，用户不在线，不存储到 Redis
                            info!(
                                to_id = %req.to_id,
                                to_open_id = %to_open_id,
                                message_id = %message_id,
                                message_content_type = 4,
                                "语音/视频呼叫消息，用户不在线，不存储到 Redis（通话邀请是实时消息，过期后无意义）"
                            );
                            false
                        } else {
                            true
                        };

                        if let Err(e) = mqtt_publish_result {
                            error!(
                                to_id = %req.to_id,
                                to_mqtt_id = %to_mqtt_id,
                                %topic,
                                message_id = %message_id,
                                error = %e,
                                "MQTT 发布失败，将消息存储到 Redis 作为备份"
                            );
                            // MQTT 发布失败，存储到 Redis 作为备份（除非是语音/视频呼叫且用户不在线）
                            if should_store_to_redis {
                                if let Ok(payload_str) = String::from_utf8(payload.clone()) {
                                    if let Err(redis_err) =
                                        RedisClient::add_offline_message(&to_open_id, &payload_str)
                                            .await
                                    {
                                        warn!(
                                            to_id = %req.to_id,
                                            to_open_id = %to_open_id,
                                            error = %redis_err,
                                            "Redis 离线消息存储失败（消息已保存到数据库，不会丢失）"
                                        );
                                    } else {
                                        info!(
                                            to_id = %req.to_id,
                                            to_open_id = %to_open_id,
                                            message_id = %message_id,
                                            "✅ 消息已存储到 Redis（MQTT 发布失败时的备份）"
                                        );
                                    }
                                }
                            } else {
                                // MQTT 发布成功
                                // 重要：对于普通消息，无论用户是否在线，都存储到 Redis 作为备份
                                // 但对于语音/视频呼叫消息，如果用户不在线，不存储（因为过期后无意义）
                                // 原因：
                                // 1. 如果用户在线但 WebSocket 连接不稳定，消息可能丢失
                                // 2. 如果用户在消息发布后才连接，MQTT broker 不会存储消息（因为订阅发生在发布之后）
                                // 3. Redis 作为统一备份，确保消息不丢失
                                //
                                if should_store_to_redis {
                                    if let Ok(payload_str) = String::from_utf8(payload.clone()) {
                                        if let Err(redis_err) = RedisClient::add_offline_message(
                                            &to_open_id,
                                            &payload_str,
                                        )
                                        .await
                                        {
                                            warn!(
                                                to_id = %req.to_id,
                                                to_open_id = %to_open_id,
                                                error = %redis_err,
                                                "Redis 离线消息存储失败（MQTT 已发布，消息可能不会丢失）"
                                            );
                                        } else {
                                            if is_online {
                                                info!(
                                                    to_id = %req.to_id,
                                                    to_mqtt_id = %to_mqtt_id,
                                                    to_open_id = %to_open_id,
                                                    %topic,
                                                    message_id = %message_id,
                                                    "✅ 消息已保存到数据库，MQTT 发布成功，Redis 已备份（用户在线，三重保障）"
                                                );
                                            } else {
                                                info!(
                                                    to_id = %req.to_id,
                                                    to_open_id = %to_open_id,
                                                    message_id = %message_id,
                                                    "✅ 消息已保存到数据库，MQTT broker 和 Redis 双重存储（用户离线，确保消息不丢失）"
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    info!(
                                        to_id = %req.to_id,
                                        to_open_id = %to_open_id,
                                        message_id = %message_id,
                                        "✅ 消息已保存到数据库，MQTT 发布成功（语音/视频呼叫消息，用户不在线，不存储到 Redis）"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            message_id = %message_id,
                            error = %e,
                            "消息编码失败"
                        );
                    }
                }

                // 更新发送者和接收者的聊天记录
                // 注意：from_user 已经在上面获取过了，这里不需要重复获取

                let from_external_id = from_user.open_id.clone();
                let to_external_id = to_user.open_id.clone();
                let conn = db::pool();
                // 生成统一的 chat_id（使用排序后的用户ID，确保双方使用相同的 chat_id）
                let (min_id, max_id) = if from_external_id < to_external_id {
                    (&from_external_id, &to_external_id)
                } else {
                    (&to_external_id, &from_external_id)
                };
                let chat_id = format!("single_{}_{}", min_id, max_id);

                // 为发送者更新或创建聊天记录（发送者视角：to_id 是接收者）
                // 注意：即使 get_or_create_chat 失败，消息也已经保存并发送，不会影响消息接收
                if let Err(e) = im_chat_service::get_or_create_chat(
                    chat_id.clone(),
                    1, // chat_type: 1 = 单聊
                    from_external_id.clone(),
                    to_external_id.clone(),
                )
                .await
                {
                    warn!(chat_id = %chat_id, from_id = %from_external_id, to_id = %to_external_id, error = ?e, "创建或获取发送者聊天记录失败（消息已保存并发送，不影响消息接收）");
                } else {
                    // 更新聊天记录的 sequence 和 update_time（同时指定 chat_id、owner_id 和 chat_type，确保类型正确）
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE im_chat
                         SET sequence = $1, update_time = $2, version = version + 1
                         WHERE chat_id = $3 AND owner_id = $4 AND chat_type = 1
                         "#,
                        now.unix_timestamp() * 1000,
                        now,
                        chat_id,
                        from_external_id
                    )
                    .execute(conn)
                    .await
                    {
                        warn!(error = %e, "更新发送者聊天记录失败（消息已保存并发送，不影响消息接收）");
                    }
                }

                // 为接收者更新或创建聊天记录（接收者视角：to_id 是发送者）
                // 注意：这里使用相同的 chat_id，但 owner_id 和 to_id 不同
                // 注意：即使 get_or_create_chat 失败，消息也已经保存并发送，不会影响消息接收
                if let Err(e) = im_chat_service::get_or_create_chat(
                    chat_id.clone(),
                    1, // chat_type: 1 = 单聊
                    to_external_id.clone(),
                    from_external_id.clone(),
                )
                .await
                {
                    warn!(chat_id = %chat_id, from_id = %to_external_id, to_id = %from_external_id, error = ?e, "创建或获取接收者聊天记录失败（消息已保存并发送，不影响消息接收）");
                } else {
                    // 更新聊天记录的 sequence 和 update_time（同时指定 chat_id、owner_id 和 chat_type，确保类型正确）
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE im_chat
                         SET sequence = $1, update_time = $2, version = version + 1
                         WHERE chat_id = $3 AND owner_id = $4 AND chat_type = 1
                         "#,
                        now.unix_timestamp() * 1000,
                        now,
                        chat_id,
                        from_external_id
                    )
                    .execute(conn)
                    .await
                    {
                        warn!(error = %e, "更新接收者聊天记录失败（消息已保存并发送，不影响消息接收）");
                    }
                }

                return json_ok(MyResponse::success_with_msg("Ok"));
            }
            Err(e) => {
                error!(
                    "保存单聊消息失败: {:?}, 请求: from_id={}, to_id={}, message_body={}",
                    e, req.from_id, req.to_id, req.message_body
                );
                Err(AppError::internal(format!("发送消息失败: {:?}", e)))
            }
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SingleMessageParams {
    pub to_id: String,
    pub since_sequence: Option<i64>,
    pub limit: i32,
}

/// 获取单聊信息
#[endpoint(tags("im_message"))]
pub async fn get_single_message(
    depot: &mut Depot,
    params: QueryParam<SingleMessageParams, true>,
) -> JsonResult<MyResponse<Vec<ImSingleMessage>>> {
    if let Ok(user) = depot.obtain::<User>() {
        let req = params.into_inner();

        match im_message_service::get_single_messages(
            &user.open_id,
            &req.to_id,
            req.since_sequence,
            req.limit,
        )
        .await
        {
            Ok(message) => json_ok(MyResponse::success_with_data("Ok", message)),
            Err(e) => Err(AppError::internal(format!("获取消息失败: {:?}", e))),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[endpoint(tags("im_message"))]
pub async fn mark_single_message_read(
    depot: &mut Depot,
    message_id: PathParam<String>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(user) = depot.obtain::<User>() {
        let req = message_id.into_inner();
        match im_message_service::mark_single_message_read(&req, &user.open_id).await {
            Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
            Err(e) => Err(AppError::internal(format!("标记消息已读失败: {:?}", e))),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SendGroupMessageRequest {
    pub group_id: String,
    pub from_id: String,
    pub message_body: String,
    pub message_content_type: i32,
    pub extra: Option<String>,
    pub reply_to: Option<String>,
}

/// 发送群聊信息
#[endpoint(tags("im_message"))]
pub async fn send_group_message(
    depot: &mut Depot,
    req: JsonBody<SendGroupMessageRequest>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(_user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let pool = db::pool();
        let subscription_service = depot
            .obtain::<Arc<SubscriptionService>>()
            .map_err(|_| AppError::internal("SubscriptionService not found"))?;
        let publisher = mqtt::get_mqtt_publisher();
        let _redis_client = utils::RedisClient;

        // 验证请求参数
        if req.from_id.is_empty() {
            return Err(AppError::public("from_id 不能为空"));
        }

        if req.group_id.is_empty() {
            return Err(AppError::public("group_id 不能为空"));
        }

        if req.message_body.is_empty() {
            return Err(AppError::public("消息内容不能为空"));
        }

        // 先获取发送者的 open_id，确保统一使用 open_id
        let from_user = match user_service::get_by_open_id(&req.from_id).await {
            Ok(user) => user,
            Err(_) => {
                // 作为用户名查找
                match user_service::get_by_name(&req.from_id).await {
                    Ok(user) => user,
                    Err(_) => {
                        warn!(from_id = %req.from_id, "无法找到发送者用户");
                        return Err(AppError::not_found("发送者用户不存在"));
                    }
                }
            }
        };

        // 统一使用 open_id 作为消息的 from_id
        let from_open_id = from_user.open_id.clone();

        let now = OffsetDateTime::now_utc();
        let now_timestamp = now.unix_timestamp() * 1000;

        let message_id = Ulid::new().to_string();

        // 统一 group_id 格式：确保有 group_ 前缀
        let normalized_group_id = if req.group_id.starts_with("group_") {
            req.group_id.clone()
        } else {
            format!("group_{}", req.group_id)
        };

        info!(
            original_group_id = %req.group_id,
            normalized_group_id = %normalized_group_id,
            from_id = %from_open_id,
            message_id = %message_id,
            "开始发送群消息"
        );

        // 使用模块函数而不是服务实例

        // 先检查群组是否存在且未解散
        match im_group_service::get_group(&normalized_group_id).await {
            Ok(group) => {
                if group.del_flag == 0 {
                    warn!(
                        original_group_id = %req.group_id,
                        normalized_group_id = %normalized_group_id,
                        "群组已解散，无法发送消息"
                    );
                    return Err(AppError::public("群组已解散，无法发送消息"));
                }
            }
            Err(e) => {
                // 如果群组不存在，可能是2人聊天，继续处理
                // 但如果是3人及以上的群组，应该返回错误
                warn!(
                    original_group_id = %req.group_id,
                    normalized_group_id = %normalized_group_id,
                    error = ?e,
                    "群组不存在或已解散"
                );
                // 对于群组不存在的情况，我们仍然尝试获取成员
                // 如果成员数为0，说明群组确实不存在或已解散
            }
        }

        // 先获取群组的所有成员
        let members = match im_group_service::get_group_members(&normalized_group_id).await {
            Ok(members) => {
                info!(
                    original_group_id = %req.group_id,
                    normalized_group_id = %normalized_group_id,
                    member_count = members.len(),
                    "获取群成员成功"
                );
                members
            }
            Err(e) => {
                error!(
                    original_group_id = %req.group_id,
                    normalized_group_id = %normalized_group_id,
                    error = ?e,
                    "获取群成员失败"
                );
                return Err(AppError::internal(format!("获取群成员失败: {:?}", e)));
            }
        };

        // 根据 chat_type 决定使用单聊还是群聊逻辑，而不是根据成员数
        // 重要：以 chat_type 为主判断，人数只能作为辅助
        // 原因：有可能开始拉群人数超过2个人，后面群主把人员移除群聊，这个群就剩下他一个人
        // 如果以人数判断，就会有bug
        // 构建 chat_id：从 normalized_group_id 中提取原始 group_id（去掉 group_ 前缀）
        let original_group_id = normalized_group_id.trim_start_matches("group_").to_string();
        let chat_id = format!("group_{}", original_group_id);
        let chat_type = match im_chat_service::get_or_create_chat(
            chat_id.clone(),
            2, // 默认群聊类型
            from_open_id.clone(),
            normalized_group_id.clone(),
        )
        .await
        {
            Ok(chat) => {
                info!(
                    chat_id = %chat_id,
                    chat_type = chat.chat_type,
                    "从 im_chat 表获取 chat_type 成功"
                );
                chat.chat_type
            }
            Err(e) => {
                // 如果查询失败，默认使用群聊类型（chat_type = 2）
                warn!(
                    chat_id = %chat_id,
                    error = ?e,
                    "查询 chat_type 失败，默认使用群聊类型（chat_type = 2）"
                );
                2
            }
        };

        let member_count = members.len();
        let is_single_chat = chat_type == 1;

        info!(
            original_group_id = %req.group_id,
            normalized_group_id = %normalized_group_id,
            chat_type = chat_type,
            member_count = member_count,
            is_single_chat = is_single_chat,
            "根据 chat_type 决定聊天类型：chat_type=1为单聊，chat_type=2为群聊（人数仅作为辅助）"
        );

        // 解析extra字段获取文件信息
        let mut file_url = None;
        let mut file_name = None;
        let mut file_type = None;

        if let Some(extra_str) = &req.extra {
            if let Ok(extra_json) = serde_json::from_str::<serde_json::Value>(extra_str) {
                file_url = extra_json
                    .get("file_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                file_name = extra_json
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                file_type = extra_json
                    .get("file_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }

        // 根据 chat_type 决定保存到哪个表：chat_type=1保存到单聊表，chat_type=2保存到群聊表
        if is_single_chat {
            // chat_type=1（单聊）：保存到单聊表
            // 找到接收者（除了发送者之外的成员）
            let mut receiver_user_option = None;
            for member in &members {
                let member_user = match user_service::get_by_open_id(&member.member_id).await {
                    Ok(user) => user,
                    Err(_) => match user_service::get_by_name(&member.member_id).await {
                        Ok(user) => user,
                        Err(_) => continue,
                    },
                };
                let member_open_id = member_user.open_id.clone();
                let member_db_id = member_user.id;
                if member_open_id != from_open_id && member_db_id != from_user.id {
                    receiver_user_option = Some(member_user);
                    break;
                }
            }

            if let Some(receiver_user) = receiver_user_option {
                let receiver_open_id = receiver_user.open_id.clone();

                // 保存到单聊表（双向保存：from->to 和 to->from）
                let single_message = ImSingleMessage {
                    message_id: message_id.clone(),
                    from_id: from_open_id.clone(),
                    to_id: receiver_open_id.clone(),
                    message_body: req.message_body.clone(),
                    message_time: now,
                    message_content_type: req.message_content_type,
                    read_status: 0,
                    extra: req.extra.clone(),
                    del_flag: 1,
                    sequence: now_timestamp,
                    message_random: Some(Ulid::new().to_string()),
                    create_time: Some(now),
                    update_time: Some(now),
                    version: Some(1),
                    reply_to: req.reply_to.clone(),
                    to_type: Some("User".to_string()),
                    file_url: None,
                    file_name: None,
                    file_type: None,
                };

                match im_message_service::save_single_message(single_message).await {
                    Ok(_) => {
                        info!(group_id = %req.group_id, message_id = %message_id, chat_type = 1, "单聊消息已保存到单聊表");
                    }
                    Err(e) => {
                        error!(group_id = %req.group_id, error = ?e, chat_type = 1, "保存单聊消息到单聊表失败");
                        return Err(AppError::internal(format!("保存消息失败: {:?}", e)));
                    }
                }
            } else {
                error!(group_id = %req.group_id, chat_type = 1, "无法找到接收者，无法保存单聊消息");
                return Err(AppError::not_found("无法找到接收者"));
            }
        } else {
            // chat_type=2（群聊）：保存到群聊表
            let group_message = crate::models::ImGroupMessage {
                message_id: message_id.clone(),
                group_id: normalized_group_id.clone(),
                from_id: from_open_id.clone(), // 使用 open_id
                message_body: req.message_body.clone(),
                message_time: now,
                message_content_type: req.message_content_type,
                extra: req.extra.clone(),
                del_flag: 1,
                sequence: Some(now_timestamp),
                message_random: Some(Ulid::new().to_string()),
                create_time: now,
                update_time: Some(now),
                version: Some(1),
                reply_to: req.reply_to.clone(),
            };

            match im_message_service::save_group_message(group_message).await {
                Ok(_) => {
                    info!(group_id = %req.group_id, message_id = %message_id, chat_type = 2, "群聊消息已保存到群聊表");
                }
                Err(e) => {
                    error!(group_id = %req.group_id, error = ?e, chat_type = 2, "保存群聊消息到群聊表失败");
                    return Err(AppError::internal(format!("保存消息失败: {:?}", e)));
                }
            }
        }

        // 消息保存成功后，继续处理推送和聊天记录更新
        // 去重：使用 HashSet 确保每个 member_id 只处理一次
        // 这样可以避免数据库中有重复记录时导致重复发送消息
        use std::collections::HashSet;
        let mut processed_member_ids = HashSet::new();
        let mut skipped_sender_count = 0;
        let mut skipped_duplicate_count = 0;

        // 获取发送者的 open_id 和内部ID，用于比较
        let from_user_open_id = from_open_id.clone();
        let from_user_db_id = from_user.id;

        // 为每个群成员（除了发送者）推送消息
        for member in &members {
            let member_id_str = &member.member_id;

            // 获取成员用户信息（需要先获取才能比较）
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

            // 跳过发送者自己：比较 open_id 或数据库ID
            // 因为 member_id 可能是用户名、open_id 或 snowflake_id，需要统一比较
            let member_open_id = member_user.open_id.clone();
            let member_db_id = member_user.id;

            if member_open_id == from_user_open_id || member_db_id == from_user_db_id {
                skipped_sender_count += 1;
                info!(group_id = %req.group_id, member_id = %member_id_str, member_open_id = %member_open_id, from_open_id = %from_user_open_id, "跳过发送者自己");
                continue;
            }

            // 如果已经处理过这个成员，跳过（去重）
            // 使用 open_id 作为唯一标识，因为它是稳定的外部标识符
            if !processed_member_ids.insert(member_open_id.clone()) {
                skipped_duplicate_count += 1;
                warn!(group_id = %req.group_id, member_id = %member_id_str, member_open_id = %member_open_id, "检测到重复的群成员记录，跳过重复发送");
                continue;
            }

            // 获取成员的MQTT ID
            let member_mqtt_id = member_user.open_id.clone();

            // 根据 chat_type 决定聊天类型和接收者ID（以 chat_type 为主，而不是成员数）
            // chat_type=1（单聊），使用对方的 open_id 作为 to_user_id
            // chat_type=2（群聊），使用 group_id 作为 to_user_id
            let (chat_type_for_message, to_user_id) = if is_single_chat {
                // 单聊：使用对方的 open_id
                (Some(1), member_open_id.clone())
            } else {
                // 群聊：使用 normalized_group_id
                (Some(2), normalized_group_id.clone())
            };

            // 构建消息格式
            let chat_message = ChatMessage {
                message_id: message_id.clone(),
                from_user_id: from_user_open_id.clone(), // 使用 open_id
                to_user_id: to_user_id.clone(),          // 单聊使用对方 open_id，群聊使用 group_id
                message: req.message_body.clone(),
                timestamp_ms: now_timestamp,
                file_url: file_url.clone(),
                file_name: file_name.clone(),
                file_type: file_type.clone(),
                chat_type: chat_type_for_message, // 根据 chat_type 决定：chat_type=1（单聊），chat_type=2（群聊）
            };

            // 从数据库查询订阅ID并同步到内存（如果内存中没有）
            let subscription_ids = {
                let mut ids = subscription_service.get_subscription_ids(member_user.id);
                if ids.is_empty() {
                    // 如果内存中没有，从数据库查询（只查询最近24小时内创建的订阅，过滤掉已不在线的用户）
                    if let Ok(db_subscriptions) = sqlx::query_scalar!(
                        r#"
                        SELECT subscription_id FROM subscriptions
                        WHERE user_id = $1
                        AND created_at >= NOW() - INTERVAL '24 HOURS'
                        ORDER BY created_at DESC
                        "#,
                        member_user.id
                    )
                    .fetch_all(pool)
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

            // 通过 MQTT 发布消息给群成员
            // 注意：broker 只有在客户端已经订阅过 topic 的情况下才会存储离线消息
            // 如果用户从未连接过，broker 不会存储消息，但消息已保存到数据库，用户可以通过其他方式获取
            let topic = format!("user/{}", member_mqtt_id);
            let is_online = !subscription_ids.is_empty();
            info!(group_id = %req.group_id, member_id = %member_id_str, is_online = is_online, subscription_count = subscription_ids.len(), %topic, "通过MQTT发布群消息（broker会自动处理离线消息，前提是用户曾经订阅过topic）");

            // 添加调试日志，确认消息的 chat_type 是否正确设置
            let chat_type_str = if is_single_chat { "单聊" } else { "群聊" };
            info!(
                group_id = %req.group_id,
                member_id = %member_id_str,
                member_count = member_count,
                %topic,
                message_id = %message_id,
                chat_type = ?chat_message.chat_type,
                from_user_id = %chat_message.from_user_id,
                to_user_id = %chat_message.to_user_id,
                "准备编码并发布MQTT消息（{}）",
                chat_type_str
            );

            match serde_json::to_vec(&chat_message) {
                Ok(payload) => {
                    let payload_clone = payload.clone();
                    // 尝试解析 payload 以确认 chat_type 是否被正确序列化
                    if let Ok(decoded) = serde_json::from_slice::<serde_json::Value>(&payload) {
                        info!(
                            group_id = %req.group_id,
                            member_id = %member_id_str,
                            message_id = %message_id,
                            chat_type_in_payload = ?decoded.get("chat_type"),
                            "群组消息编码成功，chat_type 检查"
                        );
                    }

                    // 混合方案：MQTT + Redis 离线消息
                    // 1. MQTT 处理短期离线（用户曾经连接过，broker 会自动存储）
                    // 2. Redis 处理长期离线或从未连接的用户（作为备份）
                    // 先转换为 String，以便在多个地方使用
                    let payload_str_result = String::from_utf8(payload);

                    // 检查 payload 转换是否成功
                    if let Err(e) = &payload_str_result {
                        error!(
                            group_id = %req.group_id,
                            member_id = %member_id_str,
                            member_open_id = %member_open_id,
                            message_id = %message_id,
                            error = %e,
                            "⚠️ 群组消息 payload 转换为 String 失败，无法存储到 Redis"
                        );
                    }

                    if let Err(e) = publisher.publish(&topic, payload_clone).await {
                        error!(group_id = %req.group_id, member_id = %member_id_str, %topic, error = %e, message_id = %message_id, chat_type = ?chat_type, "消息MQTT发布失败，将消息存储到 Redis 作为备份");
                        // MQTT 发布失败，存储到 Redis 作为备份
                        match payload_str_result {
                            Ok(payload_str) => {
                                if let Err(redis_err) =
                                    RedisClient::add_offline_message(&member_open_id, &payload_str)
                                        .await
                                {
                                    warn!(
                                        group_id = %req.group_id,
                                        member_id = %member_id_str,
                                        member_open_id = %member_open_id,
                                        error = %redis_err,
                                        "Redis 离线消息存储失败（消息已保存到数据库，不会丢失）"
                                    );
                                } else {
                                    info!(
                                        group_id = %req.group_id,
                                        member_id = %member_id_str,
                                        member_open_id = %member_open_id,
                                        message_id = %message_id,
                                        chat_type = ?chat_type,
                                        "✅ 消息已存储到 Redis（MQTT 发布失败时的备份）"
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    group_id = %req.group_id,
                                    member_id = %member_id_str,
                                    member_open_id = %member_open_id,
                                    message_id = %message_id,
                                    error = %e,
                                    "⚠️ 无法将消息存储到 Redis（payload 转换失败）"
                                );
                            }
                        }
                    } else {
                        // MQTT 发布成功
                        // 重要：无论用户是否在线，都存储到 Redis 作为备份
                        // 原因：
                        // 1. 如果用户在线但 WebSocket 连接不稳定，消息可能丢失
                        // 2. 如果用户在消息发布后才连接，MQTT broker 不会存储消息（因为订阅发生在发布之后）
                        // 3. Redis 作为统一备份，确保消息不丢失
                        match payload_str_result {
                            Ok(payload_str) => {
                                info!(
                                    group_id = %req.group_id,
                                    member_id = %member_id_str,
                                    member_open_id = %member_open_id,
                                    message_id = %message_id,
                                    chat_type = ?chat_type,
                                    payload_length = payload_str.len(),
                                    "准备存储群组消息到 Redis"
                                );

                                if let Err(redis_err) =
                                    RedisClient::add_offline_message(&member_open_id, &payload_str)
                                        .await
                                {
                                    warn!(
                                        group_id = %req.group_id,
                                        member_id = %member_id_str,
                                        member_open_id = %member_open_id,
                                        message_id = %message_id,
                                        chat_type = ?chat_type,
                                        error = %redis_err,
                                        "❌ Redis 离线消息存储失败（MQTT 已发布，消息可能不会丢失）"
                                    );
                                } else {
                                    if is_online {
                                        info!(
                                            group_id = %req.group_id,
                                            member_id = %member_id_str,
                                            member_open_id = %member_open_id,
                                            %topic,
                                            message_id = %message_id,
                                            chat_type = ?chat_type,
                                            "✅ 消息已保存到数据库，MQTT 发布成功，Redis 已备份（用户在线，三重保障）"
                                        );
                                    } else {
                                        info!(
                                            group_id = %req.group_id,
                                            member_id = %member_id_str,
                                            member_open_id = %member_open_id,
                                            message_id = %message_id,
                                            chat_type = ?chat_type,
                                            "✅ 消息已保存到数据库，MQTT broker 和 Redis 双重存储（用户离线，确保消息不丢失）"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    group_id = %req.group_id,
                                    member_id = %member_id_str,
                                    member_open_id = %member_open_id,
                                    message_id = %message_id,
                                    error = %e,
                                    "⚠️ 无法将消息存储到 Redis（payload 转换失败），MQTT 已发布"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(member_id = %member_id_str, error = %e, "群消息编码失败");
                }
            }
        }

        info!(
            group_id = %req.group_id,
            message_id = %message_id,
            total_members = members.len(),
            member_count = member_count,
            is_single_chat = is_single_chat,
            processed_count = processed_member_ids.len(),
            skipped_sender = skipped_sender_count,
            skipped_duplicate = skipped_duplicate_count,
            "消息发送完成（{}）",
            if is_single_chat { "单聊" } else { "群聊" }
        );

        // 更新聊天记录（为所有成员更新，包括发送者）
        let from_external_id = from_user.open_id.clone();

        // 如果是单聊，需要为发送者和接收者都创建聊天记录
        if is_single_chat {
            // 找到接收者
            let mut receiver_user_option = None;
            for member in &members {
                let member_user = match user_service::get_by_open_id(&member.member_id).await {
                    Ok(user) => user,
                    Err(_) => match user_service::get_by_name(&member.member_id).await {
                        Ok(user) => user,
                        Err(_) => continue,
                    },
                };
                let member_open_id = member_user.open_id.clone();
                let member_db_id = member_user.id;
                if member_open_id != from_external_id && member_db_id != from_user_db_id {
                    receiver_user_option = Some(member_user);
                    break;
                }
            }

            if let Some(receiver_user) = receiver_user_option {
                let receiver_external_id = receiver_user.open_id.clone();

                // 生成统一的 chat_id（使用排序后的用户ID）
                let (min_id, max_id) = if from_external_id < receiver_external_id {
                    (&from_external_id, &receiver_external_id)
                } else {
                    (&receiver_external_id, &from_external_id)
                };
                let chat_id = format!("single_{}_{}", min_id, max_id);

                // 为发送者创建聊天记录
                if let Err(e) = im_chat_service::get_or_create_chat(
                    chat_id.clone(),
                    1, // chat_type: 1 = 单聊
                    from_external_id.clone(),
                    receiver_external_id.clone(),
                )
                .await
                {
                    warn!(chat_id = %chat_id, member_id = %from_external_id, error = ?e, "创建或获取发送者聊天记录失败");
                } else {
                    // 更新聊天记录的 sequence 和 update_time
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE im_chat
                        SET sequence = $1, update_time = $2, version = version + 1
                        WHERE chat_id = $3 AND owner_id = $4 AND chat_type = 1
                        "#,
                        now_timestamp,
                        now,
                        &chat_id,
                        &from_external_id
                    )
                    .execute(pool)
                    .await
                    {
                        warn!(error = %e, "更新发送者聊天记录失败");
                    }
                }

                // 为接收者创建聊天记录
                if let Err(e) = im_chat_service::get_or_create_chat(
                    chat_id.clone(),
                    1, // chat_type: 1 = 单聊
                    receiver_external_id.clone(),
                    from_external_id.clone(),
                )
                .await
                {
                    warn!(chat_id = %chat_id, member_id = %receiver_external_id, error = ?e, "创建或获取接收者聊天记录失败");
                } else {
                    // 更新聊天记录的 sequence 和 update_time
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE im_chat
                        SET sequence = $1, update_time = $2, version = version + 1
                        WHERE chat_id = $3 AND owner_id = $4 AND chat_type = 1
                        "#,
                        now_timestamp,
                        now,
                        &chat_id,
                        &receiver_external_id
                    )
                    .execute(pool)
                    .await
                    {
                        warn!(error = %e, "更新接收者聊天记录失败");
                    }
                }
            }
        }

        // 为群成员（除了发送者）更新聊天记录（仅群聊）
        for member in &members {
            let member_user = match user_service::get_by_open_id(&member.member_id).await {
                Ok(user) => user,
                Err(_) => match user_service::get_by_name(&member.member_id).await {
                    Ok(user) => user,
                    Err(_) => {
                        warn!(member_id = %member.member_id, "无法找到群成员用户，跳过更新聊天记录");
                        continue;
                    }
                },
            };

            let member_external_id = member_user.open_id.clone();

            // 只处理群聊的聊天记录（单聊已经在上面处理了）
            if !is_single_chat {
                // 群聊：使用 group_ 前缀
                let chat_id = format!("group_{}", req.group_id);
                let to_id = req.group_id.clone();

                // 为每个成员更新或创建群聊记录
                if let Err(e) = im_chat_service::get_or_create_chat(
                    chat_id.clone(),
                    2, // chat_type: 2 = 群聊
                    member_external_id.clone(),
                    to_id.clone(),
                )
                .await
                {
                    warn!(chat_id = %chat_id, member_id = %member_external_id, error = ?e, "创建或获取群聊记录失败");
                } else {
                    // 更新聊天记录的 sequence 和 update_time
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE im_chat
                        SET sequence = $1, update_time = $2, version = version + 1
                        WHERE chat_id = $3 AND owner_id = $4 AND chat_type = 2
                        "#,
                        now_timestamp,
                        now,
                        &chat_id,
                        &member_external_id
                    )
                    .execute(pool)
                    .await
                    {
                        warn!(error = %e, "更新群组聊天记录失败");
                    }
                }
            }
        }

        Ok(Json(MyResponse::success_with_msg("消息发送成功")))
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct GroupMessageParams {
    pub since_sequence: Option<i64>,
    pub limit: i32,
}

/// 获取群聊信息
#[endpoint(tags("im_message"))]
pub async fn get_group_message(
    _depot: &mut Depot,
    group_id: PathParam<String>,
    _params: QueryParam<GroupMessageParams, true>,
) -> JsonResult<()> {
    let _req = group_id.into_inner();
    todo!()
}

/// 标记群聊信息已读
#[endpoint(tags("im_message"))]
pub async fn mark_group_message_read(
    depot: &mut Depot,
    group_id: PathParam<String>,
    message_id: PathParam<String>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let message_id = message_id.into_inner();

        let to_id = from_user.open_id.clone();

        // 获取群组成员数，决定使用哪个表的已读标记
        let members = match im_group_service::get_group_members(&group_id).await {
            Ok(members) => members,
            Err(e) => {
                warn!("获取群成员失败: {:?}", e);
                return Err(e.into());
            }
        };

        let member_count = members.len();
        let is_single_chat = member_count == 2;

        // 根据成员数决定使用哪个表的已读标记
        if is_single_chat {
            // 2人聊天：使用单聊表的 read_status 字段
            match im_message_service::mark_single_message_read(&message_id, &to_id).await {
                Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
                Err(_e) => Err(AppError::internal("标记单聊消息已读失败")),
            }
        } else {
            // 3人及以上：使用群聊消息状态表
            match im_message_service::mark_group_message_read(&group_id, &message_id, &to_id).await
            {
                Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
                Err(_e) => Err(AppError::internal("标记群消息已读失败")),
            }
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[endpoint(tags("im_message"))]
pub async fn get_group_message_status(
    group_id: PathParam<String>,
    message_id: PathParam<String>,
) -> JsonResult<MyResponse<Vec<ImGroupMessageStatus>>> {
    let group_id = group_id.into_inner();
    let message_id = message_id.into_inner();
    match im_message_service::get_group_message_status(&group_id, &message_id).await {
        Ok(status) => json_ok(MyResponse::success_with_data("Ok", status)),
        Err(err) => Err(err.into()),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
struct LimitParam(String);

#[endpoint(tags("im_message"))]
pub async fn get_user_group_message_status(
    depot: &mut Depot,
    group_id: PathParam<String>,
    params: QueryParam<LimitParam, false>,
) -> JsonResult<MyResponse<Vec<ImGroupMessageStatus>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let group_id = group_id.into_inner();
        let to_id = from_user.open_id.clone();
        let limit = params.into_inner().and_then(|s| s.0.parse::<i32>().ok());

        match im_message_service::get_user_group_message_status(&group_id, &to_id, limit).await {
            Ok(status) => json_ok(MyResponse::success_with_data("Ok", status)),
            Err(err) => Err(err.into()),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}
