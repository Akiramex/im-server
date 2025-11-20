use salvo::{
    http::{ParseError, ResBody},
    prelude::*,
};

mod cors;
mod jwt;

pub use cors::cors_hoop;
pub use jwt::auth_hoop;
use tracing::{info, warn};

use crate::AppError;

const CUSTOM_404_PAGE: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>404 Not Found</title>
</head>
<body>
    <h1>404 Not Found</h1>
    <p>The requested resource could not be found.</p>
</body>
</html>
"#;

#[handler]
pub async fn error_404(res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(StatusCode::NOT_FOUND) = res.status_code {
        res.render("404 Not Found");
        ctrl.skip_rest();
    }
    // res.render(Text::Html(CUSTOM_404_PAGE));
    // ctrl.skip_rest();
}

#[handler]
pub async fn catch_status_error(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl,
) {
    if let ResBody::Error(e) = &res.body {
        if let Some(e) = &e.cause {
            if let Some(e) = e.downcast_ref::<ParseError>() {
                #[cfg(debug_assertions)]
                let error = AppError::Public(e.to_string());
                #[cfg(not(debug_assertions))]
                let error = AppError::Public("bad json");

                error.write(req, depot, res).await;
                ctrl.skip_rest();
            } else {
                let error = AppError::Public(format!("unknown error: {}", e));
                error.write(req, depot, res).await;
                ctrl.skip_rest();
            }
        } else {
            let error = AppError::Public(e.brief.clone());
            error.write(req, depot, res).await;
            ctrl.skip_rest();
        }
    } else if res.status_code == Some(StatusCode::METHOD_NOT_ALLOWED) {
        let error = AppError::Public("method not allowed".to_string());
        error.write(req, depot, res).await;
        ctrl.skip_rest();
    } else if res.status_code == Some(StatusCode::NOT_FOUND) {
        let error = AppError::Public("404 not found".to_string());
        error.write(req, depot, res).await;
        ctrl.skip_rest();
    } else {
        let error = AppError::Public("unknown error".to_string());
        error.write(req, depot, res).await;
        ctrl.skip_rest();
    }
}
