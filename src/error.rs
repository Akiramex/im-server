use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};
use salvo::prelude::*;
use thiserror::Error;

use crate::MyResponse;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("public: `{0}`")]
    Public(String),
    #[error("internal: `{0}`")]
    Internal(String),
    #[error("parse: `{0}`")]
    ParseError(#[from] salvo::http::ParseError),
    #[error("http status error: `{0}`")]
    HttpStatus(#[from] StatusError),
    #[error("anyhow error:`{0}`")]
    Anyhow(#[from] anyhow::Error),
    #[error("sqlx::Error:`{0}`")]
    SqlxError(#[from] sqlx::Error),
}

#[async_trait]
impl Writer for AppError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let code = match &self {
            Self::HttpStatus(err) => err.code,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let scribe: MyResponse<_> = match self {
            Self::Public(msg) | Self::Internal(msg) => MyResponse::error(msg),
            e => MyResponse::error_with_code(-2, e.to_string()),
        };
        res.status_code(code);
        res.render(Json(scribe));
    }
}

impl AppError {
    pub fn public<S: Into<String>>(msg: S) -> Self {
        Self::Public(msg.into())
    }

    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }
}

impl EndpointOutRegister for AppError {
    fn register(components: &mut salvo::oapi::Components, operation: &mut salvo::oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", StatusError::to_schema(components)),
        );
    }
}
