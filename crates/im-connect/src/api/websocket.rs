use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use crate::config::{self, JwtConfig};
use crate::prelude::*;
use im_share::mqtt::{ImMqtt, MqttConfig};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::header::AUTHORIZATION;
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use serde::{Deserialize, Serialize};
use time::{Duration, UtcDateTime};
use tokio::sync::RwLock;

#[derive(Deserialize, Serialize, Debug)]
pub struct JwtClaims {
    pub open_id: u64,
    pub exp: i64,
    pub iat: i64,
}

impl JwtClaims {
    pub fn new(open_id: u64, exp: i64) -> Self {
        let now = UtcDateTime::now();
        let exp = now + Duration::hours(exp);
        JwtClaims {
            open_id,
            exp: exp.unix_timestamp(),
            iat: now.unix_timestamp(),
        }
    }
}
pub fn verify_token(token: &str, jwt_config: &JwtConfig) -> anyhow::Result<JwtClaims> {
    let claims = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(claims.claims)
}

#[handler]
pub async fn ws_handler(
    req: &mut Request,
    res: &mut Response,
    _ctrl: &mut FlowCtrl,
) -> &'static str {
    let jwt_cfg = &config::get().jwt;

    let subscription_id: String = req
        .params()
        .get("subscription_id")
        .cloned()
        .unwrap_or_default();

    let token = req.header::<String>(AUTHORIZATION).and_then(|s| {
        if s.starts_with("Bearer ") {
            Some(s[7..].to_string())
        } else {
            None
        }
    });

    let token = match token {
        Some(t) => t,
        None => {
            warn!(%subscription_id, "WebSocket 升级请求缺少 Authorization token");
            return "缺少认证 token";
        }
    };

    let claims = match verify_token(&token, jwt_cfg) {
        Ok(c) => c,
        Err(e) => {
            warn!(%subscription_id, error = %e, "WebSocket token 验证失败");
            return "缺少认证 token";
        }
    };

    info!(
        %subscription_id,
        user_id = %claims.open_id,
        "WebSocket token 验证成功"
    );

    // 从 token 中提取用户信息
    // 如果 token 中包含 open_id（is_open_id = true），直接从 token 获取，无需查询数据库
    // 这样可以避免不必要的数据库查询，提高性能
    let (user_mqtt_id, user_open_id) = {
        // Token 中包含 open_id 的数字形式（雪花算法生成的）
        // 直接使用，无需查询数据库
        let open_id = claims.open_id.to_string();
        info!(
            %subscription_id,
            open_id = %open_id,
            mqtt_id = %claims.open_id,
            "从 token 直接获取用户信息（无需查询数据库）"
        );
        (claims.open_id, open_id)
    };

    let _ = WebSocketUpgrade::new()
        .upgrade(req, res, move |socket| {
            handler_websocket_connection(socket, subscription_id, user_mqtt_id, user_open_id)
        })
        .await;

    ""
}

fn mqtt_user_topic(user_id: &str) -> String {
    format!("user/{user_id}/inbox")
}

