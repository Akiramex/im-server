use salvo::oapi::extract::{JsonBody, PathParam, QueryParam};
use salvo::prelude::*;
use time::OffsetDateTime;

use crate::dto::{
    AddFriendRequest, GetFriendsResp, GetFriendshipRequests, GetFriendshipResp,
    HandleFriendshipRequests, SimpleFriendshipResp, UpdateRemarkReq,
};
use crate::models::User;
use crate::models::im_friendship::ImFriendshipRequest;
use crate::service::{im_friendship_service, user_service};
use crate::{prelude::*, utils};

/// 根据open_id获取好友列表
///
/// 不需要登录
#[endpoint(tags("im_friendship"))]
pub async fn get_friends_by_open_id(
    open_id: PathParam<String>,
) -> JsonResult<MyResponse<Vec<SimpleFriendshipResp>>> {
    let open_id = open_id.into_inner();

    match im_friendship_service::get_friends(&open_id).await {
        Ok(friends) => {
            let friends: Vec<SimpleFriendshipResp> =
                friends.into_iter().map(|f| f.into()).collect();
            json_ok(MyResponse::success_with_data("Ok", friends))
        }
        Err(e) => Err(AppError::internal(format!("获取好友列表失败: {}", e))),
    }
}

/// 获取好友列表
#[endpoint(tags("im_friendship"))]
pub async fn get_friends(depot: &mut Depot) -> JsonResult<MyResponse<Vec<GetFriendsResp>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let owner_id = from_user.open_id.clone();
        info!(
            "获取好友列表: owner_id={}, name={}, open_id={:?}",
            owner_id, from_user.name, from_user.open_id
        );

        match im_friendship_service::get_friends(&owner_id).await {
            Ok(friends) => {
                // 为每个好友查询用户信息
                let mut friends_with_info = Vec::new();
                for friend in friends {
                    // 根据 to_id（可能是用户名、手机号或 open_id）查询用户信息
                    let friend_user = match user_service::get_by_name(&friend.to_id).await {
                        Ok(user) => Some(user),
                        Err(_) => match user_service::get_by_phone(&friend.to_id).await {
                            Ok(user) => Some(user),
                            Err(_) => user_service::get_by_open_id(&friend.to_id).await.ok(),
                        },
                    };

                    let request_info = GetFriendsResp {
                        friendship: friend,
                        user: friend_user.map(|user| user.into()),
                    };
                    friends_with_info.push(request_info);
                }
                json_ok(MyResponse::success_with_data("Ok", friends_with_info))
            }
            Err(e) => Err(AppError::internal(format!("获取好友列表失败: {}", e))),
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 添加好友
#[endpoint(tags("im_friendship"))]
pub async fn add_friend(
    depot: &mut Depot,
    req: JsonBody<AddFriendRequest>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let req = req.into_inner();

        let from_id = from_user.open_id.clone();

        // 验证 to_id 必须是用户名、手机号或 open_id，并查找对应的用户
        let to_user = match user_service::get_by_name(&req.to_id).await {
            Ok(user) => Ok(user),
            Err(_) => match user_service::get_by_phone(&req.to_id).await {
                Ok(user) => Ok(user),
                Err(_) => user_service::get_by_open_id(&req.to_id).await,
            },
        };

        let to_user = to_user?;
        let to_id = to_user.open_id.clone();

        // 检查是否已经是好友
        info!(
            "检查好友关系: from_id={}, to_id={}, from_user.name={}, from_user.open_id={:?}, to_user.name={}, to_user.open_id={:?}",
            from_id, to_id, from_user.name, from_user.open_id, to_user.name, to_user.open_id
        );
        if let Ok(is_friend) = im_friendship_service::is_friend(&from_id, &to_id).await {
            if is_friend {
                warn!("已经是好友: from_id={}, to_id={}", from_id, to_id);
                return Err(AppError::public("已经是好友"));
            }
        }

        // 检查是否已经有待处理的好友请求（双向检查）
        // 注意：只检查待处理的请求（approve_status = 0），已拒绝的请求（approve_status = 2）可以重新发送
        // 1. 检查是否已经向对方发送过待处理的请求（from_id -> to_id）
        let existing_requests_to =
            im_friendship_service::get_friendship_requests(&to_id, Some(0)).await;
        if let Ok(requests) = existing_requests_to {
            // 只检查待处理的请求（approve_status = 0）
            if requests
                .iter()
                .any(|r| r.from_id == from_id && r.approve_status == Some(0))
            {
                return Err(AppError::public("已经发送过好友请求，等待对方处理"));
            }
        }

        // 2. 检查对方是否已经向自己发送过待处理的请求（to_id -> from_id），如果是，应该提示用户直接同意
        let existing_requests_from =
            im_friendship_service::get_friendship_requests(&from_id, Some(0)).await;
        if let Ok(requests) = existing_requests_from {
            // 只检查待处理的请求（approve_status = 0）
            if requests
                .iter()
                .any(|r| r.from_id == to_id && r.approve_status == Some(0))
            {
                return Err(AppError::public(
                    "对方已经向您发送过好友请求，请先处理对方的请求",
                ));
            }
        }

        // 3. 如果之前有被拒绝的请求，允许重新发送（删除旧请求或创建新请求）
        // 检查是否有被拒绝的请求（approve_status = 2）
        let _rejected_requests_to =
            im_friendship_service::get_friendship_requests(&to_id, Some(2)).await;
        // 如果有被拒绝的请求，可以重新发送（创建新请求会覆盖旧请求）
        // 这里不做任何处理，允许继续创建新请求

        use ulid::Ulid;
        let request_id = Ulid::new().to_string();
        // 创建好友请求（先克隆需要后续使用的字段）
        let from_id_clone = from_id.clone();
        let to_id_clone = to_id.clone();
        let remark_clone = req.remark.clone();
        let add_source_clone = req.add_source.clone();
        let message_clone = req.message.clone();

        let not = utils::now_timestamp();
        let timestamp = OffsetDateTime::now_utc();
        let friendship_request = ImFriendshipRequest {
            id: request_id.clone(),
            from_id,
            to_id,
            remark: req.remark,
            read_status: Some(0),
            add_source: req.add_source,
            message: req.message,
            approve_status: Some(0), // 0: 待处理
            create_time: Some(timestamp),
            update_time: Some(timestamp),
            sequence: Some(not),
            del_flag: Some(1),
            version: Some(1),
        };

        // 插入好友请求
        match im_friendship_service::create_friendship_request(friendship_request).await {
            Ok(_) => {
                info!(
                    "创建好友请求成功: request_id={}, from_id={}, to_id={}",
                    request_id, from_id_clone, to_id_clone
                );

                // 通过 MQTT 推送好友请求通知给接收者
                // to_id_clone 已经是用户名或手机号，直接查找
                let to_user = match user_service::get_by_name(&to_id_clone).await {
                    Ok(user) => Ok(user),
                    Err(_) => user_service::get_by_phone(&to_id_clone).await,
                };

                if let Ok(to_user) = to_user {
                    error!("嘟嘟嘟 --- MQTT功能待完成");
                    //todo!()
                } else {
                    warn!(to_id = %to_id_clone, "无法获取接收者信息，无法通过MQTT发送好友请求通知");
                    // 无法获取用户信息时，无法通过 MQTT 发布，但好友请求已保存到数据库
                    // 用户可以通过查询好友请求列表来获取
                }

                json_ok(MyResponse::success_with_msg(format!(
                    "好友请求已发送，等待对方同意: request_id={}",
                    request_id
                )))
            }
            Err(err) => Err(err),
        }
    } else {
        Err(AppError::unauthorized("未登录"))
    }
}

