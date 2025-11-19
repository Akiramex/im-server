use salvo::http::{ParseError, StatusCode, StatusError};
use salvo::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("public: `{0}`")]
    Public(String),
    #[error("internal: `{0}`")]
    Internal(String),
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
        let scribe = match self {
            Self::Public(msg) => StatusError::internal_server_error().brief(msg),
            Self::Internal(msg) => {
                tracing::error!(msg = msg, "internal error");
                StatusError::internal_server_error()
            }
            Self::HttpStatus(e) => e,
            e => StatusError::internal_server_error()
                .brief(format!("Unknown error happened: {e}"))
                .cause(e),
        };
        res.status_code(code);
        res.render(scribe);
    }
}
