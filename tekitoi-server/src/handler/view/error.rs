use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
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
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "view error {}: {:?}", self.code.as_u16(), self.message)
    }
}

impl ResponseError for ViewError {
    fn error_response(&self) -> HttpResponse {
        let template = self
            .clone()
            .render_once()
            .expect("couldn't render error page");
        HttpResponse::build(self.code)
            .insert_header(ContentType::html())
            .body(template)
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
