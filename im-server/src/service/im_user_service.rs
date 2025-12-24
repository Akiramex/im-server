use time::OffsetDateTime;
use time::serde::timestamp;

use crate::db;
use crate::models::{ImSafeUser, ImUser, ImUserData};
use crate::prelude::*;
use crate::utils::{self, RedisClient};
static USE_REDIS: bool = true;

fn cache_key_user_id(user_id: &str) -> String {
    format!("im_user:user_id:{}", user_id)
}

fn cache_key_user_name(user_name: &str) -> String {
    format!("im_user:user_name:{}", user_name)
}

fn cache_key_mobile(mobile: &str) -> String {
    format!("im_user:mobile:{}", mobile)
}

fn cache_key_user_data(user_id: &str) -> String {
    format!("im_user_data:{}", user_id)
}

async fn cache_im_user(user: &ImUser) -> AppResult<()> {
    if USE_REDIS {
        let user_json =
            serde_json::to_string(user).map_err(|_| AppError::internal("ImUser serde error"))?;

        let ttl = 3600u64; // 1小时

        // 通过 user_id 缓存
        if let Err(e) =
            RedisClient::set_with_ttl(&cache_key_user_id(&user.user_id), &user_json, ttl).await
        {
            tracing::warn!(error = ?e, "缓存 ImUser 信息失败 (user_id)");
        }

        // 通过 user_name 缓存
        if let Some(ref user_name) = user.user_name
            && let Err(e) =
                RedisClient::set_with_ttl(&cache_key_user_name(user_name), &user_json, ttl).await
        {
            tracing::warn!(error = ?e, "缓存 ImUser 信息失败 (user_name)");
        }

        // 通过 mobile 缓存（如果有）
        if let Some(ref mobile) = user.mobile
            && let Err(e) =
                RedisClient::set_with_ttl(&cache_key_mobile(mobile), &user_json, ttl).await
        {
            tracing::warn!(error = ?e, "缓存 ImUser 信息失败 (mobile)");
        }
    }
    Ok(())
}

async fn get_im_user_from_cache(key: &str) -> AppResult<Option<ImUser>> {
    if USE_REDIS {
        match RedisClient::get(key).await {
            Ok(Some(user_json)) => match serde_json::from_str::<ImUser>(&user_json) {
                Ok(user) => {
                    tracing::debug!("从缓存获取 ImUser 信息: {}", key);
                    return Ok(Some(user));
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "反序列化 ImUser 信息失败");
                    let _ = RedisClient::del(key).await;
                }
            },
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = ?e, "从缓存获取 ImUser 信息失败");
            }
        }
    }
    Ok(None)
}

async fn invalidate_im_user_cache(user: &ImUser) {
    if USE_REDIS {
        let mut keys = vec![cache_key_user_id(&user.user_id)];

        if let Some(ref user_name) = user.user_name {
            keys.push(cache_key_user_name(user_name));
        }

        if let Some(ref mobile) = user.mobile {
            keys.push(cache_key_mobile(mobile));
        }

        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        if let Err(e) = RedisClient::del_many(&key_refs).await {
            tracing::warn!(error = ?e, "清除 ImUser 缓存失败");
        }
    }
}

/// 根据user_id获取用户（user_id 对应 users.open_id）
pub async fn get_by_user_id(user_id: &str) -> AppResult<ImUser> {
    let conn = db::pool();
    // 先从缓存获取
    if let Some(user) = get_im_user_from_cache(&cache_key_user_id(user_id)).await? {
        return Ok(user);
    }

    let user = sqlx::query_as!(
        ImUser,
        r#"SELECT open_id as user_id, name as user_name, password_hash as password,
                phone as mobile, create_time, update_time, version, del_flag
         FROM users
         WHERE open_id = $1 AND (del_flag IS NULL OR del_flag = 1)"#,
        user_id
    )
    .fetch_optional(conn)
    .await?;

    match user {
        Some(u) => {
            // 缓存用户信息
            let _ = cache_im_user(&u).await;
            Ok(u)
        }
        None => Err(AppError::not_found(user_id)),
    }
}

/// 根据用户名获取用户
pub async fn get_by_user_name(user_name: &str) -> AppResult<ImUser> {
    let conn = db::pool();
    // 先从缓存获取
    if let Some(user) = get_im_user_from_cache(&cache_key_user_name(user_name)).await? {
        return Ok(user);
    }

    let user = sqlx::query_as!(
        ImUser,
        r#"SELECT open_id as user_id, name as user_name, password_hash as password,
                phone as mobile, create_time, update_time, version, del_flag
         FROM users
         WHERE name = $1 AND (del_flag IS NULL OR del_flag = 1)"#,
        user_name
    )
    .fetch_optional(conn)
    .await?;

    match user {
        Some(u) => {
            // 缓存用户信息
            let _ = cache_im_user(&u).await;
            Ok(u)
        }
        None => Err(AppError::not_found(user_name)),
    }
}

