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
            Router::with_path("subscriptions/{subscription_id}/user")
                .get(subcription_api::get_user_id_by_subscription),
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
                .push(Router::with_path("auth").post(im_user_api::login))
                .push(
                    Router::with_path("friendships/{open_id}/friends")
                        .get(im_friendship_api::get_friends_by_open_id),
                )
                .push(
                    Router::with_path("friends")
                        .hoop(auth_hoop)
                        .get(im_friendship_api::get_friends)
                        .post(im_friendship_api::add_friend)
                        .push(
                            Router::with_path("{to_id}")
                                .delete(im_friendship_api::remove_friend)
                                .push(
                                    Router::with_path("remark")
                                        .put(im_friendship_api::update_remark),
                                )
                                .push(
                                    Router::with_path("black")
                                        .post(im_friendship_api::black_friend),
                                ),
                        ),
                )
                .push(
                    Router::with_path("friendship-requests")
                        .hoop(auth_hoop)
                        .get(im_friendship_api::get_friendship_requests)
                        .post(im_friendship_api::create_friendship_request)
                        .push(
                            Router::with_path("{request_id}")
                                .post(im_friendship_api::handle_friendship_request),
                        ),
                ),
        );

    vec![v1]
}
