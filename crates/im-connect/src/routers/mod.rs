use salvo::prelude::*;

pub fn root() -> Router {
    let router = Router::new().append(&mut create_router());
    router
}

pub fn create_router() -> Vec<Router> {
    vec![Router::new()]
}
