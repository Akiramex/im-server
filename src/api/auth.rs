use std::u64;

use crate::{
    AppError, AppResponse, JsonResult,
    dto::{LoginReq, LoginResp},
    service::user_service,
    utils::{self},
};
use salvo::{http::cookie::Cookie, oapi::extract::JsonBody, prelude::*};
use tracing::error;

#[handler]
pub async fn post_login(
    login_req: JsonBody<LoginReq>,
    res: &mut Response,
    depot: &mut Depot,
) -> JsonResult<AppResponse<LoginResp>> {
    let login_req = login_req.into_inner();

    let user = user_service::verify_user(&login_req.username, &login_req.password).await?;

    let open_id = user
        .open_id
        .clone()
        .ok_or(AppError::public("open id not exist"))?;

    depot.insert("user", user.clone());

    let open_id_number = open_id.parse::<u64>().map_err(|_| {
        error!(user_id = %user.id, open_id = %open_id, "open_id 不是数字格式，无法生成 token");
        AppError::public("open id not exist")
    })?;

    let token = utils::get_token(open_id_number)?;

    let odata = LoginResp { token };

    let cookie = Cookie::build(("jwt_token", odata.token.clone()))
        .path("/")
        .http_only(true)
        .build();

    res.add_cookie(cookie);

    Ok(Json(AppResponse::success_with_data(
        "登录成功".to_owned(),
        odata,
    )))
}
