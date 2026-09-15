//! HTTP 层的错误表示。
//!
//! 对外只暴露结构化的错误类别与一句说明。**错误消息不得包含请求内容**：原文只进审计库，
//! 任何情况下都不随错误响应回显。

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub kind: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "a valid console session is required",
        )
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", what)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }

    /// 请求内容无法安全处理时的统一出口。调用方必须**不转发**该请求。
    pub fn unprocessable(kind: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, kind, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": {
                    "kind": self.kind,
                    "message": self.message,
                }
            })),
        )
            .into_response()
    }
}

impl From<privacy_store::Error> for ApiError {
    fn from(error: privacy_store::Error) -> Self {
        match error {
            privacy_store::Error::NotFound(what) => Self::not_found(what),
            privacy_store::Error::ProviderNameTaken(name) => {
                Self::conflict(format!("provider '{name}' already exists"))
            }
            privacy_store::Error::RuleIdTaken(id) => {
                Self::conflict(format!("rule '{id}' already exists"))
            }
            privacy_store::Error::InvalidProvider(reason) => Self::bad_request(reason),
            privacy_store::Error::AdminAlreadyExists => {
                Self::conflict("the admin account already exists")
            }
            privacy_store::Error::InvalidCredentials(reason) => Self::bad_request(reason),
            privacy_store::Error::AdminMissing => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "admin_missing",
                "no admin account is configured",
            ),
            privacy_store::Error::Rule(error) => Self::bad_request(error.to_string()),
            other => {
                // 数据库与编码错误的原文可能包含语句与数据，只记录类别。
                tracing::error!(error_kind = "store", error = %other, "store operation failed");
                Self::internal("storage operation failed")
            }
        }
    }
}

impl From<privacy_rules::Error> for ApiError {
    fn from(error: privacy_rules::Error) -> Self {
        Self::bad_request(error.to_string())
    }
}
