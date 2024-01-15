use std::ops::Add;

use chrono::Duration;
use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProviderAccessToken {
    pub id: Uuid,
    pub redirect_request_id: Uuid,
    pub token: String,
}

impl FromRow<'_, SqliteRow> for ProviderAccessToken {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            redirect_request_id: row.try_get(1)?,
            token: row.try_get(2)?,
        })
    }
}

pub struct CreateAccessToken<'a> {
    redirect_request_id: Uuid,
    token: &'a str,
    duration: Option<Duration>,
}

impl<'a> CreateAccessToken<'a> {
    pub fn new(redirect_request_id: Uuid, token: &'a str, duration: Option<Duration>) -> Self {
        Self {
            redirect_request_id,
            token,
            duration,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Uuid, sqlx::Error> {
        let now = chrono::Utc::now();
        let expires_at = self.duration.map(|dur| now.add(dur).timestamp());
        let id = Uuid::new_v4();

        sqlx::query_scalar(
            r#"insert into access_tokens (id, redirect_request_id, token, created_at, expired_at)
values ($1, $2, $3, $4, $5)
returning id"#,
        )
        .bind(id)
        .bind(self.redirect_request_id)
        .bind(self.token)
        .bind(now.timestamp())
        .bind(expires_at)
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

pub struct FindAccessToken {
    token: Uuid,
}

impl FindAccessToken {
    pub fn new(token: Uuid) -> Self {
        Self { token }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Option<ProviderAccessToken>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, redirect_request_id, token
from access_tokens
where id = $1
limit 1"#,
        )
        .bind(self.token)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Option<ProviderAccessToken>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
