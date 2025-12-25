use crate::{
    models::{ChatWithName, ImChat, User},
    prelude::*,
    service::im_chat_service,
};
use salvo::{
    oapi::extract::{JsonBody, PathParam},
    prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize, ToSchema)]
pub struct UpdateChatRequest {
    pub is_top: Option<i16>,
    pub is_mute: Option<i16>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateChatRequest {
    pub chat_id: String,
    pub chat_type: i32,
    pub to_id: String,
}

#[endpoint(tags("im_chat"))]
pub async fn get_or_create_chat(
    depot: &mut Depot,
    req: JsonBody<CreateChatRequest>,
) -> JsonResult<MyResponse<ImChat>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();

        // 使用 open_id 作为 owner_id（与创建聊天记录时保持一致）
        match im_chat_service::get_or_create_chat(
            req.chat_id,
            req.chat_type,
            from_user.open_id.clone(),
            req.to_id,
        )
        .await
        {
            Ok(chat) => json_ok(MyResponse::success_with_data("Ok", chat)),
            Err(_) => Err(AppError::internal("创建或获取聊天失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[endpoint(tags("im_chat"))]
pub async fn get_user_chats(depot: &mut Depot) -> JsonResult<MyResponse<Vec<ChatWithName>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        // 使用 open_id 查询聊天记录（与创建聊天记录时保持一致）
        // 使用包含名称信息的方法
        match im_chat_service::get_user_chats_with_names(&from_user.open_id).await {
            Ok(chats) => json_ok(MyResponse::success_with_data("Ok", chats)),
            Err(_) => Err(AppError::internal("获取聊天列表失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[endpoint(tags("im_chat"))]
pub async fn update_chat(
    depot: &mut Depot,
    chat_id: PathParam<String>,
    req: JsonBody<UpdateChatRequest>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(_from_user) = depot.obtain::<User>() {
        let chat_id = chat_id.into_inner();
        let req = req.into_inner();

        if let Some(is_top) = req.is_top {
            match im_chat_service::set_chat_top(&chat_id, is_top).await {
                Ok(_) => json_ok(MyResponse::success_with_msg("置顶状态更新成功")),
                Err(_) => Err(AppError::internal("更新置顶状态失败")),
            }
        } else if let Some(is_mute) = req.is_mute {
            match im_chat_service::set_chat_mute(&chat_id, is_mute).await {
                Ok(_) => json_ok(MyResponse::success_with_msg("免打扰状态更新成功")),
                Err(_) => Err(AppError::internal("更新免打扰状态失败")),
            }
        } else {
            json_ok(MyResponse::success_with_msg("无更新操作"))
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[endpoint(tags("im_chat"))]
pub async fn delete_chat(
    depot: &mut Depot,
    chat_id: PathParam<String>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let chat_id = chat_id.into_inner();

        // 使用 open_id 删除聊天记录（与创建聊天记录时保持一致）
        match im_chat_service::delete_chat(&chat_id, &from_user.open_id).await {
            Ok(_) => json_ok(MyResponse::success_with_msg("聊天删除成功")),
            Err(_) => Err(AppError::internal("删除聊天失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 获取未读消息统计
#[endpoint(tags("im_chat"))]
pub async fn get_unread_stats(depot: &mut Depot) -> JsonResult<MyResponse<serde_json::Value>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        // 使用 open_id 查询未读消息统计
        match im_chat_service::get_unread_message_stats(&from_user.open_id).await {
            Ok(stats) => json_ok(MyResponse::success_with_data("获取未读消息统计成功", stats)),
            Err(_) => Err(AppError::internal("获取未读消息统计失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateReadSequenceRequest {
    pub read_sequence: i64,
}

/// 更新已读序列号
#[endpoint(tags("im_chat"))]
pub async fn update_read_sequence(
    depot: &mut Depot,
    chat_id: PathParam<String>,
    req: JsonBody<UpdateReadSequenceRequest>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(_from_user) = depot.obtain::<User>() {
        let chat_id = chat_id.into_inner();
        let req = req.into_inner();

        match im_chat_service::update_read_sequence(&chat_id, req.read_sequence).await {
            Ok(_) => json_ok(MyResponse::success_with_msg("已读序列号更新成功")),
            Err(_) => Err(AppError::internal("更新已读序列号失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateChatRemarkRequest {
    pub remark: Option<String>,
}

/// 更新群聊备注
#[endpoint(tags("im_chat"))]
pub async fn update_chat_remark(
    depot: &mut Depot,
    chat_id: PathParam<String>,
    req: JsonBody<UpdateChatRemarkRequest>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let chat_id = chat_id.into_inner();
        let req = req.into_inner();

        info!(
            "更新群聊备注请求: chat_id={}, owner_id={}, remark={:?}",
            chat_id, from_user.open_id, req.remark
        );

        match im_chat_service::update_chat_remark(&chat_id, &from_user.open_id, req.remark.clone())
            .await
        {
            Ok(_) => {
                info!(
                    "成功更新群聊备注: chat_id={}, remark={:?}",
                    chat_id, req.remark
                );
                json_ok(MyResponse::success_with_msg("群聊备注更新成功"))
            }
            Err(_) => Err(AppError::internal("更新群聊备注失败")),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}
