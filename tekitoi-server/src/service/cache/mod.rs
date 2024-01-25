use deadpool_redis::{redis::RedisError, PoolError};

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

pub enum CacheError {
    RedisPool(PoolError),
    RedisClient(RedisError),
}

pub enum CacheClient {
    Redis(deadpool_redis::Connection),
}

impl CacheClient {
    pub async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        match self {
            Self::Redis(inner) => deadpool_redis::redis::cmd("GET")
                .arg(key)
                .query_async(inner)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    pub async fn remove(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        match self {
            Self::Redis(inner) => deadpool_redis::redis::cmd("GETDEL")
                .arg(key)
                .query_async(inner)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    pub async fn set(&mut self, key: &str, value: &str) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => deadpool_redis::redis::cmd("SET")
                .arg(key)
                .arg(value)
                .query_async(inner)
                .await
                .map_err(CacheError::RedisClient),
        }
    }

    pub async fn set_exp(
        &mut self,
        key: &str,
        value: &str,
        duration: i64,
    ) -> Result<(), CacheError> {
        match self {
            Self::Redis(inner) => deadpool_redis::redis::cmd("SETEX")
                .arg(key)
                .arg(duration)
                .arg(value)
                .query_async(inner)
                .await
                .map_err(CacheError::RedisClient),
        }
    }
}
