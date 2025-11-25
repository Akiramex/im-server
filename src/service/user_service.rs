use crate::AppError;
use crate::dto::UserListResp;
use crate::models::User;
use crate::utils::RedisClient;
use crate::utils::snowflake::generate_snowflake_id;
use crate::{AppResult, db, utils};

static USE_REDIS: bool = true;

struct UserInsertResult {
    id: i64,
}

fn cache_key_open_id(open_id: &str) -> String {
    format!("user:open_id:{}", open_id)
}

fn cache_key_name(name: &str) -> String {
    format!("user:name:{}", name)
}

fn cache_key_email(email: &str) -> String {
    format!("user:email:{}", email)
}

async fn cache_user(user: &User) -> AppResult<()> {
    if !USE_REDIS {
        return Ok(());
    }

    let user_json =
        serde_json::to_string(user).map_err(|_| AppError::internal("User Serde_json error"))?;

    // 缓存用户信息，使用多个键
    let ttl = 3600u64; // 1小时
    if let Some(ref open_id) = user.open_id
        && let Err(e) =
            RedisClient::set_with_ttl(&cache_key_open_id(open_id), &user_json, ttl).await
    {
        return Err(AppError::internal(format!("Failed to cache user: {}", e)));
    }

    if let Err(e) = RedisClient::set_with_ttl(&cache_key_name(&user.name), &user_json, ttl).await {
        return Err(AppError::internal(format!("Failed to cache user: {}", e)));
    }

    if let Err(e) = RedisClient::set_with_ttl(&cache_key_email(&user.email), &user_json, ttl).await
    {
        return Err(AppError::internal(format!("Failed to cache user: {}", e)));
    }

    Ok(())
}

async fn get_user_from_cache(key: &str) -> Option<User> {
    if !USE_REDIS {
        return None;
    }
    match RedisClient::get(key).await {
        Ok(Some(user_json)) => match serde_json::from_str::<User>(&user_json) {
            Ok(user) => {
                tracing::debug!("从缓存获取用户信息: {}", key);
                return Some(user);
            }
            Err(e) => {
                tracing::warn!(error = ?e, "反序列化用户信息失败");
                let _ = RedisClient::del(key).await;
            }
        },
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(error = ?e, "从缓存获取用户信息失败");
        }
    }
    None
}

pub async fn create_user(
    name: String,
    email: String,
    password: String,
    phone: Option<String>,
) -> AppResult<User> {
    if get_by_name(&name).await.is_ok() {
        return Err(AppError::public("User name already exists"));
    }
    if get_by_email(&email).await.is_ok() {
        return Err(AppError::public("User email already exists"));
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

    let new_user = User::new(result.id, Some(open_id), name, email);
    cache_user(&new_user).await?;
    Ok(new_user)
}

pub async fn list_users(
    username: Option<String>,
    current_page: i64,
    page_size: i64,
) -> AppResult<UserListResp> {
    let conn = db::pool();
    let username_filter = username.clone().unwrap_or_default();
    let like_pattern = format!("%{}%", username_filter);
    let offset = (current_page - 1) * page_size;
    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!: i64" FROM users
        WHERE name LIKE $1
        "#,
        like_pattern
    )
    .fetch_one(conn)
    .await?;

    let users = sqlx::query_as!(User, r#"SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users
        WHERE name LIKE $1 AND (status IS NULL OR status = 1)
        LIMIT $2 OFFSET $3"#, like_pattern, page_size, offset)
    .fetch_all(conn)
    .await?;

    Ok(UserListResp {
        users,
        total,
        current_page,
        page_size,
    })
}

pub async fn get_by_name(name: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(name).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE name = $1 AND (status IS NULL OR status = 1)", name)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::public("User not found")),
    }
}

pub async fn get_by_email(email: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(email).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE email = $1 AND (status IS NULL OR status = 1)", email)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::public("User not found")),
    }
}

pub async fn get_by_open_id(open_id: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(open_id).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, "SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE open_id = $1 AND (status IS NULL OR status = 1)", open_id)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::public("User not found")),
    }
}

pub async fn verify_user(email_or_name: &str, password: &str) -> AppResult<User> {
    let user = if email_or_name.contains('@') {
        get_by_email(email_or_name).await?
    } else {
        get_by_name(email_or_name).await?
    };

    match &user.password_hash {
        Some(password_hash) => {
            utils::verify_password(password, password_hash)?;
            Ok(user)
        }
        None => Err(AppError::public("Invalid password")),
    }
}