async fn handler_websocket_connection(
    mut socket: WebSocket,
    subscription_id: String,
    user_mqtt_id: u64,
    user_open_id: String,
) {
    // 使用基于 user_mqtt_id (snowflake_id) 的固定 client_id，确保同一用户的会话可以恢复
    // 这样 MQTT broker 可以为离线用户存储消息
    // 注意：使用 user_mqtt_id (snowflake_id) 而不是 subscription_id，因为：
    // 1. subscription_id 每次连接都会变化（每次登录生成新的）
    // 2. user_mqtt_id (snowflake_id) 和 open_id 是用户唯一且不变的标识符
    // 3. 如果同一用户有多个设备，它们会共享同一个 MQTT 会话，broker 会推送消息给所有连接的设备

    let client_id = format!("im-conn-{}", user_mqtt_id);
    info!(
        subscription_id = %subscription_id,
        open_id = %user_open_id,
        mqtt_id = %user_mqtt_id,
        client_id = %client_id,
        "创建MQTT客户端（使用固定client_id以支持离线消息，基于open_id/mqtt_id而非subscription_id）"
    );

    let mqtt_info = &crate::config::get().mqtt;

    let im = Arc::new(ImMqtt::connect(MqttConfig::new(
        mqtt_info.host.clone(),
        mqtt_info.port,
        client_id.clone(),
    )));

    info!(
        subscription_id = %subscription_id,
        open_id = %user_open_id,
        mqtt_id = %user_mqtt_id,
        client_id = %client_id,
        "为用户创建独立的MQTT客户端（基于唯一标识符open_id）"
    );

    let topic = mqtt_user_topic(&user_mqtt_id.to_string());

    // 保存 topic 用于后续取消订阅
    let topic_for_unsubscribe = topic.clone();

    info!(
        subscription_id = %subscription_id,
        open_id = %user_open_id,
        mqtt_id = %user_mqtt_id,
        %topic,
        client_id = %client_id,
        "准备订阅MQTT topic（基于唯一标识符mqtt_id）"
    );

    let mut rx = match im.subscribe(&topic).await {
        Ok(r) => {
            info!(
                subscription_id = %subscription_id,
                open_id = %user_open_id,
                mqtt_id = %user_mqtt_id,
                topic = %topic,
                client_id = %client_id,
                "✅ MQTT订阅成功（QoS 1），等待broker推送消息（包括离线消息，基于唯一标识符open_id）"
            );

            // 注意：subscribe 方法返回的 Receiver 表示已成功订阅
            // 如果返回了 Receiver，说明订阅成功，broadcast channel 中已经有接收者
            info!(
                subscription_id = %subscription_id,
                open_id = %user_open_id,
                mqtt_id = %user_mqtt_id,
                topic = %topic,
                "MQTT订阅确认：已获得 broadcast channel 接收者，可以接收消息"
            );

            // 等待一小段时间，让broker有时间推送离线消息
            // 注意：这不是必需的，因为broker会在订阅确认后立即推送离线消息
            // 但添加这个延迟可以帮助调试，确保订阅完全建立
            // 同时，broker推送离线消息可能需要一些时间
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            info!(
                subscription_id = %subscription_id,
                mqtt_id = %user_mqtt_id,
                topic = %topic,
                "开始监听MQTT消息（broker应该已经推送了离线消息，如果有的话；注意：只有订阅后发布的消息才会被broker存储）"
            );

            r
        }
        Err(e) => {
            error!(
                subscription_id = %subscription_id,
                open_id = %user_open_id,
                mqtt_id = %user_mqtt_id,
                topic = %topic,
                client_id = %client_id,
                error = %e,
                "❌ MQTT订阅失败"
            );
            // 发送关闭帧并关闭连接
            let _ = socket.send(Message::close()).await;
            return;
        }
    };

    info!(
        subscription_id = %subscription_id,
        open_id = %user_open_id,
        mqtt_id = %user_mqtt_id,
        %topic,
        "WS已连接，已订阅MQTT（基于唯一标识符open_id，subscription_id仅用于本次连接）"
    );

    // 定期发送 ping 保持连接活跃
    let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // 跟踪连接是否已经关闭，避免在已关闭的连接上发送关闭帧
    let mut connection_closed = false;

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if let Err(e) = socket.send(Message::ping(vec![])).await {
                    warn!(%subscription_id, user_id = %user_mqtt_id, error = %e, "发送 ping 失败");
                    connection_closed = true;
                    break;
                }
            }
            incoming = rx.recv() => {
                match incoming {
                    Ok(msg) => {
                        info!(
                            subscription_id = %subscription_id,
                            open_id = %user_open_id,
                            mqtt_id = %user_mqtt_id,
                            received_topic = %msg.topic,
                            expected_topic = %topic,
                            payload_len = msg.payload.len(),
                            "📨 收到MQTT消息（从broadcast channel）"
                        );

                        if msg.topic != topic {
                            warn!(
                                subscription_id = %subscription_id,
                                open_id = %user_open_id,
                                mqtt_id = %user_mqtt_id,
                                received_topic = %msg.topic,
                                expected_topic = %topic,
                                "收到不匹配的topic消息，跳过（可能是订阅了多个topic）"
                            );
                            continue;
                        }

                        // 尝试解析消息内容用于调试
                        let message_id = if let Ok(text) = String::from_utf8(msg.payload.clone()) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                let msg_id = json.get("message_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                                info!(
                                    subscription_id = %subscription_id,
                                    mqtt_id = %user_mqtt_id,
                                    message_id = ?msg_id,
                                    chat_type = ?json.get("chat_type"),
                                    from_user_id = ?json.get("from_user_id"),
                                    to_user_id = ?json.get("to_user_id"),
                                    topic = %msg.topic,
                                    payload_len = msg.payload.len(),
                                    "✅ 处理MQTT消息（topic匹配，消息内容解析成功，准备发送到WebSocket客户端）"
                                );
                                msg_id
                            } else {
                                info!(
                                    subscription_id = %subscription_id,
                                    mqtt_id = %user_mqtt_id,
                                    topic = %msg.topic,
                                    payload_len = msg.payload.len(),
                                    payload_preview = %text.chars().take(100).collect::<String>(),
                                    "✅ 处理MQTT消息（topic匹配，但无法解析为JSON，准备发送到WebSocket客户端）"
                                );
                                None
                            }
                        } else {
                            info!(
                                subscription_id = %subscription_id,
                                mqtt_id = %user_mqtt_id,
                                topic = %msg.topic,
                                payload_len = msg.payload.len(),
                                "✅ 处理MQTT消息（topic匹配，二进制消息，准备发送到WebSocket客户端）"
                            );
                            None
                        };

                        // 直接使用原始消息，不进行ID转换
                        // 前端可以处理 open_id，不需要转换为用户名
                        // 这样可以避免异步转换导致的延迟和错误
                        let payload = msg.payload;
                        let payload_len = payload.len();

                        // 尝试解析消息内容用于日志
                        let message_text = if let Ok(text) = String::from_utf8(payload.clone()) {
                            Some(text)
                        } else {
                            None
                        };

                        let send_result = match &message_text {
                            Some(text) => {
                                info!(
                                    subscription_id = %subscription_id,
                                    mqtt_id = %user_mqtt_id,
                                    message_id = ?message_id,
                                    message_len = text.len(),
                                    "发送文本消息到WebSocket客户端"
                                );
                                socket.send(Message::text(text.clone())).await
                            },
                            None => {
                                info!(
                                    subscription_id = %subscription_id,
                                    mqtt_id = %user_mqtt_id,
                                    message_id = ?message_id,
                                    payload_len = payload_len,
                                    "发送二进制消息到WebSocket客户端"
                                );
                                socket.send(Message::binary(payload)).await
                            },
                        };

                        match send_result {
                            Ok(_) => {
                                info!(
                                    subscription_id = %subscription_id,
                                    mqtt_id = %user_mqtt_id,
                                    message_id = ?message_id,
                                    payload_len = payload_len,
                                    "✅ 消息已成功发送到WebSocket客户端"
                                );
                            },
                            Err(e) => {
                                warn!(
                                    %subscription_id,
                                    user_id = %user_mqtt_id,
                                    error = %e,
                                    "❌ 发送消息到客户端失败"
                                );
                                // 发送失败通常意味着连接已断开，退出循环
                                connection_closed = true;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        // broadcast channel 错误通常表示：
                        // 1. 通道已关闭（所有发送者都关闭了）
                        // 2. 接收者滞后太多（消息积压超过256条）
                        let error_str = e.to_string();
                        if error_str.contains("channel closed") || error_str.contains("closed") {
                            warn!(
                                subscription_id = %subscription_id,
                                open_id = %user_open_id,
                                mqtt_id = %user_mqtt_id,
                                error = %e,
                                "MQTT broadcast channel 已关闭（MQTT连接可能已断开）"
                            );
                            connection_closed = true;
                            break;
                        } else {
                            warn!(
                                subscription_id = %subscription_id,
                                open_id = %user_open_id,
                                mqtt_id = %user_mqtt_id,
                                error = %e,
                                "MQTT接收通道错误（可能是消息积压，等待后重试）"
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
            from_client = socket.recv() => {
                match from_client {
                    None => {
                        info!(%subscription_id, user_id = %user_mqtt_id, open_id = %user_open_id, "WS关闭");
                        // 从在线用户列表移除
                        {
                            let mut online_users = ONLINE_USERS.write().await;
                            if let Some(subs) = online_users.get_mut(&user_mqtt_id) {
                                subs.remove(&subscription_id);
                                if subs.is_empty() {
                                    online_users.remove(&user_mqtt_id);
                                }
                            }
                        }
                        connection_closed = true;
                        break;
                    }
                    Some(Ok(message)) => {
                        if message.is_close() {
                            info!(%subscription_id, user_id = %user_mqtt_id, open_id = %user_open_id, "WS关闭");
                            // 从在线用户列表移除
                            {
                                let mut online_users = ONLINE_USERS.write().await;
                                if let Some(subs) = online_users.get_mut(&user_mqtt_id) {
                                    subs.remove(&subscription_id);
                                    if subs.is_empty() {
                                        online_users.remove(&user_mqtt_id);
                                    }
                                }
                            }
                            connection_closed = true;
                            break;
                        }

                        if message.is_ping() {
                            // 收到 ping，回复 pong
                            let Ok(message_str) = message.as_str() else {
                                break;
                            };
                            // 复制字符串数据以避免生命周期问题
                            let message_data = message_str.to_string();
                            if let Err(e) = socket.send(Message::pong(message_data)).await {
                                warn!(%subscription_id, user_id = %user_mqtt_id, error = %e, "回复 pong 失败");
                                connection_closed = true;
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!(%subscription_id, user_id = %user_mqtt_id, open_id = %user_open_id, error = %e, "WS接收错误");
                        // 从在线用户列表移除
                        {
                            let mut online_users = ONLINE_USERS.write().await;
                            if let Some(subs) = online_users.get_mut(&user_mqtt_id) {
                                subs.remove(&subscription_id);
                                if subs.is_empty() {
                                    online_users.remove(&user_mqtt_id);
                                }
                            }
                        }
                        // 检查错误类型，如果是连接重置或已关闭，不需要发送关闭帧
                        let error_str = e.to_string();
                        let is_connection_reset = error_str.contains("Connection reset")
                            || error_str.contains("connection reset")
                            || error_str.contains("Broken pipe")
                            || error_str.contains("broken pipe")
                            || error_str.contains("Connection aborted")
                            || error_str.contains("connection aborted")
                            || error_str.contains("Sending after closing")
                            || error_str.contains("sending after closing");

                        // 只有在连接仍然有效时才尝试发送关闭帧
                        if !is_connection_reset {
                            if let Err(close_err) = socket.send(Message::close()).await {
                                // 如果发送关闭帧也失败，说明连接已经关闭
                                let close_err_str = close_err.to_string();
                                if close_err_str.contains("Sending after closing")
                                    || close_err_str.contains("sending after closing") {
                                    connection_closed = true;
                                }
                            }
                        } else {
                            connection_closed = true;
                        }
                        break;
                    }
                }
            }
        }
    }
}

// 在线用户列表（user_id -> subscription_id 集合，支持多设备）
// 这是 im-connect 特有的，用于跟踪在线用户
pub static ONLINE_USERS: LazyLock<Arc<RwLock<HashMap<u64, std::collections::HashSet<String>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
