pub mod sqlite;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DatabaseConfig {
    Sqlite(sqlite::Config),
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::Sqlite(Default::default())
    }
}

impl DatabaseConfig {
    pub async fn build(&self) -> Result<DatabasePool, Box<dyn std::error::Error>> {
        Ok(match self {
            Self::Sqlite(inner) => inner.build().await.map(DatabasePool::Sqlite)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum DatabasePool {
    Sqlite(sqlx::SqlitePool),
}

impl DatabasePool {
    pub async fn begin<'c>(&self) -> Result<DatabaseTransaction<'c>, sqlx::Error> {
        match self {
            Self::Sqlite(inner) => inner.begin().await.map(DatabaseTransaction::Sqlite),
        }
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        match self {
            Self::Sqlite(inner) => sqlx::migrate!("./migrations/sqlite").run(inner).await,
        }
    }
}

#[derive(Debug)]
pub enum DatabaseTransaction<'c> {
    Sqlite(sqlx::Transaction<'c, sqlx::Sqlite>),
}

impl<'c> DatabaseTransaction<'c> {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        match self {
            Self::Sqlite(inner) => inner.commit().await,
        }
    }
}
