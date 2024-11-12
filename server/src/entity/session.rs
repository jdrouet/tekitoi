use std::time::Duration;

use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub access_token: String,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub scope: Option<String>,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        Ok(Self {
            access_token: row.try_get(0)?,
            client_id: row.try_get(1)?,
            user_id: row.try_get(2)?,
            scope: row.try_get(3)?,
        })
    }
}

pub struct Create<'a> {
    pub access_token: &'a str,
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub scope: Option<&'a str>,
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
            r#"insert into sessions (access_token, client_id, user_id, scope, created_at, valid_until)
values ($1, $2, $3, $4, $5, $6)
returning access_token, client_id, user_id, scope"#,
        )
        .bind(self.access_token)
        .bind(self.client_id)
        .bind(self.user_id)
        .bind(self.scope)
        .bind(now)
        .bind(until)
        .fetch_one(executor)
        .await
    }
}
