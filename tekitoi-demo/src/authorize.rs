use axum::extract::State;
use axum::response::Redirect;
use axum::Extension;
use oauth2::basic::BasicClient;
use oauth2::{CsrfToken, PkceCodeChallenge};

// #[get("/api/authorize")]
pub async fn handler(
    Extension(oauth_client): Extension<BasicClient>,
    State(cache): State<crate::settings::LocalCache>,
) -> Redirect {
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
    cache.set(csrf_token, pkce_verifier).await;

    tracing::trace!("redirecting to {:?}", auth_url);

    Redirect::temporary(auth_url.as_str())
}
