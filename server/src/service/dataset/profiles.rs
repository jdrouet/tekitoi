use uuid::Uuid;

use crate::entity::provider::ProviderKind;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct User {
    id: Uuid,
    login: String,
    email: String,
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
        crate::entity::provider::Upsert::new(app.id, ProviderKind::Profiles)
            .execute(&mut *tx)
            .await?;

        for user in self.users.iter() {
            crate::entity::user::Upsert::new(
                user.id,
                app.id,
                ProviderKind::Profiles,
                &user.login,
                &user.email,
                None,
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
                    id: super::ALICE_ID,
                    login: "alice".into(),
                    email: "alice@example.com".into(),
                },
                User {
                    id: super::BOB_ID,
                    login: "bob".into(),
                    email: "bob@example.com".into(),
                },
            ],
        }
    }
}
