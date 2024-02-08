use crate::{
    model::{
        application::{DeleteOtherApplications, UpsertApplication},
        provider::{DeleteOtherProviders, UpsertProvider},
    },
    service::database::{DatabasePool, DatabaseTransaction},
};
use oauth2::{AuthUrl, ClientId, ClientSecret, Scope, TokenUrl};
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

use self::{
    github::GithubProviderConfig, gitlab::GitlabProviderConfig, google::GoogleProviderConfig,
    oauth::OauthProviderConfig,
};

pub mod github;
pub mod gitlab;
pub mod google;
pub mod oauth;

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct ApplicationCollectionConfig(pub(crate) HashMap<String, ApplicationConfig>);

impl ApplicationCollectionConfig {
    async fn delete_other_applications<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
    ) -> Result<(), sqlx::Error> {
        let names = self.0.keys().collect::<Vec<&String>>();
        DeleteOtherApplications::new(&names).execute(tx).await?;
        Ok(())
    }

    async fn upsert_applications<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
    ) -> Result<(), sqlx::Error> {
        for (name, app) in self.0.iter() {
            app.synchronize(tx, name.as_str()).await?;
        }

        Ok(())
    }

    pub(crate) async fn synchronize(&self, pool: &DatabasePool) -> Result<(), sqlx::Error> {
        tracing::debug!("synchronize application collection");
        let mut tx = pool.begin().await?;

        self.delete_other_applications(&mut tx).await?;
        self.upsert_applications(&mut tx).await?;

        tx.commit().await
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ApplicationConfig {
    #[serde(default)]
    pub label: Option<String>,
    pub redirect_uri: Url,
    pub client_id: String,
    pub client_secrets: Vec<String>,
    #[serde(default)]
    pub providers: ProviderCollectionConfig,
}

impl ApplicationConfig {
    pub async fn synchronize<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
        name: &str,
    ) -> Result<Uuid, sqlx::Error> {
        tracing::debug!("synchronize application name={name:?}");

        let application_id = UpsertApplication::new(
            name,
            self.label.as_deref(),
            &self.client_id,
            &self.client_secrets,
            &self.redirect_uri,
        )
        .execute(tx)
        .await?;

        self.providers.synchronize(tx, application_id).await?;

        Ok(application_id)
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct ProviderCollectionConfig(pub(crate) HashMap<String, ProviderConfig>);

impl ProviderCollectionConfig {
    async fn delete_other_providers<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
        application_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let names = self.0.keys().collect::<Vec<&String>>();
        DeleteOtherProviders::new(application_id, &names)
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn upsert_providers<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
        application_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        for (name, provider) in self.0.iter() {
            provider
                .synchronize(tx, application_id, name.as_str())
                .await?;
        }

        Ok(())
    }

    pub async fn synchronize<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
        application_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        tracing::debug!("synchronize provider collection");

        self.delete_other_providers(tx, application_id).await?;
        self.upsert_providers(tx, application_id).await?;

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(flatten)]
    pub inner: ProviderInnerConfig,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum ProviderInnerConfig {
    Github(GithubProviderConfig),
    Gitlab(GitlabProviderConfig),
    Google(GoogleProviderConfig),
    Oauth(OauthProviderConfig),
}

impl ProviderInnerConfig {
    pub fn oauth_scopes(&self) -> Vec<oauth2::Scope> {
        match self {
            Self::Github(inner) => inner.scopes.iter().map(|s| Scope::new(s.clone())).collect(),
            Self::Gitlab(inner) => inner.scopes.iter().map(|s| Scope::new(s.clone())).collect(),
            Self::Google(inner) => inner.scopes.iter().map(|s| Scope::new(s.clone())).collect(),
            Self::Oauth(inner) => inner.scopes.iter().map(|s| Scope::new(s.clone())).collect(),
        }
    }

    pub fn oauth_client(&self) -> oauth2::basic::BasicClient {
        oauth2::basic::BasicClient::new(
            self.client_id(),
            self.client_secret(),
            self.authorization_url(),
            self.token_url(),
        )
    }

    pub(crate) fn provider_client(self, access_token: String) -> Box<dyn ProviderClient> {
        match self {
            Self::Oauth(inner) => inner.provider_client(access_token),
            Self::Github(inner) => inner.provider_client(access_token),
            Self::Gitlab(inner) => inner.provider_client(access_token),
            Self::Google(inner) => inner.provider_client(access_token),
        }
    }

    fn client_id(&self) -> ClientId {
        ClientId::new(match self {
            Self::Oauth(inner) => inner.client_id.clone(),
            Self::Github(inner) => inner.client_id.clone(),
            Self::Gitlab(inner) => inner.client_id.clone(),
            Self::Google(inner) => inner.client_id.clone(),
        })
    }

    fn client_secret(&self) -> Option<ClientSecret> {
        Some(ClientSecret::new(match self {
            Self::Oauth(inner) => inner.client_secret.clone(),
            Self::Github(inner) => inner.client_secret.clone(),
            Self::Gitlab(inner) => inner.client_secret.clone(),
            Self::Google(inner) => inner.client_secret.clone(),
        }))
    }

    fn authorization_url(&self) -> AuthUrl {
        AuthUrl::from_url(match self {
            Self::Oauth(inner) => inner.authorization_url.clone(),
            Self::Github(inner) => inner.authorization_url.clone(),
            Self::Gitlab(inner) => inner.authorization_url.clone(),
            Self::Google(inner) => inner.authorization_url.clone(),
        })
    }

    fn token_url(&self) -> Option<TokenUrl> {
        Some(TokenUrl::from_url(match self {
            Self::Oauth(inner) => inner.token_url.clone(),
            Self::Github(inner) => inner.token_url.clone(),
            Self::Gitlab(inner) => inner.token_url.clone(),
            Self::Google(inner) => inner.token_url.clone(),
        }))
    }
}

impl ProviderConfig {
    pub async fn synchronize<'c>(
        &self,
        tx: &mut DatabaseTransaction<'c>,
        application_id: Uuid,
        name: &str,
    ) -> Result<Uuid, sqlx::Error> {
        tracing::debug!("synchronize provider name={name:?}");

        UpsertProvider::new(application_id, name, self.label.as_deref(), &self.inner)
            .execute(tx)
            .await
    }
}

#[axum::async_trait]
pub(crate) trait ProviderClient: std::fmt::Debug + Send {
    async fn fetch_user(&self) -> Result<ProviderUser, String>;
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "provider", rename_all = "kebab-case")]
pub enum ProviderUser {
    Github(github::GithubUser),
    Gitlab(gitlab::GitlabUser),
    Google(google::GoogleUser),
    Oauth(oauth::OauthUser),
}

impl From<github::GithubUser> for ProviderUser {
    fn from(value: github::GithubUser) -> Self {
        Self::Github(value)
    }
}

impl From<gitlab::GitlabUser> for ProviderUser {
    fn from(value: gitlab::GitlabUser) -> Self {
        Self::Gitlab(value)
    }
}

impl From<google::GoogleUser> for ProviderUser {
    fn from(value: google::GoogleUser) -> Self {
        Self::Google(value)
    }
}

impl From<oauth::OauthUser> for ProviderUser {
    fn from(value: oauth::OauthUser) -> Self {
        Self::Oauth(value)
    }
}
