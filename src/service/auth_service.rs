use crate::config::JwtConfig;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::{Duration, UtcDateTime};
#[derive(Deserialize, Serialize, Debug)]
pub struct JwtClaims {
    pub open_id: u64,
    pub exp: i64,
    pub iat: i64,
}

impl JwtClaims {
    pub fn new(open_id: u64, exp: i64) -> Self {
        let now = UtcDateTime::now();
        let exp = now + Duration::hours(exp);
        JwtClaims {
            open_id,
            exp: exp.unix_timestamp(),
            iat: now.unix_timestamp(),
        }
    }
}

pub fn get_token(open_id: u64, jwt_config: &JwtConfig) -> anyhow::Result<String> {
    let claim = JwtClaims::new(open_id, jwt_config.expiry);
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claim,
        &EncodingKey::from_secret(jwt_config.secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &str, jwt_config: &JwtConfig) -> anyhow::Result<JwtClaims> {
    let claims = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(claims.claims)
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn test_utc() {
        assert_eq!(
            UtcDateTime::now().unix_timestamp(),
            OffsetDateTime::now_utc().unix_timestamp()
        );
    }
}
