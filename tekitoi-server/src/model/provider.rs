use chrono::Utc;
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite, Transaction};
use url::Url;
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

pub(crate) struct Provider {
    pub id: Uuid,
    #[allow(unused)]
    pub application_id: Uuid,
    pub name: String,
    pub label: Option<String>,
    pub config: crate::service::client::ProviderInnerConfig,
}

impl Provider {
    pub fn label_or_name(&self) -> &str {
        self.label.as_deref().unwrap_or(self.name.as_str())
    }

    pub fn oauth_client(&self, base_url: &str) -> oauth2::basic::BasicClient {
        use oauth2::RedirectUrl;

        let redirect_url = format!("{base_url}/api/redirect");
        let redirect_url = RedirectUrl::new(redirect_url).expect("unable to parse redirect url");

        self.config.oauth_client().set_redirect_uri(redirect_url)
    }

    pub fn oauth_authorization_request(
        &self,
        base_url: &str,
    ) -> (Url, CsrfToken, PkceCodeVerifier) {
        // Generate a PKCE challenge.
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        // Generate the full authorization URL.
        let client = self.oauth_client(base_url);
        let auth_request = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.config.oauth_scopes().into_iter());
        let (auth_url, csrf_token) = auth_request.set_pkce_challenge(pkce_challenge).url();

        (auth_url, csrf_token, pkce_verifier)
    }
}

impl FromRow<'_, SqliteRow> for Provider {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let config: serde_json::Value = row.try_get(4)?;
        let config: crate::service::client::ProviderInnerConfig =
            serde_json::from_value(config).expect("couldn't decode json object");

        Ok(Self {
            id: row.try_get(0)?,
            application_id: row.try_get(1)?,
            name: row.try_get(2)?,
            label: row.try_get(3)?,
            config,
        })
    }
}

pub(crate) struct ListProviderByApplicationId {
    application_id: Uuid,
}

impl ListProviderByApplicationId {
    pub fn new(application_id: Uuid) -> Self {
        Self { application_id }
    }

    async fn execute_sqlite(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<Vec<Provider>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, application_id, name, label, config
from providers
where application_id = $1"#,
        )
        .bind(self.application_id)
        .fetch_all(&mut **tx)
        .await
    }

    pub(crate) async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Vec<Provider>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct FindProviderForApplicationAuthorizationRequest {
    application_authorization_request_id: Uuid,
    provider_id: Uuid,
}

impl FindProviderForApplicationAuthorizationRequest {
    pub fn new(application_authorization_request_id: Uuid, provider_id: Uuid) -> Self {
        Self {
            application_authorization_request_id,
            provider_id,
        }
    }

    async fn execute_sqlite(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<Option<Provider>, sqlx::Error> {
        sqlx::query_as(
            r#"select providers.id, providers.application_id, providers.name, providers.label, providers.config
from providers
join application_authorization_requests on application_authorization_requests.application_id = providers.application_id
where application_authorization_requests.id = $1
    and providers.id = $2
limit 1"#,
        )
        .bind(self.application_authorization_request_id)
        .bind(self.provider_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub(crate) async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Option<Provider>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct GetProviderById {
    id: Uuid,
}

impl GetProviderById {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    async fn execute_sqlite(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<Provider, sqlx::Error> {
        sqlx::query_as(
            r#"select id, application_id, name, label, config
from providers
where providers.id = $1
limit 1"#,
        )
        .bind(self.id)
        .fetch_one(&mut **tx)
        .await
    }

    pub(crate) async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Provider, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct GetProviderByAccessToken {
    token: Uuid,
}

impl GetProviderByAccessToken {
    pub fn new(token: Uuid) -> Self {
        Self { token }
    }

    async fn execute_sqlite(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<Provider, sqlx::Error> {
        sqlx::query_as(
            r#"select providers.id, providers.application_id, providers.name, providers.label, providers.config
from providers
join provider_authorization_requests on provider_authorization_requests.provider_id = providers.id
join redirect_requests on redirect_requests.provider_authorization_request_id = provider_authorization_requests.id
join access_tokens on access_tokens.redirect_request_id = redirect_requests.id
where access_tokens.id = $1
limit 1"#,
        )
        .bind(self.token)
        .fetch_one(&mut **tx)
        .await
    }

    pub(crate) async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Provider, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct UpsertProvider<'a> {
    application_id: Uuid,
    name: &'a str,
    label: Option<&'a str>,

    config: &'a crate::service::client::ProviderInnerConfig,
}

impl<'a> UpsertProvider<'a> {
    pub(crate) fn new(
        application_id: Uuid,
        name: &'a str,
        label: Option<&'a str>,

        config: &'a crate::service::client::ProviderInnerConfig,
    ) -> Self {
        Self {
            application_id,
            name,
            label,
            config,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now().timestamp();

        let config =
            serde_json::to_value(&self.config).expect("couldn't jsonify oauth configuration");

        let provider_id = sqlx::query_scalar(
            r#"insert into providers (id, application_id, name, label, config, created_at, updated_at)
values ($1, $2, $3, $4, $5, $6, $6)
on conflict (application_id, name) do update set
    label = $4,
    config = $5,
    updated_at = $6,
    deleted_at = null
returning id"#,
        )
        .bind(id)
        .bind(self.application_id)
        .bind(self.name)
        .bind(self.label)
        .bind(config)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;

        tracing::debug!("done");

        Ok(provider_id)
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

pub(crate) struct DeleteOtherProviders<'a> {
    application_id: Uuid,
    names: &'a [&'a String],
}

impl<'a> DeleteOtherProviders<'a> {
    pub fn new(application_id: Uuid, names: &'a [&'a String]) -> Self {
        Self {
            application_id,
            names,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();
        let names = serde_json::to_value(self.names).expect("couldn't jsonify [String]");

        sqlx::query(
            r#"update providers
set deleted_at = $1
where application_id = $2 and name not in (select value from json_each($3))"#,
        )
        .bind(now)
        .bind(self.application_id)
        .bind(&names)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<(), sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
