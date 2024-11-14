use crate::entity::user::Entity as UserEntity;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Config {
    users: Vec<UserEntity>,
}

impl Config {
    pub(super) async fn synchronize<'c>(
        &self,
        mut tx: sqlx::Transaction<'c, sqlx::Sqlite>,
        app: &crate::entity::application::Entity,
    ) -> anyhow::Result<sqlx::Transaction<'c, sqlx::Sqlite>> {
        crate::entity::provider::Upsert::new(
            app.id,
            crate::entity::provider::ProviderKind::UserList,
        )
        .execute(&mut *tx)
        .await?;

        for user in self.users.iter() {
            crate::entity::user::Upsert::new(user.id, app.id, &user.login, &user.email)
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
                UserEntity {
                    id: super::ALICE_ID,
                    login: "alice".into(),
                    email: "alice@example.com".into(),
                },
                UserEntity {
                    id: super::BOB_ID,
                    login: "bob".into(),
                    email: "bob@example.com".into(),
                },
            ],
        }
    }
}
