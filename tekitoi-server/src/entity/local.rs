use oauth2::PkceCodeVerifier;

use super::incoming::IncomingAuthorizationRequest;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LocalAuthorizationRequest {
    pub initial: IncomingAuthorizationRequest,
    pub pkce_verifier: PkceCodeVerifier,
}
