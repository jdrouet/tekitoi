use chrono::Utc;
use oauth2::ClientId;
use sqlx::{sqlite::SqliteRow, FromRow, Row, Sqlite, Transaction};
use url::Url;
use uuid::Uuid;

use crate::service::database::DatabaseTransaction;

pub(crate) struct Application {
    pub id: Uuid,
    pub client_id: ClientId,
    pub client_secrets: Vec<String>,
    pub name: String,
    pub label: Option<String>,
    pub redirect_uri: Url,
}

impl Application {
    pub(crate) fn label_or_name(&self) -> &str {
        self.label.as_deref().unwrap_or(self.name.as_str())
    }

    pub(crate) fn is_redirect_uri_matching(&self, url: &Url) -> bool {
        &self.redirect_uri == url
    }
}

impl FromRow<'_, SqliteRow> for Application {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        let client_id: String = row.try_get(1)?;
        let client_secrets: serde_json::Value = row.try_get(2)?;
        let redirect_url: String = row.try_get(5)?;

        Ok(Self {
            id: row.try_get(0)?,
            client_id: ClientId::new(client_id),
            client_secrets: serde_json::from_value(client_secrets)
                .expect("couldn't dejsonify [String]"),
            name: row.try_get(3)?,
            label: row.try_get(4)?,
            redirect_uri: Url::parse(&redirect_url).expect("couldn't parse url"),
        })
    }
}

pub(crate) struct FindApplicationByClientId<'a> {
    client_id: &'a str,
}

impl<'a> FindApplicationByClientId<'a> {
    pub(crate) fn new(client_id: &'a str) -> Self {
        Self { client_id }
    }
}

impl<'a> FindApplicationByClientId<'a> {
    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Option<Application>, sqlx::Error> {
        sqlx::query_as(
            r#"select id, client_id, client_secrets, name, label, redirect_uri
from applications
where client_id = $1
limit 1"#,
        )
        .bind(self.client_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Option<Application>, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct UpsertApplication<'a> {
    name: &'a str,
    label: Option<&'a str>,
    client_id: &'a str,
    client_secrets: &'a [String],
    redirect_uri: &'a Url,
}

impl<'a> UpsertApplication<'a> {
    pub(crate) fn new(
        name: &'a str,
        label: Option<&'a str>,
        client_id: &'a str,
        client_secrets: &'a [String],
        redirect_uri: &'a Url,
    ) -> Self {
        Self {
            name,
            label,
            client_id,
            client_secrets,
            redirect_uri,
        }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now().timestamp();

        let secrets =
            serde_json::to_value(self.client_secrets).expect("unable to jsonify [String]");

        sqlx::query_scalar(
            r#"insert into applications (id, name, label, client_id, client_secrets, redirect_uri, created_at, updated_at)
values ($1, $2, $3, $4, $5, $6, $7, $7)
on conflict (name) do update set
    label = $3,
    client_id = $4,
    client_secrets = $5,
    redirect_uri = $6,
    updated_at = $7,
    deleted_at = null
returning id"#,
        )
        .bind(id)
        .bind(self.name)
        .bind(self.label)
        .bind(self.client_id)
        .bind(secrets)
        .bind(self.redirect_uri.to_string().as_str())
        .bind(now)
        .fetch_one(&mut **tx)
        .await
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<Uuid, sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}

pub(crate) struct DeleteOtherApplications<'a> {
    names: &'a [&'a String],
}

impl<'a> DeleteOtherApplications<'a> {
    pub(crate) fn new(names: &'a [&'a String]) -> Self {
        Self { names }
    }

    async fn execute_sqlite<'c>(
        &self,
        tx: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();
        let names = serde_json::to_value(self.names).expect("couldn't jsonify [String]");

        sqlx::query(
            r#"update applications
set deleted_at = $1
where name not in (select $2)"#,
        )
        .bind(now)
        .bind(&names)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn execute<'c>(
        &self,
        executor: &mut DatabaseTransaction<'c>,
    ) -> Result<(), sqlx::Error> {
        match executor {
            DatabaseTransaction::Sqlite(inner) => self.execute_sqlite(inner).await,
        }
    }
}
