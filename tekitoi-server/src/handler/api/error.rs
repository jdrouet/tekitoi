use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(Debug)]
pub struct ApiError {
    code: StatusCode,
    message: String,
}

impl ApiError {
    pub fn bad_request<T: ToString>(value: T) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST,
            message: value.to_string(),
        }
    }

    pub fn internal_server<T: ToString>(value: T) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: value.to_string(),
        }
    }
    pub fn unauthorized<T: ToString>(value: T) -> Self {
        Self {
            code: StatusCode::UNAUTHORIZED,
            message: value.to_string(),
        }
    }

    fn status_code(&self) -> StatusCode {
        self.code
    }

    fn message(&self) -> &str {
        &self.message
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
            self.message()
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status_code(), Json(self.message())).into_response()
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
