use std::borrow::Cow;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use url::Url;

#[derive(Debug, serde::Serialize)]
pub(crate) struct ApiError {
    #[serde(skip)]
    code: StatusCode,
    error: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_description: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_uri: Option<Url>,
}

impl ApiError {
    pub(crate) fn bad_request<T: Into<Cow<'static, str>>>(value: T) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST,
            error: value.into(),
            error_description: None,
            error_uri: None,
        }
    }

    pub(crate) fn internal_server<T: Into<Cow<'static, str>>>(value: T) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            error: value.into(),
            error_description: None,
            error_uri: None,
        }
    }

    pub(crate) fn unauthorized<T: Into<Cow<'static, str>>>(value: T) -> Self {
        Self {
            code: StatusCode::UNAUTHORIZED,
            error: value.into(),
            error_description: None,
            error_uri: None,
        }
    }

    fn status_code(&self) -> StatusCode {
        self.code
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:?}",
            self.status_code()
                .canonical_reason()
                .unwrap_or("unknown status code"),
            self.error
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code(), Json(self)).into_response()
    }
}

impl From<deadpool_redis::PoolError> for ApiError {
    fn from(error: deadpool_redis::PoolError) -> Self {
        tracing::error!("redis pool error: {:?}", error);
        Self::internal_server("Unable to perform internal action")
    }
}

impl From<deadpool_redis::redis::RedisError> for ApiError {
    fn from(error: deadpool_redis::redis::RedisError) -> Self {
        tracing::error!("redis error: {:?}", error);
        Self::internal_server("Unable to perform internal action")
    }
}

impl From<serde_qs::Error> for ApiError {
    fn from(error: serde_qs::Error) -> Self {
        tracing::error!("query string deserialize error: {:?}", error);
        Self::internal_server("Unable to perform internal action")
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        tracing::error!("json string deserialize error: {:?}", error);
        Self::internal_server("Unable to perform internal action")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!("database error: {:?}", error);
        Self::internal_server("Unable to perform internal action")
    }
}
