use uuid::Uuid;

use crate::entity::provider::ProviderKind;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct User {
    id: Uuid,
    login: String,
    email: String,
    password: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Config {
    users: Vec<User>,
}

impl Config {
    pub(super) async fn synchronize<'c>(
        &self,
        mut tx: sqlx::Transaction<'c, sqlx::Sqlite>,
        app: &crate::entity::application::Entity,
    ) -> anyhow::Result<sqlx::Transaction<'c, sqlx::Sqlite>> {
        crate::entity::provider::Upsert::new(app.id, ProviderKind::Credentials)
            .execute(&mut *tx)
            .await?;

        for user in self.users.iter() {
            crate::entity::user::Upsert::new(
                user.id,
                app.id,
                ProviderKind::Credentials,
                &user.login,
                &user.email,
                Some(&user.password),
            )
            .execute(&mut *tx)
            .await?;
        }
        Ok(tx)
    }
}

#[cfg(test)]
impl Config {
    pub(crate) fn test() -> Self {
        Self {
            users: vec![
                User {
                    id: super::CHARLES_ID,
                    login: "charles".into(),
                    email: "charles@example.com".into(),
                    password: "this-is-a-password".into(),
                },
                User {
                    id: super::DAVID_ID,
                    login: "david".into(),
                    email: "david@example.com".into(),
                    password: "this-is-another-password".into(),
                },
            ],
        }
    }
}
