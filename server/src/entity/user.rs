#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub id: uuid::Uuid,
    pub login: String,
    pub email: String,
}
