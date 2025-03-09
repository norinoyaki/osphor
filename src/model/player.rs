use std::{collections::HashMap, str::FromStr};

use argon2_kdf::{Algorithm, Hash, Hasher};
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redb::ReadableTable;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value;
use skillratings::weng_lin::WengLinRating;

use crate::{
    database::{load_data, ACCOUNTS, PLAYERS},
    routes::{AppError, Instance},
};

// JWT Secret Key (Consider making this configurable)
const JWT_SECRET: &str = "fumo";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    pub username: String,

    #[serde(default)]
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub username: String,

    #[serde(default = "default_rating")]
    pub rank: WengLinRating,

    pub data: HashMap<String, value::Value>,
}

fn default_rating() -> WengLinRating {
    WengLinRating::new()
}

impl Player {
    // Handler to requests all players data
    pub async fn bulk(State(state): State<Instance>) -> Result<impl IntoResponse, AppError> {
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

    pub async fn register(
        State(state): State<Instance>,
        TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
        Json(mut player): Json<Player>,
    ) -> Result<impl IntoResponse, AppError> {
        let txn = state.players_db.begin_read()?;
        let mut account: Account = Account {
            username: player.username.clone(),
            password: auth.token().to_owned(),
        };

        let account_table = txn.open_table(ACCOUNTS)?;

        if account_table.get(&*account.username)?.is_some() {
            return Ok((StatusCode::CONFLICT, "Username already taken"));
        }

        let temp = player.clone();

        // This is the user-defined field config
        let data_schema = load_data();

        // Clear existing data to ensure no extra fields
        player.data.clear();

        for field in data_schema.keys() {
            if let Some(value) = temp.data.get(field) {
                player.data.insert(field.clone(), value.clone());
            } else if let Some(default) = data_schema.get(field) {
                // Ensure default values
                player.data.insert(field.clone(), default.clone());
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
                .hash(auth.token().as_bytes())
                .unwrap()
        })
        .await
        .unwrap();

        account.password = hash.to_string();

        let account_value = serde_json::to_string(&account)?;
        let players_value = serde_json::to_string(&player)?;

        // Repeat prosess if were failed for less than 3 times
        for _ in 1..=3 {
            let txn = match state.players_db.begin_write() {
                Ok(txn) => txn,
                Err(_) => {
                    continue;
                }
            };

            {
                let mut accounts_table = txn.open_table(ACCOUNTS)?;
                let mut players_table = txn.open_table(PLAYERS)?;

                if accounts_table
                    .insert(&*account.username, &*account_value)
                    .is_err()
                {
                    continue;
                };

                if players_table
                    .insert(&*account.username, &*players_value)
                    .is_err()
                {
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
}

impl Account {
    /// Handles user login and generates a session token.
    pub async fn login(
        State(state): State<Instance>,
        TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
        Json(form): Json<Account>,
    ) -> impl IntoResponse {
        let txn = state.players_db.begin_read().unwrap();
        let table = txn.open_table(ACCOUNTS).unwrap();

        let account_data = match table.get(&*form.username).unwrap() {
            Some(f) => f.value().to_owned(),
            None => return (StatusCode::UNAUTHORIZED, "User not found.").into_response(),
        };

        let account = serde_json::from_str::<Account>(&account_data).unwrap();
        let hash = Hash::from_str(&account.password).unwrap();
        if !hash.verify(auth.token().as_bytes()) {
            return (StatusCode::UNAUTHORIZED, "Wrong password.").into_response();
        }

        let token = Self::generate_session(&account.username);
        (StatusCode::OK, token).into_response()
    }

    /// Middleware to validate user session token.
    pub async fn validate(TypedHeader(auth): TypedHeader<Authorization<Bearer>>) -> String {
        match Self::verify_session(auth.token()) {
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
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )
        .map(|token_data| token_data.claims)
    }
}
