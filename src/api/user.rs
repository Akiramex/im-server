use salvo::prelude::*;

#[handler]
pub async fn list_users() -> String {
    "Hello, World!".to_string()
}

#[handler]
pub async fn create_user() -> String {
    "Hello, World!".to_string()
}