/// 删除好友
#[endpoint(tags("im_friendship"))]
pub async fn remove_friend(
    depot: &mut Depot,
    to_id: PathParam<String>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let to_id = to_id.into_inner();

        let to_user = match user_service::get_by_name(&to_id).await {
            Ok(user) => Ok(user),
            Err(_) => match user_service::get_by_phone(&to_id).await {
                Ok(user) => Ok(user),
                Err(_) => user_service::get_by_open_id(&to_id).await,
            },
        };

        let to_user = match to_user {
            Ok(user) => user,
            Err(_) => {
                warn!("无法找到要删除的好友: to_id={}", to_id);
                return Err(AppError::not_found(
                    "未找到该用户，请确认用户ID或用户名是否正确",
                ));
            }
        };

        let to_id_open_id = to_user.open_id;
        let owner_id = from_user.open_id.clone();
        info!(
            "删除好友: owner_id={}, to_id={}, to_id_open_id={}",
            owner_id, to_id, to_id_open_id
        );

        // 使用转换后的 open_id 删除好友
        match im_friendship_service::remove_friend(&owner_id, &to_id_open_id).await {
            Ok(_) => {
                info!(
                    "成功删除好友: owner_id={}, to_id={}",
                    owner_id, to_id_open_id
                );
                json_ok(MyResponse::success_with_msg("Ok"))
            }
            Err(e) => {
                warn!(
                    "删除好友失败: owner_id={}, to_id={}, error={:?}",
                    owner_id, to_id_open_id, e
                );
                Err(AppError::internal("删除好友失败"))
            }
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 更新备注
#[endpoint(tags("im_friendship"))]
pub async fn update_remark(
    depot: &mut Depot,
    to_id: PathParam<String>,
    req: JsonBody<UpdateRemarkReq>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let to_id = to_id.into_inner();
        let req = req.into_inner();
        match im_friendship_service::update_remark(&from_user.open_id, &to_id, req.remark).await {
            Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
            Err(err) => Err(err),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 切换拉黑状态
#[endpoint(tags("im_friendship"))]
pub async fn black_friend(
    depot: &mut Depot,
    to_id: PathParam<String>,
) -> JsonResult<MyResponse<()>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let to_id = to_id.into_inner();
        match im_friendship_service::black_friend(&from_user.open_id, &to_id).await {
            Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
            Err(err) => Err(err),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 创建好友申请
#[endpoint(tags("im_friendship"))]
pub async fn create_friendship_request(
    req: JsonBody<ImFriendshipRequest>,
) -> JsonResult<MyResponse<()>> {
    let req = req.into_inner();

    match im_friendship_service::create_friendship_request(req).await {
        Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
        Err(err) => Err(err),
    }
}

/// 获取好友申请
#[endpoint(tags("im_friendship"))]
pub async fn get_friendship_requests(
    query: QueryParam<GetFriendshipRequests, false>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<Vec<GetFriendshipResp>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let approve_status = match query.into_inner() {
            Some(approve_status) => Some(approve_status.approve_status),
            None => None,
        };
        match im_friendship_service::get_friendship_requests(&from_user.open_id, approve_status)
            .await
        {
            Ok(requests) => {
                let mut requests_with_info = Vec::new();
                for request in requests {
                    // 根据 from_id 查询用户信息
                    // 注意：from_id 在数据库中通常存储的是 open_id
                    // 所以优先使用 get_by_open_id 查询，如果失败再尝试其他方式
                    let user_info = match user_service::get_by_open_id(&request.from_id).await {
                        Ok(user) => {
                            info!(
                                from_id = %request.from_id,
                                user_name = %user.name,
                                "通过 open_id 查询到用户信息"
                            );
                            Some(user)
                        }
                        Err(_) => {
                            // 如果 open_id 查询失败，尝试作为用户名查询
                            match user_service::get_by_name(&request.from_id).await {
                                Ok(user) => {
                                    info!(
                                        from_id = %request.from_id,
                                        user_name = %user.name,
                                        "通过用户名查询到用户信息"
                                    );
                                    Some(user)
                                }
                                Err(_) => {
                                    // 最后尝试作为手机号查询
                                    match user_service::get_by_phone(&request.from_id).await {
                                        Ok(user) => {
                                            info!(
                                                from_id = %request.from_id,
                                                user_name = %user.name,
                                                "通过手机号查询到用户信息"
                                            );
                                            Some(user)
                                        }
                                        Err(e) => {
                                            warn!(
                                                from_id = %request.from_id,
                                                error = ?e,
                                                "无法查询到用户信息（尝试了 open_id、用户名、手机号）"
                                            );
                                            None
                                        }
                                    }
                                }
                            }
                        }
                    };

                    let request_info = GetFriendshipResp {
                        friendship_req: request,
                        user: user_info.map(|user| user.into()),
                    };
                    requests_with_info.push(request_info);
                }
                json_ok(MyResponse::success_with_data("Ok", requests_with_info))
            }
            Err(e) => Err(AppError::internal(format!("获取好友请求失败: {}", e))),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 处理好友申请
///
/// 1：同意 2：拒绝
#[endpoint(tags("im_friendship"))]
pub async fn handle_friendship_request(
    request_id: PathParam<String>,
    req: JsonBody<HandleFriendshipRequests>,
) -> JsonResult<MyResponse<()>> {
    let request_id = request_id.into_inner();
    let req = req.into_inner();

    let _ =
        im_friendship_service::handle_friendship_request(&request_id, req.approve_status).await?;

    json_ok(MyResponse::success_with_msg("Ok"))
}
