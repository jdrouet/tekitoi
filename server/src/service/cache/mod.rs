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
    pub async fn remove(&self, key: &str) -> Option<String> {
        self.inner.remove(key).await
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: String, value: String) {
        self.inner.insert(key, value).await
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
