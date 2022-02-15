use actix_web::{get, HttpResponse};

#[get("/api/status")]
async fn handle() -> HttpResponse {
    HttpResponse::NoContent().finish()
}
