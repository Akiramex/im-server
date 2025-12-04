use salvo::prelude::*;

use crate::{
    AppError, JsonResult, MyResponse, SubscriptionService, db, json_ok,
    models::{ChatMessage, ImSingleMessage, User},
    service::{im_message_service, user_service},
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

#[endpoint(tags("im_message"))]
pub async fn send_single_message(
    req: JsonBody<SendSingleMessageRequest>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<()>> {
    if let Ok(user) = depot.obtain::<User>() {
        let req = req.into_inner();
        let subscription_service = depot
            .obtain::<Arc<SubscriptionService>>()
            .map_err(|_| AppError::internal("SubscriptionService not found"))?;
        let conn = db::pool();

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

                error!("嘟嘟嘟 --- MQTT功能待完成");
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

#[endpoint(tags("im_message"))]
pub async fn get_single_message() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn mark_single_message_read() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn send_group_message() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn get_group_message() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn mark_group_message_read() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn get_group_message_status() -> JsonResult<()> {
    todo!()
}

#[endpoint(tags("im_message"))]
pub async fn get_user_group_message_status() -> JsonResult<()> {
    todo!()
}
