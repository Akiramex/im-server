use salvo::prelude::*;

mod cors;
mod jwt;

pub use cors::cors_hoop;
pub use jwt::auth_hoop;

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
pub async fn error_404(&self, res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(StatusCode::NOT_FOUND) = res.status_code {
        res.render("404 Not Found");
        ctrl.skip_rest();
    }
    // res.render(Text::Html(CUSTOM_404_PAGE));
    // ctrl.skip_rest();
}
