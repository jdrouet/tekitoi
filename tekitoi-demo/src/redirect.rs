use actix_web::{get, http::header::LOCATION, web::Data, web::Query, HttpResponse};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, PkceCodeVerifier, TokenResponse};
use redis::AsyncCommands;

// Once the user has been redirected to the redirect URL, you'll have access to the
// authorization code. For security reasons, your code should verify that the `state`
// parameter returned by the server matches `csrf_state`.

#[derive(Debug, serde::Deserialize)]
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

#[get("/api/redirect")]
async fn handler(
    oauth_client: Data<BasicClient>,
    redis_client: Data<redis::Client>,
    params: Query<QueryParams>,
) -> HttpResponse {
    tracing::trace!("authorize redirection {:?}", params);
    match params.0 {
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
                    HttpResponse::Found()
                        .append_header((
                            LOCATION,
                            format!("/?token={}", token.access_token().secret()),
                        ))
                        .finish()
                })
                .unwrap_or_else(|err| HttpResponse::ServiceUnavailable().json(err.to_string()))
        }
        QueryParams::Error {
            error_uri, error, ..
        } => {
            tracing::debug!("error with message {:?}, redirecting...", error);
            HttpResponse::Found()
                .append_header((LOCATION, error_uri))
                .finish()
        }
    }
}
