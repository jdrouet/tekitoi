pub trait CachePayload: serde::Serialize + serde::de::DeserializeOwned {
    fn to_query_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    fn from_query_string(value: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(value)
    }
}
