use crate::{db, models::User, prelude::*};
use salvo::{oapi::extract::QueryParam, prelude::*};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[endpoint(tags("message"))]
pub async fn send_message() -> JsonResult<MyResponse<()>> {
    todo!()
}

#[derive(Deserialize, Clone, Debug, ToSchema)]
struct SinceTimestamp(i64);

/// 获取离线消息
#[endpoint(tags("message"))]
pub async fn get_messages(
    depot: &mut Depot,
    params: QueryParam<SinceTimestamp, false>,
) -> JsonResult<MyResponse<Vec<GetMessageResult>>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let conn = db::pool();
        let since_timestamp = params.into_inner().unwrap_or(SinceTimestamp(0)).0;
        let time = OffsetDateTime::from_unix_timestamp(since_timestamp / 1000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        let messages = match sqlx::query_as!(
            ImSingleMessageRow,
            r#"SELECT message_id, from_id, to_id, message_body, message_time,
                    message_content_type, read_status, extra, del_flag, sequence,
                    message_random, create_time, update_time, version, reply_to,
                    to_type, file_url, file_name, file_type
             FROM im_single_message
             WHERE to_id = $1 AND message_time > $2 AND del_flag = 1 AND message_content_type != 4
             ORDER BY message_time ASC
             LIMIT 100"#,
            &from_user.open_id,
            time
        )
        .fetch_all(conn)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "查询离线消息失败");
                return Err(AppError::internal("查询消息失败"));
            }
        };

        let mut result = vec![];
        for row in messages {
            result.push(GetMessageResult {
                message_id: row.message_id,
                from_user_id: row.from_id,
                to_user_id: row.to_id,
                message: row.message_body,
                message_time: row.message_time,
                file_url: row.file_url,
                file_name: row.file_name,
                file_type: row.file_type,
            });
        }

        json_ok(MyResponse::success_with_data("Ok", result))
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ImSingleMessageRow {
    message_id: String,
    from_id: String,
    to_id: String,
    message_body: String,
    message_time: OffsetDateTime,
    message_content_type: i32,
    read_status: i32,
    extra: Option<String>,
    del_flag: i16,
    sequence: i64,
    message_random: Option<String>,
    create_time: Option<OffsetDateTime>,
    update_time: Option<OffsetDateTime>,
    version: Option<i64>,
    reply_to: Option<String>,
    to_type: Option<String>,
    file_url: Option<String>,
    file_name: Option<String>,
    file_type: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct GetMessageResult {
    message_id: String,
    from_user_id: String,
    to_user_id: String,
    message: String,
    message_time: OffsetDateTime,
    file_url: Option<String>,
    file_name: Option<String>,
    file_type: Option<String>,
}
