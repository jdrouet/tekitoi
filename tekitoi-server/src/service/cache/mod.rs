use deadpool_redis::{redis::RedisError, PoolError};

use crate::entity::{
    incoming::IncomingAuthorizationRequest, local::LocalAuthorizationRequest,
    redirected::RedirectedAuthorizationRequest, token::ProviderAccessToken,
};

mod redis;

const CACHE_DURATION: i64 = 600; // 10 min

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CacheConfig {
    Redis(redis::Config),
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::Redis(Default::default())
    }
}

impl CacheConfig {
    pub fn build(&self) -> CachePool {
        match self {
            Self::Redis(inner) => CachePool::Redis(inner.build()),
        }
    }
}

#[derive(Clone)]
pub enum CachePool {
    Redis(redis::RedisPool),
}

impl CachePool {
    pub async fn acquire(&self) -> Result<CacheClient, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .acquire()
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
    Redis(redis::RedisClient),
}

impl CacheClient {
    #[inline]
    pub async fn insert_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
        req: IncomingAuthorizationRequest,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => inner
                .insert_incoming_authorization_request(csrf_token, req)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn remove_incoming_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<IncomingAuthorizationRequest>, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .remove_incoming_authorization_request(csrf_token)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn insert_local_authorization_request(
        &mut self,
        csrf_token: &str,
        req: LocalAuthorizationRequest,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => inner
                .insert_local_authorization_request(csrf_token, req)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn remove_local_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<LocalAuthorizationRequest>, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .remove_local_authorization_request(csrf_token)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn insert_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
        req: RedirectedAuthorizationRequest,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => inner
                .insert_redirected_authorization_request(csrf_token, req)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn remove_redirected_authorization_request(
        &mut self,
        csrf_token: &str,
    ) -> Result<Option<RedirectedAuthorizationRequest>, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .remove_redirected_authorization_request(csrf_token)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn insert_provider_access_token(
        &mut self,
        token: &str,
        value: ProviderAccessToken,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => inner
                .insert_provider_access_token(token, value)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    #[inline]
    pub async fn find_provider_access_token(
        &mut self,
        token: &str,
    ) -> Result<Option<ProviderAccessToken>, CacheError> {
        match self {
            Self::Redis(inner) => inner
                .find_provider_access_token(token)
                .await
                .map_err(CacheError::RedisClient),
        }
    }
}
