use salvo::jwt_auth::{self, CookieFinder, HeaderFinder, JwtTokenFinder, QueryFinder};
use salvo::prelude::*;

use crate::utils::auth::verify_token;

#[handler]
pub async fn auth_hoop(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let finders: Vec<Box<dyn JwtTokenFinder>> = vec![
        Box::new(HeaderFinder::new()),
        Box::new(QueryFinder::new("token")),
        Box::new(CookieFinder::new("jwt_token")),
    ];

    let token = {
        let mut result = None;
        for finder in finders {
            result = finder.find_token(req).await;
            if result.is_some() {
                break;
            }
        }
        result
    };

    if let Some(token) = token {
        match verify_token(&token) {
            Ok(data) => {
                depot.insert(jwt_auth::JWT_AUTH_DATA_KEY, data);
                depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Authorized);
                depot.insert(jwt_auth::JWT_AUTH_TOKEN_KEY, token);
            }
            Err(e) => {
                tracing::info!(error = ?e, "jwt auth error");
                depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Forbidden);
                depot.insert(jwt_auth::JWT_AUTH_ERROR_KEY, e);
                res.status_code(StatusCode::FORBIDDEN);

                ctrl.skip_rest();
            }
        }
    } else {
        depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Unauthorized);
        // 自定义返回值
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render("未登录");
        ctrl.skip_rest();
    }
}