/// 根据手机号获取用户
#[allow(dead_code)]
pub async fn get_by_mobile(mobile: &str) -> AppResult<ImUser> {
    let conn = db::pool();
    // 先从缓存获取
    if let Some(user) = get_im_user_from_cache(&cache_key_mobile(mobile)).await? {
        return Ok(user);
    }

    let user = sqlx::query_as!(
        ImUser,
        r#"SELECT open_id as user_id, name as user_name, password_hash as password,
                phone as mobile, create_time, update_time, version, del_flag
         FROM users
         WHERE phone = $1 AND (del_flag IS NULL OR del_flag = 1)"#,
        mobile
    )
    .fetch_optional(conn)
    .await?;

    match user {
        Some(u) => {
            // 缓存用户信息
            let _ = cache_im_user(&u).await;
            Ok(u)
        }
        None => Err(AppError::not_found(mobile)),
    }
}

/// 创建用户
pub async fn create(
    user_id: String,
    user_name: String,
    password: String,
    mobile: Option<String>,
) -> AppResult<ImSafeUser> {
    let conn = db::pool();

    //
    let _ = user_id
        .parse::<i64>()
        .map_err(|_| AppError::public("user_id 必须为数字"))?;

    // 检查用户名是否已存在
    if get_by_user_name(&user_name).await.is_ok() {
        return Err(AppError::public(format!("{user_name} 已经存在")));
    }

    // 加密密码
    let password_hash = utils::hash_password(&password)?;

    // 检查 open_id 是否已存在（user_id 对应 open_id）
    let existing = sqlx::query!("SELECT id FROM users WHERE open_id = $1", user_id)
        .fetch_optional(conn)
        .await?;

    if existing.is_some() {
        return Err(AppError::public("open_id 冲突"));
    }

    let timestamp = OffsetDateTime::now_utc();

    // 插入到 users 表，使用 open_id 作为 user_id
    // 注意：users 表需要 email 字段，这里使用 user_id@im.local 作为临时 email
    let email = format!("{}@im.local", user_id);
    sqlx::query!(
        r#"INSERT INTO users (open_id, name, email, password_hash, phone, create_time, update_time, version, del_flag, status)
         VALUES ($1, $2, $3, $4, $5, $6, $6, 1, 1, 1)"#,
        user_id,
        user_name,
        email,
        password_hash,
        mobile,
        timestamp.unix_timestamp() * 1000,
    )
    .execute(conn)
    .await?;

    let new_user = ImUser {
        user_id: user_id.clone(),
        user_name: Some(user_name),
        password: Some(password_hash),
        mobile,
        create_time: Some(timestamp.unix_timestamp() * 1000),
        update_time: Some(timestamp.unix_timestamp() * 1000),
        version: Some(1),
        del_flag: Some(1),
    };

    // 自动创建对应的 im_user_data 记录
    let default_user_data = ImUserData {
        user_id: user_id.clone(),
        name: None,
        avatar: None,
        gender: None,
        birthday: None,
        location: None,
        self_signature: None,
        friend_allow_type: 1,  // 默认允许任何人添加好友
        forbidden_flag: 0,     // 默认未封禁
        disable_add_friend: 0, // 默认允许添加好友
        silent_flag: 0,        // 默认未静音
        user_type: 1,          // 默认普通用户
        del_flag: 1,           // 默认未删除
        extra: None,
        create_time: Some(timestamp),
        update_time: Some(timestamp),
        version: Some(1),
    };

    // 创建用户数据记录
    if let Err(e) = create_user_data_internal(default_user_data, timestamp).await {
        tracing::warn!(error = ?e, user_id = %user_id, "创建用户数据记录失败，但用户已创建");
    }

    // 缓存新创建的用户
    let _ = cache_im_user(&new_user).await;

    Ok(new_user.into())
}

