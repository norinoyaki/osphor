use std::collections::HashMap;

use argon2_kdf::{Algorithm, Hasher};
use axum::{extract::State, response::IntoResponse, Json};
use redb::ReadableTable;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value;
use skillratings::weng_lin::WengLinRating;

use crate::{
    database::{load_data, PLAYERS},
    routes::Instance,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub username: String,
    pub password: String,

    #[serde(default = "default_rating")]
    pub rank: WengLinRating,

    pub data: HashMap<String, value::Value>,
}

fn default_rating() -> WengLinRating {
    WengLinRating::new()
}

// Handler to requests all players data
pub async fn players_get(State(state): State<Instance>) -> impl IntoResponse {
    let txn = match state.db.begin_read() {
        Ok(txn) => txn,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database transaction failed: {:?}", e),
            )
                .into_response();
        }
    };

    let table = match txn.open_table(PLAYERS) {
        Ok(table) => table,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to open players table: {:?}", e),
            )
                .into_response();
        }
    };

    let mut players = Vec::new();

    for entry in table.iter().unwrap() {
        if let Ok((_id, data)) = entry {
            match serde_json::from_str::<Player>(data.value()) {
                Ok(player) => players.push(player),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to deserialize player data: {:?}", e),
                    )
                        .into_response()
                }
            }
        }
    }

    (StatusCode::OK, Json(players)).into_response()
}

// Handler to create a player
pub async fn players_post(
    State(state): State<Instance>,
    Json(mut player): Json<Player>,
) -> impl IntoResponse {
    let txn = state.db.begin_write().unwrap();

    // I have no fucking idea what the fuck did I made
    // It clone the real response, replace player data hashmap with default value
    // and map all the real response data into default value hashmap
    // intentionally to prevent extra data being pushed from response
    let temp = player.clone();
    let data_schema = load_data(); // This is the user-defined field config

    player.data.clear(); // Clear existing data to ensure no extra fields

    for field in data_schema.keys() {
        if let Some(value) = temp.data.get(field) {
            player.data.insert(field.clone(), value.clone());
        } else if let Some(default) = data_schema.get(field) {
            player.data.insert(field.clone(), default.clone()); // Ensure default values
        }
    }

    {
        let mut table = match txn.open_table(PLAYERS) {
            Ok(table) => table,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let hash = Hasher::new()
            .algorithm(Algorithm::Argon2id)
            .salt_length(24)
            .hash_length(42)
            .iterations(8)
            .memory_cost_kib(62500)
            .threads(1)
            .hash(player.password.as_bytes())
            .unwrap();

        player.password = hash.to_string();
        let value = serde_json::to_string(&player).unwrap();
        table.insert(&*player.username, &*value).unwrap();
    }
    txn.commit().unwrap();

    (
        StatusCode::CREATED,
        "Player successfully created with custom fields.".to_string(),
    )
}
