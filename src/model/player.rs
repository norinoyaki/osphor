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
    routes::{AppError, Instance},
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
pub async fn players_get(State(state): State<Instance>) -> Result<impl IntoResponse, AppError> {
    let txn = state.players_db.begin_read()?;

    let table = txn.open_table(PLAYERS)?;
    let mut players = Vec::new();

    for entry in table.iter().unwrap().flatten() {
        let (_id, data) = entry;
        let player = serde_json::from_str::<Player>(data.value())?;
        players.push(player);
    }

    Ok((StatusCode::OK, Json(players)).into_response())
}

pub async fn players_post(
    State(state): State<Instance>,
    Json(mut player): Json<Player>,
) -> Result<impl IntoResponse, AppError> {
    let txn = state.players_db.begin_read()?;
    let table = txn.open_table(PLAYERS)?;
    if table.get(&*player.username)?.is_some() {
        return Ok((StatusCode::CONFLICT, "Username already taken"));
    }

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

    let hash = tokio::task::spawn_blocking(move || {
        Hasher::new()
            .algorithm(Algorithm::Argon2id)
            .salt_length(16)
            .hash_length(32)
            .iterations(1)
            .memory_cost_kib(62500)
            .threads(1)
            .hash(player.password.as_bytes())
            .unwrap()
    })
    .await
    .unwrap();

    player.password = hash.to_string();

    let value = serde_json::to_string(&player)?;

    // Repeat prosess if were failed for less than 3 times
    for _ in 1..=3 {
        let txn = match state.players_db.begin_write() {
            Ok(txn) => txn,
            Err(_) => {
                continue;
            }
        };

        {
            let mut table = txn.open_table(PLAYERS)?;

            if table.insert(&*player.username, &*value).is_err() {
                continue;
            };
        }

        if txn.commit().is_err() {
            continue;
        }

        break;
    }

    Ok((
        StatusCode::CREATED,
        "Player successfully created with custom fields.",
    ))
}
