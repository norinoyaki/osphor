use std::{collections::HashMap, path::Path, str::FromStr};

use argon2_kdf::Hash;
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::{
    extract::TypedHeader,
    headers::{authorization::Bearer, Authorization},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mlua::{Integer, Lua, Table};
use redb::{Database, TableDefinition};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{model::player::Player, routes::Instance};

// Database Table Definitions
pub const ACCOUNTS: TableDefinition<&str, &str> = TableDefinition::new("accounts");
pub const PLAYERS: TableDefinition<&str, &str> = TableDefinition::new("players");
pub const SESSIONS: TableDefinition<u32, &str> = TableDefinition::new("sessions");

// JWT Secret Key (Consider making this configurable)
pub const JWT_SECRET: &str = "fumo";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

/// Initializes the players database, creating tables if they don't exist.
pub fn init_players_db(path: String) -> Database {
    let players_format = &format!("{}/players.db", path);
    let players_path = Path::new(players_format);
    let players_db = if players_path.exists() {
        Database::open(players_path).expect("Failed to open database.")
    } else {
        Database::create(players_path).expect("Failed to create database.")
    };

    let txn = players_db.begin_write().unwrap();
    {
        txn.open_table(ACCOUNTS).unwrap();
        txn.open_table(PLAYERS).unwrap();
        txn.open_table(SESSIONS).unwrap();
    }
    txn.commit().unwrap();

    players_db
}

/// Loads schema from a Lua script to define player attributes.
pub fn load_data() -> HashMap<String, Value> {
    let lua = Lua::new();
    let script = std::fs::read_to_string("./server/schema.lua").expect("Failed to load script");
    let schema: Table = lua.load(&script).eval().expect("Invalid Lua schema");

    let mut data = HashMap::new();

    if let Ok(fields) = schema.get::<Table>("players") {
        for pair in fields.pairs::<Integer, Table>().flatten() {
            let (_, field) = pair;
            let name: String = field.get("name").unwrap();
            let field_type: String = field.get("type").unwrap();
            let default: Value = match field_type.as_str() {
                "int" => Value::from(field.get::<i32>("default").unwrap_or(0)),
                "bigint" => Value::from(field.get::<i64>("default").unwrap_or(0)),
                "float" => Value::from(field.get::<f32>("default").unwrap_or(0.0)),
                "real" => Value::from(field.get::<f64>("default").unwrap_or(0.0)),
                "string" => Value::from(field.get::<String>("default").unwrap_or_default()),
                "boolean" => Value::from(field.get::<bool>("default").unwrap_or(false)),
                _ => Value::Null,
            };
            data.insert(name, default);
        }
    }
    data
}

/// Handles user login and generates a session token.
pub async fn login(State(state): State<Instance>, Json(login): Json<Login>) -> impl IntoResponse {
    let txn = state.players_db.begin_read().unwrap();
    let table = txn.open_table(PLAYERS).unwrap();

    let user_data = match table.get(&*login.username).unwrap() {
        Some(f) => f.value().to_owned(),
        None => return (StatusCode::UNAUTHORIZED, "User not found.").into_response(),
    };

    let user = serde_json::from_str::<Player>(&user_data).unwrap();
    let hash = Hash::from_str(&user.password).unwrap();
    if !hash.verify(login.password.as_bytes()) {
        return (StatusCode::UNAUTHORIZED, "Wrong password.").into_response();
    }

    let token = generate_session(&user.username);
    (StatusCode::OK, token).into_response()
}

/// Middleware to validate user session token.
pub async fn validate(TypedHeader(auth): TypedHeader<Authorization<Bearer>>) -> String {
    match verify_session(auth.token()) {
        Ok(claims) => format!("{:?}", claims),
        Err(_) => "Failed".to_string(),
    }
}

/// Generates a JWT session token.
pub fn generate_session(sub: &str) -> String {
    let expiration = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: sub.to_string(),
        exp: expiration.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .expect("Failed to create token")
}

/// Verifies the provided JWT token.
pub fn verify_session(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|token_data| token_data.claims)
}
