use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    #[serde(default)]
    pub id: Option<u64>,
    pub username: String,
    pub display: String,
}

impl Player {}
