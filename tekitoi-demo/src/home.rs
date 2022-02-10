use actix_web::http::header::ContentType;
use actix_web::{get, web::Query, HttpResponse};
use sailfish::TemplateOnce;

#[derive(TemplateOnce)]
#[template(path = "home.html")]
struct HomeTemplate {
    token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct QueryParams {
    token: Option<String>,
}

impl From<QueryParams> for HomeTemplate {
    fn from(value: QueryParams) -> Self {
        Self { token: value.token }
    }
}

#[get("/")]
async fn handler(params: Query<QueryParams>) -> HttpResponse {
    tracing::trace!("home requested");
    let ctx: HomeTemplate = params.0.into();
    let template = ctx.render_once().unwrap();
    HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template)
}
