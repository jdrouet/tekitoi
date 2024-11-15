use crate::entity::provider::ProviderKind;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Config {}

impl Config {
    pub(super) async fn synchronize<'c>(
        &self,
        mut tx: sqlx::Transaction<'c, sqlx::Sqlite>,
        app: &crate::entity::application::Entity,
    ) -> anyhow::Result<sqlx::Transaction<'c, sqlx::Sqlite>> {
        crate::entity::provider::Upsert::new(app.id, ProviderKind::Credentials)
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
}
