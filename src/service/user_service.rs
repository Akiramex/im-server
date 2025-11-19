use crate::dto::CreateUserReq;
use crate::models::User;
use crate::utils::hash_password;
use crate::utils::snowflake::generate_snowflake_id;
use crate::{AppResult, JsonResult, db, utils};
use salvo::writing::Json;
struct UserInsertResult {
    id: i64,
}

pub async fn create_user(
    name: String,
    email: String,
    password: String,
    phone: Option<String>,
) -> JsonResult<User> {
    if get_by_name(&name).await.is_ok() {
        return Err(AppError::Public("用户名已被占用".to_owned()));
    }

    let open_id = generate_snowflake_id().to_string();
    let password_hash = utils::hash_password(&password)?;

    let result = sqlx::query_as!(
        UserInsertResult,
        "INSERT INTO users (open_id, name, email, password_hash, phone, status, gender)
        VALUES ($1, $2, $3, $4, $5, 1, 3)
        RETURNING id",
        open_id,
        name,
        email,
        password_hash,
        phone
    )
    .fetch_one(db::pool())
    .await?;

    // 放到缓存
    let new_user = User::new(result.id, Some(open_id), name, email);
    Ok(Json(new_user))
}

pub async fn get_by_name(name: &str) -> AppResult<User> {
    // 从缓存里面拿 todo

    let conn = db::pool();

    let user = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE name = $1 AND (status IS NULL OR status = 1)", name)
        .fetch_one(conn)
        .await?;

    Ok(user)
}

pub async fn get_by_email(email: &str) -> AppResult<User> {
    // 从缓存里面拿 todo

    let conn = db::pool();

    let user = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE email = $1 AND (status IS NULL OR status = 1)", email)
        .fetch_one(conn)
        .await?;

    Ok(user)
}

pub async fn verify_user(email: &str, password: &str) -> AppResult<User> {
    let user = get_by_email(email).await?;

    Ok(user)
}
