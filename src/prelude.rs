//! Crate prelude
#![allow(unused_imports)]

use salvo::writing::Json;

pub use crate::error::AppError;

pub use tracing::{debug, error, info, warn};

pub use crate::models::MyResponse;

pub use std::format as f;

pub type AppResult<T> = Result<T, AppError>;

pub type JsonResult<T> = Result<Json<T>, AppError>;

pub fn json_ok<T>(data: T) -> JsonResult<T> {
    Ok(Json(data))
}
