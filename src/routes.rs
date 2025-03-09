use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use redb::Database;
use std::sync::Arc;
use tracing::info;

use crate::{
    database::{init_players_db, login, validate},
    model::{players_get, players_post},
    Args,
};

#[derive(Clone)]
pub struct Instance {
    pub players_db: Arc<Database>,
}

pub async fn start_routes(args: &Args) -> Result<(), anyhow::Error> {
    let instance = Instance {
        players_db: Arc::new(init_players_db(args.dir.clone())),
    };

    let app = Router::new()
        .route("/api", get(root))
        .route("/api/players", get(players_get).post(players_post))
        .route("/api/login", post(login))
        .route("/api/validate", post(validate))
        .with_state(instance);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.ip, args.port)).await?;

    info!(
        "REST API is now listening towards {}:{}",
        args.ip, args.port
    );

    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> String {
    "Root Instances of Osphor API".to_string()
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
