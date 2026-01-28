use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub status: u16,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn bad_request(message: String) -> Self {
        Self {
            code: "BAD_REQUEST".to_string(),
            message,
            status: 400,
        }
    }

    pub fn unauthorized(message: String) -> Self {
        Self {
            code: "UNAUTHORIZED".to_string(),
            message,
            status: 401,
        }
    }

    pub fn forbidden(message: String) -> Self {
        Self {
            code: "FORBIDDEN".to_string(),
            message,
            status: 403,
        }
    }

    pub fn not_found(message: String) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message,
            status: 404,
        }
    }

    pub fn conflict(message: String) -> Self {
        Self {
            code: "CONFLICT".to_string(),
            message,
            status: 409,
        }
    }

    pub fn internal(message: String) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message,
            status: 500,
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
