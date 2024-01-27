use oauth2::basic::BasicTokenType;
use oauth2::{EmptyExtraTokenFields, StandardTokenResponse};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProviderAccessToken {
    pub inner: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    pub kind: String,
    pub client_id: String,
}
