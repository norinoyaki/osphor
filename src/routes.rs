use std::sync::Arc;

use crate::{
    database::init_database,
    model::{players_get, players_post},
    Args,
};
use axum::{routing::get, Router};
use redb::Database;

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
