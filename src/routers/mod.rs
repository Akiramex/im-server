use crate::api::*;
use crate::hoops;
use salvo::prelude::*;

pub fn root() -> Router {
    let router = Router::new()
        .push(create_public_router())
        .push(create_private_router());

    router
}

pub fn create_public_router() -> Router {
    Router::with_path("auth")
        .push(Router::with_path("login").post(auth::post_login))
        .push(Router::with_path("register").post(user::create_user))
}

pub fn create_private_router() -> Router {
    Router::with_path("api/v1").push(
        Router::with_hoop(hoops::auth_hoop).push(Router::with_path("users").get(user::list_users)),
    )
}
