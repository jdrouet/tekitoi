use std::time::Duration;

use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub code: String,
    pub state: String,
    pub scope: Option<String>,
    pub client_id: Uuid,
    pub user_id: Uuid,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        Ok(Self {
            code: row.try_get(0)?,
            client_id: row.try_get(1)?,
            user_id: row.try_get(2)?,
            state: row.try_get(3)?,
            scope: row.try_get(4)?,
        })
    }
}

pub struct Create<'a> {
    pub code: &'a str,
    pub state: &'a str,
    pub scope: Option<&'a str>,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub time_to_live: Duration,
}

impl Create<'_> {
    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Entity, sqlx::Error> {
        let now = chrono::Utc::now();
        let until = now + self.time_to_live;
        sqlx::query_as(
            r#"insert into authorizations (code, client_id, user_id, state, scope, created_at, valid_until)
values ($1, $2, $3, $4, $5, $6, $7)
returning code, client_id, user_id, state, scope"#,
        )
        .bind(self.code)
        .bind(self.client_id)
        .bind(self.user_id)
        .bind(self.state)
        .bind(self.scope)
        .bind(now)
        .bind(until)
        .fetch_one(executor)
        .await
    }
}

pub(crate) struct FindByCode<'a> {
    pub code: &'a str,
}

impl<'a> FindByCode<'a> {
    pub fn new(code: &'a str) -> Self {
        Self { code }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Option<Entity>, sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query_as(
            r#"select code, client_id, user_id, state, scope
from authorizations
where code = $1 and valid_until > $2
limit 1"#,
        )
        .bind(self.code)
        .bind(now)
        .fetch_optional(executor)
        .await
    }
}
