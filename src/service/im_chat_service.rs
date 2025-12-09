/// 获取或创建聊天会话
pub async fn get_or_create_chat(chat_id: String, chat_type: i32, owner_id: String, to_id: String) {}

/// 获取用户的聊天会话列表
pub async fn get_user_chats(owner_id: &str) {}

/// 获取用户的聊天会话列表（包含名称信息）
pub async fn get_user_chats_with_names(owner_id: &str) {}

/// 更新会话序列号
#[allow(dead_code)]
pub async fn update_chat_sequence(chat_id: &str, sequence: i64) {}

/// 更新已读序列号
/// 更新群聊备注（仅自己可见）
pub async fn update_chat_remark(chat_id: &str, owner_id: &str, remark: Option<String>) {}

pub async fn update_read_sequence(chat_id: &str, read_sequence: i64) {}

/// 设置会话置顶
pub async fn set_chat_top(chat_id: &str, is_top: i16) {}

/// 设置会话免打扰
pub async fn set_chat_mute(chat_id: &str, is_mute: i16) {}

/// 删除聊天会话（软删除）
/// 同时删除相关的消息记录
pub async fn delete_chat(chat_id: &str, owner_id: &str) {}

/// 获取未读消息统计
/// 改进：不仅查询 im_chat 表，还直接查询消息表，确保即使没有聊天记录也能获取离线消息
pub async fn get_unread_message_stats(owner_id: &str) {}
