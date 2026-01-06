use salvo::prelude::*;

use crate::api;

pub fn root() -> Router {
    let router = Router::new().append(&mut create_router());
    router
}

pub fn create_router() -> Vec<Router> {
    let route = Router::with_path("ws/{subscription_id}").goal(api::websocket::ws_handler);
    vec![route]
}
