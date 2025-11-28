use crate::dto::{CreateUserReq, UpdateUserReq, UserListQuery, UserListResp};
use crate::json_ok;
use crate::models::SafeUser;
use crate::service::user_service;
use crate::{AppError, MyResponse};
use crate::{JsonResult, models::User};
use salvo::oapi::extract::{JsonBody, PathParam};
use salvo::prelude::*;
use tracing::{error, info, warn};

/// 分页查询user
#[endpoint(tags("user"))]
pub async fn list_users(query: &mut Request) -> JsonResult<MyResponse<UserListResp>> {
    let query: UserListQuery = query.parse_queries()?;
    let res = user_service::list_users(query.username, query.current_page, query.page_size).await;
    match res {
        Ok(resp) => json_ok(MyResponse::success_with_data("获取用户列表成功", resp)),
        Err(err) => Err(err),
    }
}

/// 创建user
#[endpoint(tags("user"))]
pub async fn create_user(idata: JsonBody<CreateUserReq>) -> JsonResult<MyResponse<SafeUser>> {
    let idata = idata.into_inner();
    let res = user_service::create_user(idata.name, idata.email, idata.password, idata.phone).await;
    match res {
        Ok(user) => json_ok(MyResponse::success_with_data("创建用户成功", user.into())),
        Err(err) => Err(err),
    }
}

/// 更新当前user信息
#[endpoint(tags("user"))]
pub async fn update_current_user(
    update_user: JsonBody<UpdateUserReq>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<SafeUser>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let update_user = update_user.into_inner();

        let user = user_service::update_user(
            &from_user.open_id,
            update_user.name,
            update_user.file_name,
            update_user.abstract_field,
            update_user.phone,
            update_user.gender,
        )
        .await;
        match user {
            Ok(user) => json_ok(MyResponse::success_with_data(
                "更新用户信息成功",
                user.into(),
            )),
            Err(err) => Err(err),
        }
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 获取user
#[endpoint(tags("user"))]
pub async fn get_user(
    id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<SafeUser>> {
    let id = id.into_inner();
    if let Ok(from_user) = depot.obtain::<User>() {
        info!(
            "查询用户，open_id 或用户名: {} (请求来自用户ID: {})",
            id, from_user.name
        );
    }
    if let Ok(numeric_id) = id.parse::<i64>() {
        let open_id = numeric_id.to_string();
        match user_service::get_by_open_id(&open_id).await {
            Ok(user) => {
                info!(
                    id = user.id,
                    open_id = user.open_id,
                    name = user.name,
                    "通过 open_id 找到用户"
                );
                return json_ok(MyResponse::success_with_data(
                    "open_id获取用户成功",
                    user.into(),
                ));
            }
            Err(AppError::NotFound(_)) => {
                warn!(
                    "通过 open_id 未找到用户: {}，继续尝试通过用户名查询",
                    numeric_id
                );
            }
            Err(err) => {
                error!("通过 open_id 查询用户失败: {:?}", err);
                return Err(err);
            }
        }
    }

    match user_service::get_by_name(&id).await {
        Ok(user) => {
            info!(
                id = user.id,
                open_id = user.open_id,
                name = user.name,
                "通过 name 找到用户"
            );
            json_ok(MyResponse::success_with_data(
                "name获取用户成功",
                user.into(),
            ))
        }
        Err(err) => Err(err),
    }
}
