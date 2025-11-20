use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppResponse<T> {
    code: i32,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl AppResponse<()> {
    pub fn success_with_msg(msg: String) -> Self {
        Self {
            code: 0,
            msg,
            data: None,
        }
    }

    pub fn error_with_code(code: i32, msg: String) -> Self {
        Self {
            code,
            msg,
            data: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            code: -1,
            msg,
            data: None,
        }
    }
}

impl<T> AppResponse<T> {
    pub fn success_with_data(msg: String, data: T) -> Self {
        Self {
            code: 0,
            msg,
            data: Some(data),
        }
    }

    pub fn error_with_data(code: i32, msg: String, data: T) -> Self {
        Self {
            code,
            msg,
            data: Some(data),
        }
    }
}
