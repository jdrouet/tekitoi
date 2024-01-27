use crate::entity::{
    incoming::IncomingAuthorizationRequest, local::LocalAuthorizationRequest,
    redirected::RedirectedAuthorizationRequest, token::ProviderAccessToken,
};
use deadpool_redis::redis::RedisError;
use deadpool_redis::PoolError;
use oauth2::TokenResponse;

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config(deadpool_redis::Config);

impl Config {
    pub fn build(&self) -> RedisPool {
        RedisPool(
            self.0
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("couldn't build cache pool"),
        )
    }
}

#[derive(Clone)]
pub struct RedisPool(deadpool_redis::Pool);

impl RedisPool {
    pub async fn acquire(&self) -> Result<RedisClient, PoolError> {
        self.0.get().await.map(RedisClient)
    }
}

pub struct RedisClient(deadpool_redis::Connection);

impl RedisClient {
    #[inline]
    pub async fn insert_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
        req: IncomingAuthorizationRequest,
    ) -> Result<(), RedisError> {
        self.set_exp(csrf_token, &req, super::CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<IncomingAuthorizationRequest>, RedisError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_local_authorization_request(
        &mut self,
        csrf_token: &str,
        req: LocalAuthorizationRequest,
    ) -> Result<(), RedisError> {
        self.set_exp(csrf_token, &req, super::CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_local_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<LocalAuthorizationRequest>, RedisError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
        req: RedirectedAuthorizationRequest,
    ) -> Result<(), RedisError> {
        self.set_exp(csrf_token, &req, super::CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<RedirectedAuthorizationRequest>, RedisError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_provider_access_token(
        &mut self,
        token: &str,
        value: ProviderAccessToken,
    ) -> Result<(), RedisError> {
        if let Some(exp) = value.inner.expires_in() {
            self.set_exp(token, &value, exp.as_secs() as i64).await
        } else {
            self.set(token, &value).await
        }
    }

    #[inline]
    pub async fn find_provider_access_token(
        &mut self,
        token: &str,
    ) -> Result<Option<ProviderAccessToken>, RedisError> {
        self.find(token).await
    }

    async fn find<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, RedisError> {
        let value: Option<String> = deadpool_redis::redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.0)
            .await?;
        Ok(value.and_then(|v| serde_qs::from_str(v.as_str()).ok()))
    }

    async fn remove<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, RedisError> {
        let value: Option<String> = deadpool_redis::redis::cmd("GETDEL")
            .arg(key)
            .query_async(&mut self.0)
            .await?;
        Ok(value.and_then(|v| serde_qs::from_str(v.as_str()).ok()))
    }

    async fn set<T: serde::Serialize>(&mut self, key: &str, value: &T) -> Result<(), RedisError> {
        let value = serde_qs::to_string(value).unwrap();
        deadpool_redis::redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async(&mut self.0)
            .await
    }

    async fn set_exp<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: &T,
        duration: i64,
    ) -> Result<(), RedisError> {
        let value = serde_qs::to_string(value).unwrap();
        deadpool_redis::redis::cmd("SETEX")
            .arg(key)
            .arg(duration)
            .arg(value)
            .query_async(&mut self.0)
            .await
    }
}
