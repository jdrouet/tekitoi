use std::str::FromStr;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ResponseType {
    Code,
}

#[derive(Clone, Debug)]
pub(crate) struct ResponseTypeParserError(pub String);

impl std::error::Error for ResponseTypeParserError {}

impl std::fmt::Display for ResponseTypeParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid response type {:?}", self.0)
    }
}

impl FromStr for ResponseType {
    type Err = ResponseTypeParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "code" => Ok(Self::Code),
            other => Err(ResponseTypeParserError(other.to_string())),
        }
    }
}

impl ResponseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
        }
    }
}
