use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Extension;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, PkceCodeVerifier, TokenResponse};
use redis::AsyncCommands;

// Once the user has been redirected to the redirect URL, you'll have access to the
// authorization code. For security reasons, your code should verify that the `state`
// parameter returned by the server matches `csrf_state`.

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum QueryParams {
    Success {
        code: String,
        state: String,
    },
    Error {
        error: String,
        error_description: String,
        error_uri: String,
        state: String,
    },
}

// #[get("/api/redirect")]
pub async fn handler(
    Extension(oauth_client): Extension<BasicClient>,
    State(redis_client): State<redis::Client>,
    Query(params): Query<QueryParams>,
) -> Redirect {
    tracing::trace!("authorize redirection {:?}", params);
    match params {
        QueryParams::Success { code, state } => {
            let mut redis_con = redis_client
                .get_async_connection()
                .await
                .expect("couldn't get redis connection");
            let pkce_verifier: String = redis_con
                .get(state)
                .await
                .expect("couldn't fetch from cache");
            let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);

            // Now you can trade it for an access token.
            oauth_client
                .exchange_code(AuthorizationCode::new(code))
                // Set the PKCE code verifier.
                .set_pkce_verifier(pkce_verifier)
                .request_async(async_http_client)
                .await
                .map(|token| {
                    Redirect::temporary(
                        format!("/?token={}", token.access_token().secret()).as_str(),
                    )
                })
                .unwrap_or_else(|err| {
                    Redirect::temporary(format!("/?error={}", err.to_string()).as_str())
                })
        }
        QueryParams::Error {
            error_uri, error, ..
        } => {
            tracing::debug!("error with message {:?}, redirecting...", error);
            Redirect::temporary(error_uri.as_str())
        }
    }
}
