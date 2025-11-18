use salvo::prelude::*;

use crate::api::*;

pub fn root() -> Router {
    let router = Router::new()
        .hoop(Logger::new())
        .push(create_public_router())
        .push(create_private_router());

    router
}

pub fn create_public_router() -> Router {
    Router::with_path("auth")
        .push(Router::with_path("login"))
        .push(Router::with_path("register"))
}

pub fn create_private_router() -> Router {
    Router::with_path("api/v1").push(Router::with_path("users").get(user::list_users))
}
