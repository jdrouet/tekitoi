use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

#[derive(Debug)]
pub enum ApiError {
    BadRequest { message: String },
    InternalServer { message: String },
    // ServiceUnavailable { message: String },
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::InternalServer { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            // Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::BadRequest { message } => message.as_str(),
            Self::InternalServer { message } => message.as_str(),
            // Self::ServiceUnavailable { message } => message.as_str(),
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

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.message())
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
