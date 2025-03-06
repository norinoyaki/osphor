use axum::{
    routing::{get, post},
    Router,
};
use redb::Database;
use std::sync::Arc;

use crate::{
    database::{init_database, login},
    model::{players_get, players_post},
    Args,
};

#[derive(Clone)]
pub struct Instance {
    pub db: Arc<Database>,
}

pub async fn start_routes(args: &Args) {
    let instance = Instance {
        db: Arc::new(init_database(args.dir.clone())),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/players", get(players_get).post(players_post))
        .route("/login", post(login))
        .with_state(instance);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.ip, args.port))
        .await
        .unwrap();

    println!(
        "REST API is now listening towards {}:{}",
        args.ip, args.port
    );

    axum::serve(listener, app).await.unwrap();
}

async fn root() -> String {
    "Root Instances of Osphor API".to_string()
}
