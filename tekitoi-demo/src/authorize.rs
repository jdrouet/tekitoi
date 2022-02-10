use actix_web::{get, http::header::LOCATION, web::Data, HttpResponse};
use oauth2::basic::BasicClient;
use oauth2::{CsrfToken, PkceCodeChallenge};
use redis::AsyncCommands;

#[get("/api/authorize")]
async fn handler(
    oauth_client: Data<BasicClient>,
    redis_client: Data<redis::Client>,
) -> HttpResponse {
    tracing::trace!("authorize requested");
    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // Generate the full authorization URL.
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    tracing::debug!(
        "csrf_token={:?} pkce_verifier={:?}",
        csrf_token.secret(),
        pkce_verifier.secret()
    );
    let mut redis_con = redis_client
        .get_async_connection()
        .await
        .expect("couldn't get redis connection");
    let _: String = redis_con
        .set_ex(csrf_token.secret(), pkce_verifier.secret(), 60 * 10)
        .await
        .expect("couldn't persist in cache");

    tracing::trace!("redirecting to {:?}", auth_url);

    HttpResponse::Found()
        .append_header((LOCATION, auth_url.to_string()))
        .finish()
}
