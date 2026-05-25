use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Artist {
    pub id: u64,
    pub name: String,
}
