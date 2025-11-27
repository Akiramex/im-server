use crate::api::*;
use crate::hoops;
use salvo::prelude::*;

pub fn root() -> Router {
    let router = Router::new()
        .push(create_public_router())
        .push(create_private_router());
    let doc = OpenApi::new("im-server web api", "0.0.1").merge_router(&router);
    router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(Scalar::new("/api-doc/openapi.json").into_router("scalar"))
}

pub fn create_public_router() -> Router {
    Router::with_path("auth")
        .push(Router::with_path("login").post(auth_api::post_login))
        .push(Router::with_path("register").post(auth_api::register))
}

pub fn create_private_router() -> Router {
    Router::with_path("api/v1")
        .push(
            Router::with_path("users")
                .hoop(hoops::auth_hoop)
                .post(user_api::create_user)
                .get(user_api::list_users)
                .put(user_api::update_current_user)
                .push(Router::with_path("{id}").get(user_api::get_user)),
        )
        .push(
            Router::with_path("friends")
                .hoop(hoops::auth_hoop)
                .get(friend_api::get_friends)
                .push(
                    Router::with_path("{id}")
                        .post(friend_api::add_friend)
                        .delete(friend_api::remove_friend),
                ),
        )
}
