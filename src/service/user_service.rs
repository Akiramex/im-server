use crate::JsonResult;
use crate::dto::CreateUserReq;
use salvo::oapi::extract::JsonBody;
use salvo::writing::Json;

pub async fn create_user(idata: JsonBody<CreateUserReq>) -> JsonResult<()> {
    let user = idata.into_inner();

    Ok(Json(()))
}
