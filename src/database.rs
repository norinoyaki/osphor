use std::{path::Path, str::FromStr};

use argon2_kdf::Hash;
use axum::{extract::State, response::IntoResponse, Json};
use mlua::{Integer, Lua, Table};
use redb::{Database, TableDefinition};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

pub fn load_data() -> HashMap<String, Value> {
    let lua = Lua::new();
    let script = std::fs::read_to_string("./server/schema.lua").expect("Failed to load script");
    let schema: Table = lua.load(&script).eval().expect("Invalid Lua schema");

    let mut data = HashMap::new();

    if let Ok(fields) = schema.get::<Table>("players") {
        for pair in fields.pairs::<Integer, Table>() {
            if let Ok((_, field)) = pair {
                let name: String = field.get("name").unwrap();
                let field_type: String = field.get("type").unwrap();
                let default: Value = match field_type.as_str() {
                    "int" => Value::from(field.get::<i32>("default").unwrap()),
                    "bigint" => Value::from(field.get::<i64>("default").unwrap()),
                    "float" => Value::from(field.get::<f32>("default").unwrap()),
                    "real" => Value::from(field.get::<f64>("default").unwrap()),
                    "string" => Value::from(field.get::<String>("default").unwrap()),
                    "boolean" => Value::from(field.get::<bool>("default").unwrap()),
                    _ => Value::Null,
                };
                data.insert(name, default);
            }
        }
    }
    data
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
    let hash = Hash::from_str(&user.password).unwrap();
    if !hash.verify(login.password.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, "Wrong password.").into_response();
    }

    (StatusCode::OK, Json(user)).into_response()
}
