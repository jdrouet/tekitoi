use std::ops::Add;

use chrono::Utc;
use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProviderAuthorizationRequest {
    pub id: Uuid,
    pub application_authorization_request_id: Uuid,
    pub provider_id: Uuid,
    pub csrf_token: String,
    pub pkce_verifier: String,
}

impl FromRow<'_, SqliteRow> for ProviderAuthorizationRequest {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            application_authorization_request_id: row.try_get(1)?,
            provider_id: row.try_get(2)?,
            csrf_token: row.try_get(3)?,
            pkce_verifier: row.try_get(4)?,
        })
    }
}

#[derive(Debug)]
pub struct CreateProviderAuthorizationRequest<'a> {
    application_authorization_request_id: Uuid,
    provider_id: Uuid,
    csrf_token: &'a str,
    pkce_verifier: &'a str,
}

impl<'a> CreateProviderAuthorizationRequest<'a> {
    pub fn new(
        application_authorization_request_id: Uuid,
        provider_id: Uuid,
        csrf_token: &'a str,
        pkce_verifier: &'a str,
    ) -> Self {
        Self {
            application_authorization_request_id,
            provider_id,
            csrf_token,
            pkce_verifier,
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
            r#"insert into provider_authorization_requests (id, application_authorization_request_id, provider_id, csrf_token, pkce_verifier, created_at, expired_at)
values ($1, $2, $3, $4, $5, $6, $7)
returning id"#,
        )
        .bind(id)
        .bind(self.application_authorization_request_id)
        .bind(self.provider_id)
        .bind(self.csrf_token)
        .bind(self.pkce_verifier)
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

pub struct FindProviderAuthorizationRequestByState<'a> {
    state: &'a str,
}

impl<'a> FindProviderAuthorizationRequestByState<'a> {
    pub fn new(state: &'a str) -> Self {
        Self { state }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Option<ProviderAuthorizationRequest>, sqlx::Error> {
        let now = Utc::now().timestamp();
        sqlx::query_as(
            r#"select id, application_authorization_request_id, provider_id, csrf_token, pkce_verifier
from provider_authorization_requests
where csrf_token = $1 and deleted_at is null
limit 1"#,
        )
        .bind(self.state)
        .bind(now)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Option<ProviderAuthorizationRequest>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct GetProviderAuthorizationRequestById {
    id: Uuid,
}

impl GetProviderAuthorizationRequestById {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<ProviderAuthorizationRequest, sqlx::Error> {
        sqlx::query_as(
            r#"select id, application_authorization_request_id, provider_id, csrf_token, pkce_verifier
from provider_authorization_requests
where id = $1
limit 1"#,
        )
        .bind(self.id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<ProviderAuthorizationRequest, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
