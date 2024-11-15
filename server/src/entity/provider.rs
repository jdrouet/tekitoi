use std::str::FromStr;

use uuid::Uuid;

pub(crate) const PROFILES_NAME: &str = "profiles";
pub(crate) const PROFILES_CODE: u8 = 0;
pub(crate) const CREDENTIALS_NAME: &str = "credentials";
pub(crate) const CREDENTIALS_CODE: u8 = 1;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderKind {
    Credentials,
    Profiles,
}

impl ProviderKind {
    pub const fn as_code(&self) -> u8 {
        match self {
            Self::Credentials => CREDENTIALS_CODE,
            Self::Profiles => PROFILES_CODE,
        }
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Credentials => f.write_str(CREDENTIALS_NAME),
            Self::Profiles => f.write_str(PROFILES_NAME),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProviderKindParserError(pub String);

impl std::error::Error for ProviderKindParserError {}

impl std::fmt::Display for ProviderKindParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid provider kind {:?}", self.0)
    }
}

impl FromStr for ProviderKind {
    type Err = ProviderKindParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            CREDENTIALS_NAME => Ok(Self::Credentials),
            PROFILES_NAME => Ok(Self::Profiles),
            other => Err(ProviderKindParserError(other.to_string())),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProviderKindDecoderError(pub u8);

impl std::error::Error for ProviderKindDecoderError {}

impl std::fmt::Display for ProviderKindDecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid provider kind {:?}", self.0)
    }
}

impl TryFrom<u8> for ProviderKind {
    type Error = ProviderKindDecoderError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            CREDENTIALS_CODE => Ok(Self::Credentials),
            PROFILES_CODE => Ok(Self::Profiles),
            other => Err(ProviderKindDecoderError(other)),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Entity {
    pub application_id: Uuid,
    pub kind: ProviderKind,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let kind: u8 = row.try_get(1)?;
        let kind = ProviderKind::try_from(kind).map_err(|err| sqlx::Error::ColumnDecode {
            index: "kind".into(),
            source: Box::new(err),
        })?;

        Ok(Self {
            application_id: row.try_get(0)?,
            kind,
        })
    }
}

pub(crate) struct Upsert {
    application_id: Uuid,
    kind: ProviderKind,
}

impl Upsert {
    pub fn new(application_id: Uuid, kind: ProviderKind) -> Self {
        Self {
            application_id,
            kind,
        }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Entity, sqlx::Error> {
        sqlx::query_as(
            r#"insert into providers (application_id, kind)
    values ($1, $2)
    on conflict (application_id, kind)
    do update set kind = excluded.kind
    returning application_id, kind"#,
        )
        .bind(self.application_id)
        .bind(self.kind.as_code())
        .fetch_one(executor)
        .await
    }
}

pub(crate) struct ListByApplication {
    application_id: Uuid,
}

impl ListByApplication {
    pub fn new(application_id: Uuid) -> Self {
        Self { application_id }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Vec<Entity>, sqlx::Error> {
        sqlx::query_as("select application_id, kind from providers where application_id = $1")
            .bind(self.application_id)
            .fetch_all(executor)
            .await
    }
}
