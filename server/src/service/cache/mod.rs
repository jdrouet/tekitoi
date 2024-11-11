use std::time::Duration;

mod memory;

#[derive(Debug)]
pub(crate) enum Config {
    Memory(memory::Config),
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        match crate::helper::from_env_or("CACHE_KIND", "memory").as_ref() {
            "memory" => memory::Config::from_env().map(Config::Memory),
            other => Err(anyhow::anyhow!("unknown cache kind {other:?}")),
        }
    }

    pub(crate) fn build(self) -> anyhow::Result<Client> {
        match self {
            Self::Memory(inner) => inner.build().map(Client::Memory),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Client {
    Memory(memory::Client),
}

impl Client {
    pub async fn remove<V: serde::de::DeserializeOwned>(&self, key: &str) -> Option<V> {
        match self {
            Self::Memory(inner) => inner.remove(key).await,
        }
    }

    pub async fn get<V: serde::de::DeserializeOwned>(&self, key: &str) -> Option<V> {
        match self {
            Self::Memory(inner) => inner.get(key).await,
        }
    }

    pub async fn insert<V: serde::Serialize>(&self, key: String, value: &V, ttl: Duration) {
        match self {
            Self::Memory(inner) => inner.insert(key, value, ttl).await,
        }
    }
}

#[cfg(test)]
impl Client {
    pub(crate) fn test() -> Self {
        Client::Memory(memory::Client::test())
    }
}
