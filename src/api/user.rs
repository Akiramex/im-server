use crate::MyResponse;
use crate::dto::{CreateUserReq, UserListQuery, UserListResp};
use crate::json_ok;
use crate::service::user_service;
use crate::{JsonResult, models::User};
use salvo::oapi::extract::{JsonBody, PathParam};
use salvo::prelude::*;
use tracing::info;

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
pub async fn create_user(idata: JsonBody<CreateUserReq>) -> JsonResult<MyResponse<User>> {
    let idata = idata.into_inner();
    let res = user_service::create_user(idata.name, idata.email, idata.password, idata.phone).await;
    match res {
        Ok(user) => json_ok(MyResponse::success_with_data("创建用户成功", user)),
        Err(err) => Err(err),
    }
}

/// 获取user
#[endpoint(tags("user"))]
pub async fn get_user(id: PathParam<String>, depot: &mut Depot) -> JsonResult<MyResponse<User>> {
    if let Ok(user) = depot.obtain::<User>() {
        info!(
            "查询用户，open_id或用户名: {} (请求来自用户ID: {})",
            id, user.name
        );
    }
    let res = user_service::get_user(id).await;
    match res {
        Ok(user) => json_ok(MyResponse::success_with_data("获取用户成功", user)),
        Err(err) => Err(err),
    }
}
