use crate::db;
use crate::dto::SubscriptionInfoResp;
use crate::prelude::*;
use crate::service::user_service;
use im_share::subscription::SubscriptionService;
use salvo::oapi::extract::PathParam;
use salvo::prelude::*;
use std::sync::Arc;

/// 根据订阅 ID 获取用户 ID（返回 open_id 的数字形式用于MQTT）
#[endpoint(tags("subscription"))]
pub async fn get_user_id_by_subscription(
    subscription_id: PathParam<String>,
    depot: &mut Depot,
) -> JsonResult<MyResponse<SubscriptionInfoResp>> {
    info!(subscription_id = %subscription_id, "查询订阅 ID 对应的用户");
    let conn = db::pool();
    let subscription_id = subscription_id.into_inner();

    let subscription_service = depot
        .obtain::<Arc<SubscriptionService>>()
        .map_err(|_| AppError::internal("SubscriptionService not found"))?;

    let user_db_id: i64 = match sqlx::query_scalar!(
        "SELECT user_id FROM subscriptions WHERE subscription_id = $1",
        subscription_id
    )
    .fetch_optional(conn)
    .await
    {
        Ok(Some(id)) => {
            info!(subscription_id = %subscription_id, user_id = %id, "从数据库找到订阅 ID");
            id
        }
        Ok(None) => {
            warn!(subscription_id = %subscription_id, "数据库中未找到订阅 ID，尝试从内存查询");
            // 如果数据库中没有，尝试从内存中查询（向后兼容）
            match subscription_service.get_user_id(&subscription_id) {
                Some(id) => {
                    warn!(subscription_id = %subscription_id, user_id = %id, "从内存中找到订阅 ID（未持久化）");
                    id
                }
                None => {
                    error!(subscription_id = %subscription_id, "订阅 ID 不存在（数据库和内存中都没有）");
                    return Err(AppError::not_found("订阅 ID 不存在"));
                }
            }
        }
        Err(e) => {
            error!(subscription_id = %subscription_id, error = %e, "查询订阅 ID 失败");
            // 如果数据库查询失败，尝试从内存中查询（向后兼容）
            match subscription_service.get_user_id(&subscription_id) {
                Some(id) => {
                    warn!(subscription_id = %subscription_id, user_id = %id, "数据库查询失败，从内存中找到订阅 ID");
                    id
                }
                None => {
                    error!(subscription_id = %subscription_id, "订阅 ID 不存在（数据库查询失败且内存中也没有）");
                    return Err(AppError::not_found("订阅 ID 不存在"));
                }
            }
        }
    };
    // 根据数据库id查询用户，获取 open_id 的数字形式（用于MQTT）
    match user_service::get_by_id(user_db_id).await {
        Ok(user) => {
            let open_id = user.open_id;
            info!(subscription_id = %subscription_id, user_id = %user_db_id, open_id = %open_id, "成功获取用户信息");
            json_ok(MyResponse::success_with_data(
                "Ok",
                SubscriptionInfoResp {
                    user_id: user.id,
                    open_id,
                    subscription_id: subscription_id.to_string(),
                },
            ))
        }
        Err(e) => {
            error!(subscription_id = %subscription_id, user_id = %user_db_id, error = ?e, "查询用户信息失败");
            Err(AppError::not_found("查询用户信息失败"))
        }
    }
}
