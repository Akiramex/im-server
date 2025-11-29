use crate::utils::now_timestamp;
use time::OffsetDateTime;

use crate::AppError;
use crate::AppResult;
use crate::db;
use crate::models::SafeUser;
pub async fn add_friend(user_id: &str, friend_id: &str) -> AppResult<()> {
    let conn = db::pool();

    if is_friend(user_id, friend_id).await? {
        return Err(AppError::public("不能重复添加好友"));
    }

    // 插入好友关系，双向
    let now = now_timestamp();
    let timestamp = OffsetDateTime::now_utc();
    sqlx::query!(
        r#"
            INSERT INTO im_friendship(owner_id, to_id, remark, del_flag, black , sequence, add_source, version)
            VALUES($1, $2, NULL, 1, 1, $3, 'api' , 1)
            ON CONFLICT(owner_id, to_id) DO UPDATE SET
            del_flag = 1,
            update_time = $4,
            version = im_friendship.version + 1
        "#,
        user_id,
        friend_id,
        now,
        timestamp
    )
    .execute(conn)
    .await?;

    sqlx::query!(
        r#"
            INSERT INTO im_friendship(owner_id, to_id, remark, del_flag, black , sequence, add_source, version)
            VALUES($1, $2, NULL, 1, 1, $3,'api', 1)
            ON CONFLICT(owner_id, to_id) DO UPDATE SET
            del_flag = 1,
            update_time = $4,
            version = im_friendship.version + 1
        "#,
        friend_id,
        user_id,
        now,
        timestamp
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn remove_friend(user_id: &str, friend_id: &str) -> AppResult<()> {
    let conn = db::pool();

    // 软删除双向关系（设置 del_flag = 0）
    sqlx::query!(
        r#"UPDATE im_friendship
        SET del_flag = 0, update_time = $1, version = version + 1
        WHERE((owner_id = $2 AND to_id = $3) OR (owner_id = $3 AND to_id = $2))
        AND (del_flag IS NULL OR del_flag = 1)
        "#,
        OffsetDateTime::now_utc(),
        user_id,
        friend_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn get_friends(user_id: &str) -> AppResult<Vec<SafeUser>> {
    let conn = db::pool();

    let friends = sqlx::query_as!(SafeUser,
        r#"SELECT u.id, u.open_id, u.name, u.email, u.file_name, u.abstract as abstract_field, u.phone, u.status, u.gender
        FROM im_friendship f
        INNER JOIN users u ON f.to_id = u.open_id
        WHERE f.owner_id = $1
        AND (f.del_flag IS NULL OR f.del_flag = 1)
        AND (f.black IS NULL OR f.black = 1)
        AND (u.status IS NULL OR u.status = 1)
        ORDER BY f.update_time DESC
        "#,
        user_id
    )
    .fetch_all(conn)
    .await?;

    Ok(friends)
}

pub async fn is_friend(user_id: &str, friend_id: &str) -> AppResult<bool> {
    let conn = db::pool();

    let result = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM im_friendship
        WHERE((owner_id = $1 AND to_id = $2) OR (owner_id = $2 AND to_id = $1))
        AND (del_flag IS NULL OR del_flag = 1)
        AND (black IS NULL OR black = 1)"#,
        user_id,
        friend_id,
    )
    .fetch_one(conn)
    .await?;

    Ok(result > 0)
}
