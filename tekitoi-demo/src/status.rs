use actix_web::{get, HttpResponse};

#[get("/api/status")]
async fn handler() -> HttpResponse {
    tracing::trace!("status requested");
    HttpResponse::NoContent().finish()
}
