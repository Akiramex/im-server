use crate::{
    MyResponse, config,
    dto::{CreateUserReq, LoginReq, LoginResp},
    json_ok,
    models::SafeUser,
    prelude::*,
    service::{auth_service, user_service},
};
use salvo::{http::cookie::Cookie, oapi::extract::JsonBody, prelude::*};
use tracing::error;

/// 登录
#[endpoint(tags("auth"))]
pub async fn post_login(
    login_req: JsonBody<LoginReq>,
    res: &mut Response,
    depot: &mut Depot,
) -> JsonResult<MyResponse<LoginResp>> {
    let login_req = login_req.into_inner();

    let user = user_service::verify_user(&login_req.username, &login_req.password).await?;

    let open_id = user.open_id.clone();

    depot.inject(user.clone());

    let open_id_number = open_id.parse::<u64>().map_err(|_| {
        error!(user_id = %user.id, open_id = %open_id, "open_id 不是数字格式，无法生成 token");
        AppError::public("open id not exist")
    })?;

    let token = auth_service::get_token(open_id_number, &config::get().jwt)?;

    let odata = LoginResp { token };

    let cookie = Cookie::build(("jwt_token", odata.token.clone()))
        .path("/")
        .http_only(true)
        .build();

    res.add_cookie(cookie);
    json_ok(MyResponse::success_with_data("登录成功", odata))
}

/// 注册
#[endpoint(tags("auth"))]
pub async fn register(idata: JsonBody<CreateUserReq>) -> JsonResult<MyResponse<SafeUser>> {
    let idata = idata.into_inner();
    let res = user_service::create_user(idata.name, idata.email, idata.password, idata.phone).await;
    match res {
        Ok(user) => json_ok(MyResponse::success_with_data("创建用户成功", user.into())),
        Err(err) => Err(err),
    }
}
