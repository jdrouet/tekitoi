use super::local::LocalAuthorizationRequest;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RedirectedAuthorizationRequest {
    pub inner: LocalAuthorizationRequest,
    pub code: String,
    pub kind: String,
}
