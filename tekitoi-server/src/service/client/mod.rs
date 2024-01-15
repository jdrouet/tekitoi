use crate::{
    model::{
        application::{DeleteOtherApplications, UpsertApplication},
        provider::{DeleteOtherProviders, ProviderKind, UpsertProvider},
    },
    service::database::{DatabasePool, DatabaseTransaction},
};
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

use self::{
    github::GithubProviderConfig, gitlab::GitlabProviderConfig, google::GoogleProviderConfig,
};

pub mod github;
pub mod gitlab;
pub mod google;

#[derive(Debug, Default, serde::Deserialize)]
pub struct ApplicationCollectionConfig(HashMap<String, ApplicationConfig>);

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

    pub async fn synchronize(&self, pool: &DatabasePool) -> Result<(), sqlx::Error> {
        tracing::debug!("synchronize application collection");
        let mut tx = pool.begin().await?;

        self.delete_other_applications(&mut tx).await?;
        self.upsert_applications(&mut tx).await?;

        tx.commit().await
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplicationConfig {
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
pub struct ProviderCollectionConfig(HashMap<String, ProviderConfig>);

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
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(flatten)]
    pub inner: ProviderInnerConfig,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderInnerConfig {
    Github(GithubProviderConfig),
    Gitlab(GitlabProviderConfig),
    Google(GoogleProviderConfig),
}

impl ProviderInnerConfig {
    fn kind(&self) -> ProviderKind {
        match self {
            Self::Github(_) => ProviderKind::Github,
            Self::Gitlab(_) => ProviderKind::Gitlab,
            Self::Google(_) => ProviderKind::Google,
        }
    }

    fn authorization_url(&self) -> &Url {
        match self {
            Self::Github(inner) => &inner.authorization_url,
            Self::Gitlab(inner) => &inner.authorization_url,
            Self::Google(inner) => &inner.authorization_url,
        }
    }

    fn token_url(&self) -> &Url {
        match self {
            Self::Github(inner) => &inner.token_url,
            Self::Gitlab(inner) => &inner.token_url,
            Self::Google(inner) => &inner.token_url,
        }
    }

    fn base_api_url(&self) -> &Url {
        match self {
            Self::Github(inner) => &inner.base_api_url,
            Self::Gitlab(inner) => &inner.base_api_url,
            Self::Google(inner) => &inner.base_api_url,
        }
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

        UpsertProvider::new(
            application_id,
            self.inner.kind(),
            name,
            self.label.as_deref(),
            &self.client_id,
            &self.client_secret,
            self.inner.authorization_url(),
            self.inner.token_url(),
            self.inner.base_api_url(),
            &self.scopes,
        )
        .execute(tx)
        .await
    }
}

#[derive(Debug)]
pub enum ProviderClient<'a> {
    Github(github::GithubProviderClient<'a>),
    Gitlab(gitlab::GitlabProviderClient<'a>),
    Google(google::GoogleProviderClient<'a>),
}

impl<'a> From<github::GithubProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: github::GithubProviderClient<'a>) -> Self {
        Self::Github(value)
    }
}

impl<'a> From<gitlab::GitlabProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: gitlab::GitlabProviderClient<'a>) -> Self {
        Self::Gitlab(value)
    }
}

impl<'a> From<google::GoogleProviderClient<'a>> for ProviderClient<'a> {
    fn from(value: google::GoogleProviderClient<'a>) -> Self {
        Self::Google(value)
    }
}

impl<'a> ProviderClient<'a> {
    pub async fn fetch_user(&self) -> Result<ProviderUser, String> {
        match self {
            Self::Github(client) => client.fetch_user().await.map(Into::into),
            Self::Gitlab(client) => client.fetch_user().await.map(Into::into),
            Self::Google(client) => client.fetch_user().await.map(Into::into),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "provider", rename_all = "kebab-case")]
pub enum ProviderUser {
    Github(github::GithubUser),
    Gitlab(gitlab::GitlabUser),
    Google(google::GoogleUser),
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
