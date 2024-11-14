use std::time::Duration;

use uuid::Uuid;

use super::code_challenge::CodeChallengeMethod;
use super::response_type::ResponseType;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub code: String,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub state: String,
    pub scope: Option<String>,
    pub code_challenge: String,
    pub code_challenge_method: CodeChallengeMethod, // S256
    pub response_type: ResponseType,                // code
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let code_challenge_method: u8 = row.try_get(6)?;
        let code_challenge_method =
            CodeChallengeMethod::try_from(code_challenge_method).map_err(|err| {
                sqlx::Error::ColumnDecode {
                    index: "code_challenge_method".into(),
                    source: Box::new(err),
                }
            })?;

        let response_type: u8 = row.try_get(7)?;
        let response_type =
            ResponseType::try_from(response_type).map_err(|err| sqlx::Error::ColumnDecode {
                index: "response_type".into(),
                source: Box::new(err),
            })?;

        Ok(Self {
            code: row.try_get(0)?,
            client_id: row.try_get(1)?,
            user_id: row.try_get(2)?,
            state: row.try_get(3)?,
            scope: row.try_get(4)?,
            code_challenge: row.try_get(5)?,
            code_challenge_method,
            response_type,
        })
    }
}

pub struct Create<'a> {
    pub code: &'a str,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub state: &'a str,
    pub scope: Option<&'a str>,
    pub code_challenge: &'a str,
    pub code_challenge_method: CodeChallengeMethod, // S256
    pub response_type: ResponseType,                // code
    pub time_to_live: Duration,
}

impl Create<'_> {
    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Entity, sqlx::Error> {
        let now = chrono::Utc::now();
        let until = now + self.time_to_live;
        sqlx::query_as(
            r#"insert into authorizations (code, client_id, user_id, state, scope, code_challenge, code_challenge_method, response_type, created_at, valid_until)
values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
returning code, client_id, user_id, state, scope, code_challenge, code_challenge_method, response_type"#,
        )
        .bind(self.code)
        .bind(self.client_id)
        .bind(self.user_id)
        .bind(self.state)
        .bind(self.scope)
        .bind(self.code_challenge)
        .bind(self.code_challenge_method.as_code())
        .bind(self.response_type.as_code())
        .bind(now)
        .bind(until)
        .fetch_one(executor)
        .await
    }
}

pub(crate) struct FindByCode<'a> {
    pub code: &'a str,
}

impl<'a> FindByCode<'a> {
    pub fn new(code: &'a str) -> Self {
        Self { code }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Option<Entity>, sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query_as(
            r#"select code, client_id, user_id, state, scope, code_challenge, code_challenge_method, response_type
from authorizations
where code = $1 and valid_until > $2
limit 1"#,
        )
        .bind(self.code)
        .bind(now)
        .fetch_optional(executor)
        .await
    }
}
