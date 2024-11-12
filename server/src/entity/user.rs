use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub id: Uuid,
    pub login: String,
    pub email: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Entity {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        Ok(Self {
            id: row.try_get(0)?,
            login: row.try_get(1)?,
            email: row.try_get(2)?,
        })
    }
}

pub struct Upsert<'a> {
    id: Uuid,
    application_id: Uuid,
    login: &'a str,
    email: &'a str,
}

impl<'a> Upsert<'a> {
    pub fn new(id: Uuid, application_id: Uuid, login: &'a str, email: &'a str) -> Self {
        Self {
            id,
            application_id,
            login,
            email,
        }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Entity, sqlx::Error> {
        sqlx::query_as(
            r#"insert into users (id, application_id, login, email)
values ($1, $2, $3, $4)
on conflict (id)
do update set login = excluded.login, email = excluded.email
returning id, login, email"#,
        )
        .bind(self.id)
        .bind(self.application_id)
        .bind(self.login)
        .bind(self.email)
        .fetch_one(executor)
        .await
    }
}

pub struct FindByAccessToken<'a> {
    access_token: &'a str,
}

impl<'a> FindByAccessToken<'a> {
    pub fn new(access_token: &'a str) -> Self {
        Self { access_token }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Option<Entity>, sqlx::Error> {
        let now = chrono::Utc::now();
        sqlx::query_as(
            r#"select users.id, users.login, users.email
from users
join sessions on sessions.user_id = users.id
where sessions.access_token = $1 and sessions.valid_until > $2
limit 1"#,
        )
        .bind(self.access_token)
        .bind(now)
        .fetch_optional(executor)
        .await
    }
}

pub struct FindById {
    id: Uuid,
    application_id: Uuid,
}

impl FindById {
    pub fn new(id: Uuid, application_id: Uuid) -> Self {
        Self { id, application_id }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Option<Entity>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, login, email from users where id = $1 and application_id = $2 limit 1"#,
        )
        .bind(self.id)
        .bind(self.application_id)
        .fetch_optional(executor)
        .await
    }
}

pub struct ListForApplication {
    application_id: Uuid,
}

impl ListForApplication {
    pub fn new(application_id: Uuid) -> Self {
        Self { application_id }
    }

    pub async fn execute<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
        &self,
        executor: E,
    ) -> Result<Vec<Entity>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, login, email from users where application_id = $1 order by login"#,
        )
        .bind(self.application_id)
        .fetch_all(executor)
        .await
    }
}
