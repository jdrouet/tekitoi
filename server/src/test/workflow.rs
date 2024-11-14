use std::borrow::Cow;
use std::collections::HashMap;

use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, TokenResponse};
use reqwest::Url;

use crate::service::dataset::{CLIENT_ID, CLIENT_SECRET, REDIRECT_URI};

fn get_login_url(page: &str) -> Option<&str> {
    let index = page.find("/authorize/profiles/login")?;
    let len = page[index..].find("\"")?;
    Some(&page[index..(index + len)])
}

fn get_redirection_url(page: &str) -> Option<Url> {
    let index = page.find("1; url='").map(|v| v + 8)?;
    let len = page[index..].find("'\"")?;
    Url::parse(&page[index..(index + len)]).ok()
}

#[tokio::test]
async fn should_authenticate() {
    let port = 9900;
    let app = crate::app::Application::test_with_port(port).await;
    let _handler = tokio::spawn(async move { app.run().await });

    let client = oauth2::basic::BasicClient::new(
        oauth2::ClientId::new(CLIENT_ID.to_string()),
        Some(oauth2::ClientSecret::new(CLIENT_SECRET.into())),
        oauth2::AuthUrl::new(format!("http://localhost:{port}/authorize")).unwrap(),
        Some(oauth2::TokenUrl::new(format!("http://localhost:{port}/api/access-token")).unwrap()),
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(oauth2::RedirectUrl::new(REDIRECT_URI.to_string()).unwrap());

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = oauth2::PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, _csrf_token) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let req = reqwest::get(auth_url.to_string()).await.unwrap();
    let status = req.status();
    let body = req.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let login_url = get_login_url(&body).unwrap();
    let login_url = format!("http://localhost:{port}{login_url}");
    let req = reqwest::get(login_url).await.unwrap();
    let status = req.status();
    let body = req.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let redirection = get_redirection_url(body.as_str()).unwrap();
    let params: HashMap<Cow<'_, str>, Cow<'_, str>> = redirection.query_pairs().collect();

    let token = client
        .exchange_code(AuthorizationCode::new(
            params.get("code").unwrap().to_string(),
        ))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .unwrap();

    let _user: serde_json::Value = reqwest::Client::new()
        .get(format!("http://localhost:{port}/api/user-info"))
        .header(
            "Authorization",
            format!("Bearer {}", token.access_token().secret()),
        )
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
}
