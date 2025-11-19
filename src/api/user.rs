use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;

use crate::AppError;
use crate::dto::CreateUserReq;
use crate::service::user_service;
use crate::{JsonResult, models::User};
#[handler]
pub async fn list_users() -> String {
    "Hello, World!".to_string()
}

#[handler]
pub async fn create_user(idata: JsonBody<CreateUserReq>) -> JsonResult<User> {
    let idata = idata.into_inner();
    // 字段进行检查
    user_service::create_user(idata.name, idata.email, idata.password, idata.phone).await
}
