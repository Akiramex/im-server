use crate::dto::OutboxParams;
use crate::dto::UpdateOutboxStatusRequest;
use crate::{dto::CreateOutboxRequest, models::ImOutbox, prelude::*, service::im_outbox_service};
use salvo::oapi::extract::{PathParam, QueryParam};
use salvo::oapi::{endpoint, extract::JsonBody};
use salvo::prelude::*;

#[endpoint(tags("im_outbox"))]
pub async fn create_outbox(req: JsonBody<CreateOutboxRequest>) -> JsonResult<MyResponse<ImOutbox>> {
    let req = req.into_inner();

    match im_outbox_service::create(
        &req.message_id,
        &req.payload,
        &req.exchange,
        &req.routing_key,
    )
    .await
    {
        Ok(outbox) => json_ok(MyResponse::success_with_data("Ok", outbox)),
        Err(e) => Err(e),
    }
}

#[endpoint(tags("im_outbox"))]
pub async fn get_outbox(id: PathParam<i64>) -> JsonResult<MyResponse<ImOutbox>> {
    let id = id.into_inner();
    match im_outbox_service::get_by_id(id).await {
        Ok(outbox) => json_ok(MyResponse::success_with_data("Ok", outbox)),
        Err(e) => Err(e),
    }
}

#[endpoint(tags("im_outbox"))]
pub async fn update_outbox_status(
    id: PathParam<i64>,
    req: JsonBody<UpdateOutboxStatusRequest>,
) -> JsonResult<MyResponse<()>> {
    let id = id.into_inner();
    let req = req.into_inner();

    match im_outbox_service::update_status(id, &req.status).await {
        Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
        Err(e) => Err(e),
    }
}

#[endpoint(tags("im_outbox"))]
pub async fn mark_sent(id: PathParam<i64>) -> JsonResult<MyResponse<()>> {
    let id = id.into_inner();

    match im_outbox_service::mark_sent(id).await {
        Ok(_) => json_ok(MyResponse::success_with_msg("Ok")),
        Err(e) => Err(e),
    }
}

#[endpoint(tags("im_outbox"))]
pub async fn get_pending_messages(
    params: QueryParam<OutboxParams, false>,
) -> JsonResult<MyResponse<Vec<ImOutbox>>> {
    let limit = params
        .into_inner()
        .unwrap_or(OutboxParams { limit: 100 })
        .limit;

    match im_outbox_service::get_pending_messages(limit).await {
        Ok(messages) => json_ok(MyResponse::success_with_data("Ok", messages)),
        Err(e) => Err(e),
    }
}

#[endpoint(tags("im_outbox"))]
pub async fn get_failed_messages(
    params: QueryParam<OutboxParams, false>,
) -> JsonResult<MyResponse<Vec<ImOutbox>>> {
    let limit = params
        .into_inner()
        .unwrap_or(OutboxParams { limit: 100 })
        .limit;

    match im_outbox_service::get_failed_messages(limit).await {
        Ok(messages) => json_ok(MyResponse::success_with_data("Ok", messages)),
        Err(e) => Err(e),
    }
}
