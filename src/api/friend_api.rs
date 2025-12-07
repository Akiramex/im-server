use crate::{
    models::SafeUser,
    models::User,
    prelude::*,
    service::{friend_service, user_service},
};
use salvo::{oapi::extract::PathParam, prelude::*};
use tracing::info;

/// 添加friend
#[endpoint(tags("friend"))]
pub async fn add_friend(id: PathParam<String>, depot: &mut Depot) -> JsonResult<MyResponse<()>> {
    if let Ok(current_user) = depot.obtain::<User>() {
        let friend_id = id.into_inner();
        info!(
            "添加好友请求: user_id={}, friend_id={}",
            &current_user.open_id, &friend_id
        );

        let friend_id: String = match friend_id.parse::<u64>() {
            Ok(id) => id.to_string(),
            Err(_) => {
                // 用户名查找
                let friend = user_service::get_by_name(&friend_id).await?;
                friend.open_id
            }
        };

        if friend_id == current_user.open_id {
            return Err(AppError::public("不能添加自己为好友"));
        }

        friend_service::add_friend(&current_user.open_id, &friend_id).await?;
        json_ok(MyResponse::success_with_msg("好友添加成功"))
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 删除friend
#[endpoint(tags("friend"))]
pub async fn remove_friend(id: PathParam<String>, depot: &mut Depot) -> JsonResult<MyResponse<()>> {
    if let Ok(current_user) = depot.obtain::<User>() {
        let friend_id = id.into_inner();
        info!(
            "删除好友请求: user_id={}, friend_id={}",
            &current_user.open_id, &friend_id
        );

        let friend_id: String = match friend_id.parse::<u64>() {
            Ok(id) => id.to_string(),
            Err(_) => {
                // 用户名查找
                let friend = user_service::get_by_name(&friend_id).await?;
                friend.open_id
            }
        };

        friend_service::remove_friend(&current_user.open_id, &friend_id).await?;
        json_ok(MyResponse::success_with_msg("好友删除成功"))
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 获取friends
#[endpoint(tags("friend"))]
pub async fn get_friends(depot: &mut Depot) -> JsonResult<MyResponse<Vec<SafeUser>>> {
    if let Ok(current_user) = depot.obtain::<User>() {
        let friends = friend_service::get_friends(&current_user.open_id).await?;
        return json_ok(MyResponse::success_with_data("Ok", friends));
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}
