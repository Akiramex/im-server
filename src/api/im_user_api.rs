use salvo::{
    oapi::extract::{JsonBody, PathParam},
    prelude::*,
};

use crate::{
    JsonResult, MyResponse, dto::LoginResp, json_ok, models::im_user::ImSafeUser,
    service::im_user_service,
};
use crate::{dto::CreateImUserReq, dto::LoginReq, models::ImUserData};

/// 创建 im_user
#[endpoint(tags("im_user"))]
pub async fn create_user(
    user_data: JsonBody<CreateImUserReq>,
) -> JsonResult<MyResponse<ImSafeUser>> {
    let user_data = user_data.into_inner();
    let user = im_user_service::create(
        user_data.user_id,
        user_data.user_name,
        user_data.password,
        user_data.mobile,
    )
    .await?;

    json_ok(MyResponse::success_with_data("创建用户成功", user))
}

/// 登录
#[endpoint(tags("im_user"))]
pub async fn login(login_req: JsonBody<LoginReq>) -> JsonResult<MyResponse<LoginResp>> {
    let req = login_req.into_inner();

    let _ = im_user_service::verify_user(&req.username, &req.password).await?;

    json_ok(MyResponse::success_with_data(
        "登录成功",
        LoginResp {
            token: "dummy_token".to_string(),
        },
    ))
}

/// 获取用户
#[endpoint(tags("im_user"))]
pub async fn get_user(user_id: PathParam<String>) -> JsonResult<MyResponse<ImSafeUser>> {
    let user_id = user_id.into_inner();

    let user = im_user_service::get_by_user_id(&user_id).await?;

    json_ok(MyResponse::success_with_data("Ok", user.into()))
}

/// 获取用户数据
#[endpoint(tags("im_user"))]
pub async fn get_user_data(user_id: PathParam<String>) -> JsonResult<MyResponse<ImUserData>> {
    let user_id = user_id.into_inner();

    let user_data = im_user_service::get_user_data(&user_id).await?;

    json_ok(MyResponse::success_with_data("Ok", user_data))
}

/// 更新用户数据
#[endpoint(tags("im_user"))]
pub async fn upsert_user_data(user_data: JsonBody<ImUserData>) -> JsonResult<MyResponse<()>> {
    let user_data = user_data.into_inner();

    im_user_service::upsert_user_data(user_data).await?;

    json_ok(MyResponse::success_with_msg("Ok"))
}
