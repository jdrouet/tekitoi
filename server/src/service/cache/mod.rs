use std::time::Duration;

#[derive(Debug)]
pub(crate) enum Config {
    Memory {
        max_capacity: u64,
        // A cached entry will be expired after the specified duration past from insert.
        time_to_live: u64,
    },
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let max_capacity = crate::helper::parse_env_or("CACHE_MAX_CAPACITY", 100)?;
        let time_to_live = crate::helper::parse_env_or("CACHE_TIME_TO_LIVE", 60 * 10)?;

        Ok(Self::Memory {
            max_capacity,
            time_to_live,
        })
    }

    pub(crate) fn build(self) -> anyhow::Result<Client> {
        match self {
            Self::Memory {
                max_capacity,
                time_to_live,
            } => {
                let inner = moka::future::CacheBuilder::default()
                    .max_capacity(max_capacity)
                    .time_to_live(Duration::from_secs(time_to_live))
                    .build();
                Ok(Client { inner })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Client {
    inner: moka::future::Cache<String, String>,
}

impl Client {
    pub async fn remove<V: serde::de::DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.inner
            .remove(key)
            .await
            .and_then(|v| serde_urlencoded::from_str(v.as_str()).ok())
    }

    pub async fn get<V: serde::de::DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.inner
            .get(key)
            .await
            .and_then(|v| serde_urlencoded::from_str(v.as_str()).ok())
    }

    pub async fn insert<V: serde::Serialize>(&self, key: String, value: &V) {
        match serde_urlencoded::to_string(value) {
            Ok(encoded) => self.inner.insert(key, encoded).await,
            Err(err) => {
                tracing::error!(message = "unable to insert in cache", cause = %err);
            }
        }
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn test() -> Self {
        let inner = moka::future::CacheBuilder::default()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(10))
            .build();
        Client { inner }
    }
}
