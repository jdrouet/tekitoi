use std::ops::Add;

use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};

use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RedirectedRequest {
    pub id: Uuid,
    pub local_request_id: Uuid,
    pub code: String,
}

impl FromRow<'_, SqliteRow> for RedirectedRequest {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            local_request_id: row.try_get(1)?,
            code: row.try_get(2)?,
        })
    }
}

pub struct CreateRedirectedRequest<'a> {
    local_request_id: Uuid,
    code: &'a str,
}

impl<'a> CreateRedirectedRequest<'a> {
    pub fn new(local_request_id: Uuid, code: &'a str) -> Self {
        Self {
            local_request_id,
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
            r#"insert into redirect_requests (id, local_request_id, code, created_at, expired_at)
values ($1, $2, $3, $4, $5)
returning id"#,
        )
        .bind(id)
        .bind(self.local_request_id)
        .bind(self.code)
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

pub struct FindRedirectedRequestByCode<'a> {
    code: &'a str,
}

impl<'a> FindRedirectedRequestByCode<'a> {
    pub fn new(code: &'a str) -> Self {
        Self { code }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Option<RedirectedRequest>, sqlx::Error> {
        sqlx::query_as(
            r#"select redirect_requests.id, redirect_requests.local_request_id, redirect_requests.code
from redirect_requests
join local_requests on local_requests.id = redirect_requests.local_request_id
join initial_requests on initial_requests.id = local_requests.initial_request_id
where initial_requests.code_challenge = $1
limit 1"#,
        )
        .bind(self.code)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Option<RedirectedRequest>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
