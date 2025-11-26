use salvo::jwt_auth::{self, CookieFinder, HeaderFinder, JwtTokenFinder, QueryFinder};
use salvo::prelude::*;
use tracing::warn;

use crate::config;
use crate::utils::auth::verify_token;
use crate::{AppError, service};

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
        match verify_token(&token, &config::get().jwt) {
            Ok(data) => {
                let open_id = data.open_id.to_string();
                let user = service::user_service::get_by_open_id(&open_id).await;
                if let Ok(user) = user {
                    depot.inject(user);
                } else {
                    warn!("Failed to get user by open_id: {}", open_id);
                }
                depot.insert(jwt_auth::JWT_AUTH_DATA_KEY, data);
                depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Authorized);
                depot.insert(jwt_auth::JWT_AUTH_TOKEN_KEY, token);
            }
            Err(e) => {
                tracing::info!(error = ?e, "jwt auth error");
                depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Forbidden);
                depot.insert(jwt_auth::JWT_AUTH_ERROR_KEY, e);
                AppError::unauthorized("用户验证失败")
                    .write(req, depot, res)
                    .await;
                ctrl.skip_rest();
            }
        }
    } else {
        depot.insert(jwt_auth::JWT_AUTH_STATE_KEY, JwtAuthState::Unauthorized);
        AppError::unauthorized("用户未登录")
            .write(req, depot, res)
            .await;
        ctrl.skip_rest();
    }
}
