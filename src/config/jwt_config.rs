use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    #[serde(default = "default_expiry")]
    pub expiry: i64,
}

impl JwtConfig {
    pub fn new(secret: String, expiry: i64) -> Self {
        JwtConfig { secret, expiry }
    }
}

fn default_expiry() -> i64 {
    24
}