/// 内部方法：创建用户数据记录
async fn create_user_data_internal(user_data: ImUserData, now: OffsetDateTime) -> AppResult<()> {
    let conn = db::pool();
    sqlx::query!(
        r#"INSERT INTO im_user_data
         (user_id, name, avatar, gender, birthday, location, self_signature,
          friend_allow_type, forbidden_flag, disable_add_friend, silent_flag,
          user_type, del_flag, extra, create_time, update_time, version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 1)"#,
        user_data.user_id,
        user_data.name,
        user_data.avatar,
        user_data.gender,
        user_data.birthday,
        user_data.location,
        user_data.self_signature,
        user_data.friend_allow_type,
        user_data.forbidden_flag,
        user_data.disable_add_friend,
        user_data.silent_flag,
        user_data.user_type,
        user_data.del_flag,
        user_data.extra,
        now,
        now
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn get_user_data(user_id: &str) -> AppResult<ImUserData> {
    if USE_REDIS {
        match RedisClient::get(&cache_key_user_data(user_id)).await {
            Ok(Some(data_json)) => match serde_json::from_str::<ImUserData>(&data_json) {
                Ok(data) => {
                    tracing::debug!("从缓存获取 ImUserData: {}", user_id);
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "反序列化 ImUserData 失败");
                    let _ = RedisClient::del(&cache_key_user_data(user_id)).await;
                }
            },
            Ok(None) => {}
            Err(e) => {
                tracing::error!(error = ?e, "从缓存获取 ImUserData 失败");
            }
        }
    }

    let conn = db::pool();

    let user_data = sqlx::query_as!(
        ImUserData,
        "SELECT user_id, name, avatar, gender, birthday, location, self_signature,
                friend_allow_type, forbidden_flag, disable_add_friend, silent_flag,
                user_type, del_flag, extra, create_time, update_time, version
         FROM im_user_data
         WHERE user_id = $1 AND del_flag = 1",
        user_id
    )
    .fetch_optional(conn)
    .await?;

    match user_data {
        Some(u) => {
            // 缓存用户数据（TTL: 1小时）
            if USE_REDIS && let Ok(data_json) = serde_json::to_string(&u) {
                let _ = RedisClient::set_with_ttl(&cache_key_user_data(user_id), &data_json, 3600)
                    .await;
            }

            Ok(u)
        }
        None => {
            // 如果用户数据不存在，检查用户是否存在，如果存在则自动创建默认数据
            if let Ok(_user) = get_by_user_id(user_id).await {
                let now = OffsetDateTime::now_utc();
                let default_user_data = ImUserData {
                    user_id: user_id.to_string(),
                    name: None,
                    avatar: None,
                    gender: None,
                    birthday: None,
                    location: None,
                    self_signature: None,
                    friend_allow_type: 1,
                    forbidden_flag: 0,
                    disable_add_friend: 0,
                    silent_flag: 0,
                    user_type: 1,
                    del_flag: 1,
                    extra: None,
                    create_time: Some(now),
                    update_time: Some(now),
                    version: Some(1),
                };

                // 创建默认用户数据
                if let Err(e) = create_user_data_internal(default_user_data.clone(), now).await {
                    tracing::warn!(error = ?e, user_id = %user_id, "自动创建用户数据失败");
                    return Err(AppError::not_found(user_id));
                }

                // 缓存新创建的用户数据
                if USE_REDIS && let Ok(data_json) = serde_json::to_string(&default_user_data) {
                    let _ =
                        RedisClient::set_with_ttl(&cache_key_user_data(user_id), &data_json, 3600)
                            .await;
                }

                Ok(default_user_data)
            } else {
                Err(AppError::not_found(user_id))
            }
        }
    }
}

pub async fn upsert_user_data(user_data: ImUserData) -> AppResult<()> {
    let conn = db::pool();
    let now = OffsetDateTime::now_utc();

    sqlx::query!(
        r#"INSERT INTO im_user_data
                 (user_id, name, avatar, gender, birthday, location, self_signature,
                  friend_allow_type, forbidden_flag, disable_add_friend, silent_flag,
                  user_type, del_flag, extra, create_time, update_time, version)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 1)
                 ON CONFLICT (user_id) DO UPDATE SET
                 name = EXCLUDED.name,
                 avatar = EXCLUDED.avatar,
                 gender = EXCLUDED.gender,
                 birthday = EXCLUDED.birthday,
                 location = EXCLUDED.location,
                 self_signature = EXCLUDED.self_signature,
                 friend_allow_type = EXCLUDED.friend_allow_type,
                 forbidden_flag = EXCLUDED.forbidden_flag,
                 disable_add_friend = EXCLUDED.disable_add_friend,
                 silent_flag = EXCLUDED.silent_flag,
                 user_type = EXCLUDED.user_type,
                 extra = EXCLUDED.extra,
                 update_time = EXCLUDED.update_time,
                 version = im_user_data.version + 1"#,
        user_data.user_id,
        user_data.name,
        user_data.avatar,
        user_data.gender,
        user_data.birthday,
        user_data.location,
        user_data.self_signature,
        user_data.friend_allow_type,
        user_data.forbidden_flag,
        user_data.disable_add_friend,
        user_data.silent_flag,
        user_data.user_type,
        user_data.del_flag,
        user_data.extra,
        now,
        now
    )
    .execute(conn)
    .await?;

    if USE_REDIS {
        let _ = RedisClient::del(&cache_key_user_data(&user_data.user_id)).await;
    }

    Ok(())
}

pub async fn verify_user(user_name: &str, password: &str) -> AppResult<ImSafeUser> {
    let user = get_by_user_name(user_name).await?;

    match &user.password {
        Some(password_hash) => {
            utils::verify_password(password, password_hash)?;
            Ok(user.into())
        }
        None => Err(AppError::public("Invalid Password")),
    }
}
