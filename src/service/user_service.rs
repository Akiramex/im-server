use crate::AppError;
use crate::models::User;
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
        return Err(AppError::public("User name already exists"));
    }

    let open_id = generate_snowflake_id().to_string();
    let password_hash = utils::hash_password(&password)?;

    let conn = db::pool();
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
    .fetch_one(conn)
    .await
    .map_err(|_| AppError::internal("Failed to create user"))?;

    // 放到缓存
    let new_user = User::new(result.id, Some(open_id), name, email);
    Ok(Json(new_user))
}

pub async fn get_by_name(name: &str) -> AppResult<User> {
    // 从缓存里面拿 todo

    let conn = db::pool();

    if let Some(user) = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE name = $1 AND (status IS NULL OR status = 1)", name)
        .fetch_optional(conn)
        .await? {
            return Ok(user);
        }

    Err(AppError::public("User not found"))
}

pub async fn get_by_email(email: &str) -> AppResult<User> {
    // 从缓存里面拿 todo

    let conn = db::pool();

    if let Some(user) = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE email = $1 AND (status IS NULL OR status = 1)", email)
        .fetch_optional(conn)
        .await? {
            return Ok(user);
        }

    Err(AppError::public("User not found"))
}

pub async fn verify_user(email_or_name: &str, password: &str) -> AppResult<User> {
    let user = if email_or_name.contains('@') {
        get_by_email(email_or_name).await?
    } else {
        get_by_name(email_or_name).await?
    };

    match &user.password_hash {
        Some(password_hash) => {
            utils::verify_password(password, &password_hash)?;
            Ok(user)
        }
        None => Err(AppError::public("Invalid password")),
    }
}
