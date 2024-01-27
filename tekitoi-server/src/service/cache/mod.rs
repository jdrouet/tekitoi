use deadpool_redis::{redis::RedisError, PoolError};
use oauth2::TokenResponse;

use crate::entity::{
    incoming::IncomingAuthorizationRequest, local::LocalAuthorizationRequest,
    redirected::RedirectedAuthorizationRequest, token::ProviderAccessToken,
};

const CACHE_DURATION: i64 = 600; // 10 min

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CacheConfig {
    Redis(deadpool_redis::Config),
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::Redis(Default::default())
    }
}

impl CacheConfig {
    pub fn build(&self) -> CachePool {
        match self {
            Self::Redis(inner) => inner
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .map(CachePool::Redis)
                .expect("couldn't build cache pool"),
        }
    }
}

#[derive(Clone)]
pub enum CachePool {
    Redis(deadpool_redis::Pool),
}

impl CachePool {
    pub async fn acquire(&self) -> Result<CacheClient, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .get()
                .await
                .map(CacheClient::Redis)
                .map_err(CacheError::RedisPool),
        }
    }
}

#[derive(Debug)]
pub enum CacheError {
    RedisPool(PoolError),
    RedisClient(RedisError),
}

pub enum CacheClient {
    Redis(deadpool_redis::Connection),
}

impl CacheClient {
    #[inline]
    pub async fn insert_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
        req: IncomingAuthorizationRequest,
    ) -> Result<(), CacheError> {
        self.set_exp(csrf_token, &req, CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<IncomingAuthorizationRequest>, CacheError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_local_authorization_request(
        &mut self,
        csrf_token: &str,
        req: LocalAuthorizationRequest,
    ) -> Result<(), CacheError> {
        self.set_exp(csrf_token, &req, CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_local_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<LocalAuthorizationRequest>, CacheError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
        req: RedirectedAuthorizationRequest,
    ) -> Result<(), CacheError> {
        self.set_exp(csrf_token, &req, CACHE_DURATION).await
    }

    #[inline]
    pub async fn remove_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<RedirectedAuthorizationRequest>, CacheError> {
        self.remove(csrf_token).await
    }

    #[inline]
    pub async fn insert_provider_access_token(
        &mut self,
        token: &str,
        value: ProviderAccessToken,
    ) -> Result<(), CacheError> {
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
    ) -> Result<Option<ProviderAccessToken>, CacheError> {
        self.find(token).await
    }

    async fn find<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, CacheError> {
        match self {
            Self::Redis(inner) => {
                let value: Option<String> = deadpool_redis::redis::cmd("GET")
                    .arg(key)
                    .query_async(inner)
                    .await
                    .map_err(CacheError::RedisClient)?;
                Ok(value.and_then(|v| serde_qs::from_str(v.as_str()).ok()))
            }
        }
    }

    async fn remove<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, CacheError> {
        match self {
            Self::Redis(inner) => {
                let value: Option<String> = deadpool_redis::redis::cmd("GETDEL")
                    .arg(key)
                    .query_async(inner)
                    .await
                    .map_err(CacheError::RedisClient)?;
                Ok(value.and_then(|v| serde_qs::from_str(v.as_str()).ok()))
            }
        }
    }

    async fn set<T: serde::Serialize>(&mut self, key: &str, value: &T) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => {
                let value = serde_qs::to_string(value).unwrap();
                deadpool_redis::redis::cmd("SET")
                    .arg(key)
                    .arg(value)
                    .query_async(inner)
                    .await
                    .map_err(CacheError::RedisClient)
            }
        }
    }

    async fn set_exp<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: &T,
        duration: i64,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => {
                let value = serde_qs::to_string(value).unwrap();
                deadpool_redis::redis::cmd("SETEX")
                    .arg(key)
                    .arg(duration)
                    .arg(value)
                    .query_async(inner)
                    .await
                    .map_err(CacheError::RedisClient)
            }
        }
    }
}
