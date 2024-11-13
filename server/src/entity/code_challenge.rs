use std::{borrow::Cow, str::FromStr};

use sha2::Digest;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub(crate) enum CodeChallengeMethod {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "S256")]
    S256,
}

#[derive(Clone, Debug)]
pub(crate) struct CodeChallengeMethodParserError(pub String);

impl std::error::Error for CodeChallengeMethodParserError {}

impl std::fmt::Display for CodeChallengeMethodParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid code challenge method {:?}", self.0)
    }
}

impl FromStr for CodeChallengeMethod {
    type Err = CodeChallengeMethodParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plain" => Ok(Self::Plain),
            "S256" => Ok(Self::S256),
            other => Err(CodeChallengeMethodParserError(other.to_string())),
        }
    }
}

impl CodeChallengeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::S256 => "S256",
        }
    }

    pub fn hash<'a>(&self, input: &'a str) -> Cow<'a, str> {
        match self {
            Self::Plain => Cow::Borrowed(input),
            Self::S256 => {
                use base64::engine::general_purpose::STANDARD_NO_PAD;
                use base64::Engine;

                let hash = sha2::Sha256::digest(input.as_bytes());
                Cow::Owned(
                    STANDARD_NO_PAD
                        .encode(hash)
                        .replace('+', "-")
                        .replace('/', "_"),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    struct Container {
        method: CodeChallengeMethod,
    }

    #[test]
    fn should_serialize() {
        assert_eq!(
            serde_urlencoded::to_string(&Container {
                method: CodeChallengeMethod::Plain
            })
            .unwrap(),
            "method=plain"
        );
        assert_eq!(
            serde_urlencoded::to_string(&Container {
                method: CodeChallengeMethod::S256
            })
            .unwrap(),
            "method=S256"
        );
    }

    #[test]
    fn should_deserialize() {
        assert_eq!(
            serde_json::from_str::<'_, Container>("{\"method\":\"plain\"}").unwrap(),
            Container {
                method: CodeChallengeMethod::Plain
            },
        );
        assert_eq!(
            serde_json::from_str::<'_, Container>("{\"method\":\"S256\"}").unwrap(),
            Container {
                method: CodeChallengeMethod::S256
            },
        );
    }
}
