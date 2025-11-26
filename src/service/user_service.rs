use crate::AppError;
use crate::dto::UserListResp;
use crate::models::User;
use crate::utils::RedisClient;
use crate::utils::snowflake::generate_snowflake_id;
use crate::{AppResult, db, utils};

static USE_REDIS: bool = false;

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

fn cache_key_phone(phone: &str) -> String {
    format!("user:phone:{}", phone)
}

async fn cache_user(user: &User) -> AppResult<()> {
    if !USE_REDIS {
        return Ok(());
    }

    let user_json =
        serde_json::to_string(user).map_err(|_| AppError::internal("User Serde_json error"))?;

    // 缓存用户信息，使用多个键
    let ttl = 3600u64; // 1小时
    if let Err(e) =
        RedisClient::set_with_ttl(&cache_key_open_id(&user.open_id), &user_json, ttl).await
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

    if let Some(phone) = &user.phone
        && let Err(e) = RedisClient::set_with_ttl(&cache_key_phone(phone), &user_json, ttl).await
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
        r#"INSERT INTO users (open_id, name, email, password_hash, phone, status, gender)
        VALUES ($1, $2, $3, $4, $5, 1, 3)
        RETURNING id"#,
        open_id,
        name,
        email,
        password_hash,
        phone
    )
    .fetch_one(conn)
    .await
    .map_err(|_| AppError::internal("Failed to create user"))?;

    let new_user = User::new(result.id, open_id, name, email);
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
    if let Some(user) = get_user_from_cache(&cache_key_name(name)).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, r#"SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE name = $1 AND (status IS NULL OR status = 1)"#, name)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::not_found("User")),
    }
}

pub async fn get_by_email(email: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(&cache_key_email(email)).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, r#"SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE email = $1 AND (status IS NULL OR status = 1)"#, email)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::not_found("User")),
    }
}

pub async fn get_by_phone(phone: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(&cache_key_phone(phone)).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, r#"SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE phone = $1 AND (status IS NULL OR status = 1)"#, phone)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::not_found("User")),
    }
}

pub async fn get_by_open_id(open_id: &str) -> AppResult<User> {
    if let Some(user) = get_user_from_cache(open_id).await {
        return Ok(user);
    }

    let conn = db::pool();

    let user = sqlx::query_as!(User, r#"SELECT id, open_id, name, email, password_hash, file_name, abstract as abstract_field, phone, status, gender
        FROM users WHERE open_id = $1 AND (status IS NULL OR status = 1)"#, open_id)
        .fetch_optional(conn)
        .await?;

    match user {
        Some(user) => {
            let _ = cache_user(&user).await;
            Ok(user)
        }
        None => Err(AppError::not_found("User")),
    }
}

pub async fn update_user(
    open_id: &str,
    name: Option<String>,
    file_name: Option<String>,
    abstract_field: Option<String>,
    phone: Option<String>,
    gender: Option<i32>,
) -> AppResult<User> {
    let conn = db::pool();

    let mut tx = conn.begin().await?;

    if let Some(n) = name {
        // todo check input
        match get_by_name(&n).await {
            Ok(user) => {
                if user.open_id != open_id {
                    return Err(AppError::public("昵称已被使用"));
                }
            }
            Err(AppError::NotFound(_)) => {
                // 昵称未被占用，可以更新
            }
            Err(e) => return Err(e),
        }

        sqlx::query!(
            r#"UPDATE users SET name = $1 WHERE open_id = $2"#,
            n,
            open_id
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(p) = &phone {
        // todo check input
        match get_by_phone(p).await {
            Ok(user) => {
                if user.open_id != open_id {
                    return Err(AppError::public("手机号已被使用"));
                }
            }
            Err(AppError::NotFound(_)) => {
                // 手机号未被占用，可以更新
            }
            Err(e) => return Err(e),
        }

        if p.is_empty() {
            sqlx::query!(
                r#"UPDATE users SET phone = NULL WHERE open_id = $1"#,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                r#"UPDATE users SET phone = $1 WHERE open_id = $2"#,
                p,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(f) = file_name {
        // 如果 file_name 是空字符串，设置为 NULL
        if f.is_empty() {
            sqlx::query!(
                r#"UPDATE users SET file_name = NULL WHERE open_id = $1"#,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                r#"UPDATE users SET file_name = $1 WHERE open_id = $2"#,
                f,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(a) = abstract_field {
        // 如果 abstract_field 是空字符串，设置为 NULL
        if a.is_empty() {
            sqlx::query!(
                r#"UPDATE users SET abstract = NULL WHERE open_id = $1"#,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                r#"UPDATE users SET abstract = $1 WHERE open_id = $2"#,
                a,
                open_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(g) = gender {
        sqlx::query!(
            r#"UPDATE users SET gender = $1 WHERE open_id = $2"#,
            g,
            open_id
        )
        .execute(&mut *tx)
        .await?;
    }

    let before_user = get_by_open_id(open_id).await?;

    // 执行更新
    tx.commit().await?;

    // 清除缓存
    invaliddate_user_cache(&before_user).await;

    let updated_user = get_by_open_id(open_id).await?;
    Ok(updated_user)
}

async fn invaliddate_user_cache(user: &User) {
    // 清除缓存逻辑
    if !USE_REDIS {
        return;
    }

    let mut keys = vec![
        cache_key_open_id(&user.open_id),
        cache_key_open_id(&user.name),
        cache_key_open_id(&user.email),
    ];

    if let Some(ref phone) = user.phone {
        keys.push(cache_key_phone(phone));
    }

    let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    if let Err(e) = RedisClient::del_many(&key_refs).await {
        tracing::warn!(error = ?e, "清除用户缓存失败");
    } else {
        tracing::debug!("清除用户缓存: {:?}", keys);
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
        None => Err(AppError::public("Invalid Password")),
    }
}
