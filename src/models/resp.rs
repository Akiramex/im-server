use salvo::oapi::ToSchema;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MyResponse<T> {
    code: i32,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl MyResponse<()> {
    pub fn success_with_msg(msg: impl Into<String>) -> Self {
        Self {
            code: 0,
            msg: msg.into(),
            data: None,
        }
    }

    pub fn error_with_code(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            code: -1,
            msg: msg.into(),
            data: None,
        }
    }
}

impl<T> MyResponse<T> {
    pub fn success_with_data(msg: impl Into<String>, data: T) -> Self {
        Self {
            code: 0,
            msg: msg.into(),
            data: Some(data),
        }
    }

    pub fn error_with_data(code: i32, msg: impl Into<String>, data: T) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: Some(data),
        }
    }
}
