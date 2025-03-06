use axum::{extract::State, response::IntoResponse, Json};
use redb::ReadableTable;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{database::PLAYERS, routes::Instance};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub username: String,
    pub display: String,
    pub avatar: String,

    #[serde(default = "default_exp")]
    pub exp: u32,

    #[serde(default = "default_rating")]
    pub rating: u16,

    #[serde(default = "default_deviation")]
    pub deviation: u16,

    #[serde(default = "default_volatility")]
    pub volatility: u16,

    pub password: String,
}

fn default_exp() -> u32 {
    0
}
fn default_rating() -> u16 {
    1500
}
fn default_deviation() -> u16 {
    300
}
fn default_volatility() -> u16 {
    (0.6 * 10000.0) as u16
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
                        format!("Failed to deserialive player data: {:?}", e),
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

    {
        let mut table = match txn.open_table(PLAYERS) {
            Ok(table) => table,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        player.exp = 0;
        player.rating = 1500;
        player.deviation = 300;
        player.volatility = (0.6 * 10000.0) as u16;
        let value = serde_json::to_string(&player).unwrap();
        table.insert(&*player.username, &*value).unwrap();
    }
    txn.commit().unwrap();

    (
        StatusCode::CREATED,
        "Players succesfully created.".to_string(),
    )
}
