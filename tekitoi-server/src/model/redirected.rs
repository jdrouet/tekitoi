use std::ops::Add;

use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};

use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct RedirectedRequest {
    pub id: Uuid,
    pub provider_authorization_request_id: Uuid,
    pub code: String,
}

impl FromRow<'_, SqliteRow> for RedirectedRequest {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            provider_authorization_request_id: row.try_get(1)?,
            code: row.try_get(2)?,
        })
    }
}

pub(crate) struct CreateRedirectedRequest<'a> {
    provider_authorization_request_id: Uuid,
    code: &'a str,
}

impl<'a> CreateRedirectedRequest<'a> {
    pub(crate) fn new(provider_authorization_request_id: Uuid, code: &'a str) -> Self {
        Self {
            provider_authorization_request_id,
            code,
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
            r#"insert into redirect_requests (id, provider_authorization_request_id, code, created_at, expired_at)
values ($1, $2, $3, $4, $5)
returning id"#,
        )
        .bind(id)
        .bind(self.provider_authorization_request_id)
        .bind(self.code)
        .bind(now.timestamp())
        .bind(expired.timestamp())
        .fetch_one(&mut **tx)
        .await
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Uuid, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct FindRedirectedRequestByCode<'a> {
    code: &'a str,
}

impl<'a> FindRedirectedRequestByCode<'a> {
    pub(crate) fn new(code: &'a str) -> Self {
        Self { code }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Option<RedirectedRequest>, sqlx::Error> {
        sqlx::query_as(
            r#"select redirect_requests.id, redirect_requests.provider_authorization_request_id, redirect_requests.code
from redirect_requests
join provider_authorization_requests on provider_authorization_requests.id = redirect_requests.provider_authorization_request_id
join application_authorization_requests on application_authorization_requests.id = provider_authorization_requests.application_authorization_request_id
where application_authorization_requests.code_challenge = $1
limit 1"#,
        )
        .bind(self.code)
        .fetch_optional(&mut **tx)
        .await
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Option<RedirectedRequest>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
