use std::path::Path;

use axum::{extract::State, response::IntoResponse, Json};
use redb::{Database, TableDefinition};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{model::player::Player, routes::Instance};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

pub const PLAYERS: TableDefinition<&str, &str> = TableDefinition::new("players");

pub fn init_database(path: String) -> Database {
    let working_path = format!("{}/data.db", path);
    let db_path = Path::new(&working_path);

    if db_path.exists() {
        Database::open(db_path).expect("Failed to open database.")
    } else {
        Database::create(db_path).expect("Failed to create database.")
    }
}

pub async fn login(State(state): State<Instance>, Json(login): Json<Login>) -> impl IntoResponse {
    let txn = state.db.begin_read().unwrap();
    let table = txn.open_table(PLAYERS).unwrap();

    let field = table.get(&*login.username).unwrap();
    let value = match field {
        Some(f) => f.value().to_owned(),
        None => return (StatusCode::UNAUTHORIZED, "User not found.").into_response(),
    };

    let user = serde_json::from_str::<Player>(&value).unwrap();
    if login.password != user.password {
        return (StatusCode::UNAUTHORIZED, "Wrong password.").into_response();
    }

    (StatusCode::OK, Json(user)).into_response()
}
