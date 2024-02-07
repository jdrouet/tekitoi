use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use sailfish::TemplateOnce;

#[derive(Clone, Debug, TemplateOnce)]
#[template(path = "error.html")]
pub struct ViewError {
    code: StatusCode,
    message: String,
    description: String,
}

impl ViewError {
    pub fn bad_request(message: String, description: String) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST,
            message,
            description,
        }
    }

    pub fn not_found(message: String, description: String) -> Self {
        Self {
            code: StatusCode::NOT_FOUND,
            message,
            description,
        }
    }
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "view error {}: {:?}", self.code.as_u16(), self.message)
    }
}

impl IntoResponse for ViewError {
    fn into_response(self) -> Response {
        let template = self
            .clone()
            .render_once()
            .expect("couldn't render error page");

        (self.code, Html(template)).into_response()
    }
}

impl From<deadpool_redis::PoolError> for ViewError {
    fn from(error: deadpool_redis::PoolError) -> Self {
        ViewError {
            code: StatusCode::SERVICE_UNAVAILABLE,
            message: "Unable to perform internal action.".into(),
            description: error.to_string(),
        }
    }
}

impl From<deadpool_redis::redis::RedisError> for ViewError {
    fn from(error: deadpool_redis::redis::RedisError) -> Self {
        ViewError {
            code: StatusCode::SERVICE_UNAVAILABLE,
            message: "Unable to perform internal action.".into(),
            description: error.to_string(),
        }
    }
}

impl From<serde_qs::Error> for ViewError {
    fn from(error: serde_qs::Error) -> Self {
        ViewError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Unable to perform internal action.".into(),
            description: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for ViewError {
    fn from(error: serde_json::Error) -> Self {
        ViewError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Unable to perform internal action.".into(),
            description: error.to_string(),
        }
    }
}

impl From<sqlx::Error> for ViewError {
    fn from(error: sqlx::Error) -> Self {
        ViewError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Unable to perform internal action.".into(),
            description: error.to_string(),
        }
    }
}
