use crate::api::*;
use crate::hoops::auth_hoop;
use salvo::prelude::*;

pub fn root() -> Router {
    let router = Router::new().append(&mut create_router());
    let doc = OpenApi::new("im-server web api", "0.0.1").merge_router(&router);
    router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(Scalar::new("/api-doc/openapi.json").into_router("scalar"))
}

pub fn create_router() -> Vec<Router> {
    let v1 = Router::with_path("api/v1")
        .push(
            Router::with_path("auth")
                .push(Router::with_path("login").post(auth_api::post_login))
                .push(Router::with_path("register").post(auth_api::register)),
        )
        .push(
            Router::with_path("users")
                .hoop(auth_hoop)
                .post(user_api::create_user)
                .get(user_api::list_users)
                .put(user_api::update_current_user)
                .push(Router::with_path("{id}").get(user_api::get_user)),
        )
        .push(
            Router::with_path("friends")
                .hoop(auth_hoop)
                .get(friend_api::get_friends)
                .push(
                    Router::with_path("{id}")
                        .post(friend_api::add_friend)
                        .delete(friend_api::remove_friend),
                ),
        )
        .push(
            Router::with_path("im")
                .push(
                    Router::with_path("users")
                        .post(im_user_api::create_user)
                        .push(
                            Router::with_path("{user_id}")
                                .get(im_user_api::get_user)
                                .push(
                                    Router::with_path("data")
                                        .hoop(auth_hoop)
                                        .get(im_user_api::get_user_data)
                                        .put(im_user_api::upsert_user_data),
                                ),
                        ),
                )
                .push(Router::with_path("auth").post(im_user_api::login)),
        );

    vec![v1]
}
