use std::ops::Add;

use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};
use url::Url;
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

// response_type=code
// client_id=
// code_challenge=
// code_challenge_method=
// state=
// redirect_uri=

// TODO add response_type with an enum
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IncomingRequest {
    pub id: Uuid,
    pub application_id: Uuid,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: String,
    pub redirect_uri: Url,
}

impl FromRow<'_, SqliteRow> for IncomingRequest {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let redirect_uri: String = row.try_get(5)?;

        Ok(Self {
            id: row.try_get(0)?,
            application_id: row.try_get(1)?,
            code_challenge: row.try_get(2)?,
            code_challenge_method: row.try_get(3)?,
            state: row.try_get(4)?,
            redirect_uri: Url::parse(&redirect_uri).expect("couldn't parse redirect uri"),
        })
    }
}

pub struct CreateIncomingRequest<'a> {
    pub application_id: Uuid,
    pub code_challenge: &'a str,
    pub code_challenge_method: &'a str,
    pub state: &'a str,
    pub redirect_uri: &'a Url,
}

impl<'a> CreateIncomingRequest<'a> {
    pub fn new(
        application_id: Uuid,
        code_challenge: &'a str,
        code_challenge_method: &'a str,
        state: &'a str,
        redirect_uri: &'a Url,
    ) -> Self {
        Self {
            application_id,
            code_challenge,
            code_challenge_method,
            state,
            redirect_uri,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Uuid, sqlx::Error> {
        let now = chrono::Utc::now();
        let expired = now.add(chrono::Duration::minutes(10));
        let id = Uuid::new_v4();

        sqlx::query_scalar(
            r#"insert into initial_requests (id, application_id, code_challenge, code_challenge_method, state, redirect_uri, created_at, expired_at)
values ($1, $2, $3, $4, $5, $6, $7, $8)
returning id"#,
        )
        .bind(id)
        .bind(self.application_id)
        .bind(self.code_challenge)
        .bind(self.code_challenge_method)
        .bind(self.state)
        .bind(self.redirect_uri.as_str())
        .bind(now.timestamp())
        .bind(expired.timestamp())
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Uuid, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct GetIncomingRequestById {
    request_id: Uuid,
}

impl GetIncomingRequestById {
    pub fn new(request_id: Uuid) -> Self {
        Self { request_id }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<IncomingRequest, sqlx::Error> {
        sqlx::query_as(
            r#"select id, application_id, code_challenge, code_challenge_method, state, redirect_uri
from initial_requests
where id = $1
limit 1"#,
        )
        .bind(self.request_id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<IncomingRequest, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
