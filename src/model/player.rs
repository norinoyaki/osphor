use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    #[serde(default)]
    pub timetag: Option<u64>,
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub password: String,
}

impl Player {}
