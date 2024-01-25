use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(Debug)]
pub enum ApiError {
    BadRequest { message: String },
    InternalServer { message: String },
    Unauthorized { message: String },
    // ServiceUnavailable { message: String },
}

impl ApiError {
    pub fn bad_request<T: ToString>(value: T) -> Self {
        Self::BadRequest {
            message: value.to_string(),
        }
    }

    pub fn internal_server<T: ToString>(value: T) -> Self {
        Self::InternalServer {
            message: value.to_string(),
        }
    }
    pub fn unauthorized<T: ToString>(value: T) -> Self {
        Self::Unauthorized {
            message: value.to_string(),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::InternalServer { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::BadRequest { message } => message.as_str(),
            Self::InternalServer { message } => message.as_str(),
            Self::Unauthorized { message } => message.as_str(),
        }
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
        ApiError::InternalServer {
            message: "Unable to perform internal action".into(),
        }
    }
}

impl From<deadpool_redis::redis::RedisError> for ApiError {
    fn from(error: deadpool_redis::redis::RedisError) -> Self {
        tracing::error!("redis error: {:?}", error);
        ApiError::InternalServer {
            message: "Unable to perform internal action".into(),
        }
    }
}

impl From<serde_qs::Error> for ApiError {
    fn from(error: serde_qs::Error) -> Self {
        tracing::error!("query string deserialize error: {:?}", error);
        ApiError::InternalServer {
            message: "Unable to perform internal action".into(),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        tracing::error!("json string deserialize error: {:?}", error);
        ApiError::InternalServer {
            message: "Unable to perform internal action".into(),
        }
    }
}
