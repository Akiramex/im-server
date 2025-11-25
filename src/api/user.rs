use crate::MyResponse;
use crate::dto::CreateUserReq;
use crate::json_ok;
use crate::service::user_service;
use crate::{JsonResult, models::User};
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;

#[endpoint]
pub async fn list_users() -> String {
    "Hello, World!".to_string()
}

#[endpoint]
pub async fn create_user(idata: JsonBody<CreateUserReq>) -> JsonResult<MyResponse<User>> {
    let idata = idata.into_inner();

    let res = user_service::create_user(idata.name, idata.email, idata.password, idata.phone).await;

    match res {
        Ok(user) => json_ok(MyResponse::success_with_data("创建用户成功", user)),
        Err(err) => Err(err.into()),
    }
}
