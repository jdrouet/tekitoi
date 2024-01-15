use std::str::FromStr;

use chrono::Utc;
use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};
use url::Url;
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

#[derive(Clone, Copy, Debug)]
pub enum ProviderKind {
    Github,
    Gitlab,
    Google,
}

impl AsRef<str> for ProviderKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Google => "google",
        }
    }
}

impl FromStr for ProviderKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "github" => Ok(Self::Github),
            "gitlab" => Ok(Self::Gitlab),
            "google" => Ok(Self::Google),
            other => Err(format!("unexpected provider kind {other:?}")),
        }
    }
}

pub struct Provider {
    pub id: Uuid,
    pub application_id: Uuid,
    pub kind: ProviderKind,
    pub name: String,
    pub label: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: Url,
    pub token_url: Url,
    pub base_api_url: Url,
    pub scopes: Vec<String>,
}

impl Provider {
    pub fn label_or_name(&self) -> &str {
        self.label.as_deref().unwrap_or(self.name.as_str())
    }

    pub fn oauth_client(&self, base_url: &str) -> oauth2::basic::BasicClient {
        use oauth2::basic::BasicClient;
        use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

        let redirect_url = format!("{base_url}/api/redirect");
        let redirect_url = RedirectUrl::new(redirect_url).expect("unable to parse redirect url");

        BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::from_url(self.authorization_url.clone()),
            Some(TokenUrl::from_url(self.token_url.clone())),
        )
        .set_redirect_uri(redirect_url)
    }

    pub fn provider_client<'a>(
        &self,
        access_token: &'a str,
    ) -> crate::service::client::ProviderClient<'a> {
        match self.kind {
            ProviderKind::Github => crate::service::client::github::GithubProviderClient::new(
                access_token,
                self.base_api_url.clone(),
            )
            .into(),
            ProviderKind::Gitlab => crate::service::client::gitlab::GitlabProviderClient::new(
                access_token,
                self.base_api_url.clone(),
            )
            .into(),
            ProviderKind::Google => crate::service::client::gitlab::GitlabProviderClient::new(
                access_token,
                self.base_api_url.clone(),
            )
            .into(),
        }
    }
}

impl FromRow<'_, SqliteRow> for Provider {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let kind: String = row.try_get(2)?;
        let authorization_url: String = row.try_get(7)?;
        let token_url: String = row.try_get(8)?;
        let base_api_url: String = row.try_get(9)?;
        let scopes: serde_json::Value = row.try_get(10)?;

        Ok(Self {
            id: row.try_get(0)?,
            application_id: row.try_get(1)?,
            kind: ProviderKind::from_str(kind.as_str()).expect("invalid provider kind"),
            name: row.try_get(3)?,
            label: row.try_get(4)?,
            client_id: row.try_get(5)?,
            client_secret: row.try_get(6)?,
            authorization_url: Url::parse(&authorization_url).expect("invalid authorization url"),
            token_url: Url::parse(&token_url).expect("invalid token url"),
            base_api_url: Url::parse(&base_api_url).expect("invalid base api url"),
            scopes: serde_json::from_value(scopes).expect("couldn't dejsonify [String]"),
        })
    }
}

pub struct ListProviderByApplicationId {
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
            r#"select id, application_id, kind, name, label, client_id, client_secret, authorization_url, token_url, base_api_url, scopes
from providers
where application_id = $1"#,
        )
        .bind(self.application_id)
        .fetch_all(&mut **tx)
        .await
    }

    pub async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Vec<Provider>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct FindProviderForInitialRequest {
    initial_request_id: Uuid,
    provider_id: Uuid,
}

impl FindProviderForInitialRequest {
    pub fn new(initial_request_id: Uuid, provider_id: Uuid) -> Self {
        Self {
            initial_request_id,
            provider_id,
        }
    }

    async fn execute_sqlite(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<Option<Provider>, sqlx::Error> {
        sqlx::query_as(
            r#"select providers.id, providers.application_id, providers.kind, providers.name, providers.label, providers.client_id, providers.client_secret, providers.authorization_url, providers.token_url, providers.base_api_url, providers.scopes
from providers
join initial_requests on initial_requests.application_id = providers.application_id
where initial_requests.id = $1
    and providers.id = $2
limit 1"#,
        )
        .bind(self.initial_request_id)
        .bind(self.provider_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Option<Provider>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct GetProviderById {
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
            r#"select id, application_id, kind, name, label, client_id, client_secret, authorization_url, token_url, base_api_url, scopes
from providers
where providers.id = $1
limit 1"#,
        )
        .bind(self.id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Provider, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct GetProviderByAccessToken {
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
            r#"select providers.id, providers.application_id, providers.kind, providers.name, providers.label, providers.client_id, providers.client_secret, providers.authorization_url, providers.token_url, providers.base_api_url, providers.scopes
from providers
join local_requests on local_requests.provider_id = providers.id
join redirect_requests on redirect_requests.local_request_id = local_requests.id
join access_tokens on access_tokens.redirect_request_id = redirect_requests.id
where access_tokens.id = $1
limit 1"#,
        )
        .bind(self.token)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn execute(
        &self,
        executor: &mut DatabaseTransaction<'_>,
    ) -> Result<Provider, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub struct UpsertProvider<'a> {
    application_id: Uuid,
    kind: ProviderKind,
    name: &'a str,
    label: Option<&'a str>,

    client_id: &'a str,
    client_secret: &'a str,

    authorization_url: &'a Url,
    token_url: &'a Url,
    base_api_url: &'a Url,

    scopes: &'a [String],
}

impl<'a> UpsertProvider<'a> {
    pub fn new(
        application_id: Uuid,
        kind: ProviderKind,
        name: &'a str,
        label: Option<&'a str>,

        client_id: &'a str,
        client_secret: &'a str,

        authorization_url: &'a Url,
        token_url: &'a Url,
        base_api_url: &'a Url,

        scopes: &'a [String],
    ) -> Self {
        Self {
            application_id,
            kind,
            name,
            label,
            client_id,
            client_secret,
            authorization_url,
            token_url,
            base_api_url,
            scopes,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now().timestamp();

        let scopes = serde_json::to_value(self.scopes).expect("couldn't jsonify [String]");

        let provider_id = sqlx::query_scalar(r#"insert into providers (id, application_id, kind, name, label, client_id, client_secret, authorization_url, token_url, base_api_url, scopes, created_at, updated_at)
values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
on conflict (application_id, name) do update set
    kind = $3,
    label = $5,
    client_id = $6,
    client_secret = $7,
    authorization_url = $8,
    token_url = $9,
    base_api_url = $10,
    scopes = $11,
    updated_at = $12,
    deleted_at = null
returning id"#)
            .bind(id)
            .bind(self.application_id)
            .bind(self.kind.as_ref())
            .bind(self.name)
            .bind(self.label)
            .bind(self.client_id)
            .bind(self.client_secret)
            .bind(self.authorization_url.as_str())
            .bind(self.token_url.as_str())
            .bind(self.base_api_url.as_str())
            .bind(scopes)
            .bind(now)
            .fetch_one(&mut **tx)
            .await?;

        tracing::debug!("done");

        Ok(provider_id)
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

pub struct DeleteOtherProviders<'a> {
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

    pub async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<(), sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
