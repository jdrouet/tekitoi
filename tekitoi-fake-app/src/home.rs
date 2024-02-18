use std::sync::Arc;

use axum::{extract::Query, response::Html, Extension};
use sailfish::TemplateOnce;

#[derive(TemplateOnce)]
#[template(path = "home.html")]
struct HomeTemplate {
    token: Option<String>,
    user: String,
}

impl HomeTemplate {
    pub fn new(token: Option<String>, user: String) -> Self {
        Self { token, user }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct QueryParams {
    token: Option<String>,
}

impl QueryParams {
    pub async fn get_user(&self, base_url: &str) -> Result<String, String> {
        if let Some(token) = self.token.as_ref() {
            let url = format!("{}/api/user", base_url);
            reqwest::Client::new()
                .get(url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|err| err.to_string())?
                .text()
                .await
                .map_err(|err| err.to_string())
        } else {
            Ok(String::default())
        }
    }
}

// #[get("/")]
pub async fn handler(
    Extension(config): Extension<Arc<crate::settings::Settings>>,
    Query(params): Query<QueryParams>,
) -> Html<String> {
    tracing::trace!("home requested");
    let user = match params.get_user(config.api_url.as_str()).await {
        Ok(value) => value,
        Err(err) => err,
    };
    let ctx = HomeTemplate::new(params.token, user);
    let template = ctx.render_once().unwrap();

    Html(template)
}
