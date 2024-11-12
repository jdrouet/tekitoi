use std::collections::HashSet;

use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub id: Uuid,
    pub secrets: HashSet<String>,
    pub redirect_uri: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let secrets: String = row.try_get(1)?;

        Ok(Self {
            id: row.try_get(0)?,
            secrets: HashSet::from_iter(secrets.split(',').map(String::from)),
            redirect_uri: row.try_get(2)?,
        })
    }
}

pub struct Upsert<'a> {
    id: Uuid,
    secrets: &'a HashSet<String>,
    redirect_uri: &'a str,
}

impl<'a> Upsert<'a> {
    pub fn new(id: Uuid, secrets: &'a HashSet<String>, redirect_uri: &'a str) -> Self {
        Self {
            id,
            secrets,
            redirect_uri,
        }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Entity, sqlx::Error> {
        let mut secrets = self.secrets.iter().map(|v| v.as_str()).collect::<Vec<_>>();
        secrets.sort();
        let secrets = secrets.join(",");
        sqlx::query_as(
            r#"insert into applications (id, secrets, redirect_uri)
values ($1, $2, $3)
on conflict (id)
do update set secrets = excluded.secrets, redirect_uri = excluded.redirect_uri
returning id, secrets, redirect_uri"#,
        )
        .bind(self.id)
        .bind(&secrets)
        .bind(self.redirect_uri)
        .fetch_one(executor)
        .await
    }
}

pub(crate) struct FindById {
    id: Uuid,
}

impl FindById {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Option<Entity>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, secrets, redirect_uri
from applications
where id = $1
limit 1"#,
        )
        .bind(self.id)
        .fetch_optional(executor)
        .await
    }
}
