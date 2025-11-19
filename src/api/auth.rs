use crate::dto::LoginReq;
use salvo::{oapi::extract::JsonBody, prelude::*};

#[handler]
pub async fn post_login(login_req: JsonBody<LoginReq>) {
    let login_req = login_req.into_inner();
    // TODO: Implement login logic
}
