use std::str::FromStr;

pub(crate) const CODE_CODE: u8 = 0;

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

#[derive(Clone, Debug)]
pub(crate) struct ResponseTypeDecodeError(pub u8);

impl std::error::Error for ResponseTypeDecodeError {}

impl std::fmt::Display for ResponseTypeDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid response type {:?}", self.0)
    }
}

impl TryFrom<u8> for ResponseType {
    type Error = ResponseTypeDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            CODE_CODE => Ok(Self::Code),
            other => Err(ResponseTypeDecodeError(other)),
        }
    }
}

impl ResponseType {
    pub const fn as_code(&self) -> u8 {
        match self {
            Self::Code => CODE_CODE,
        }
    }
}
