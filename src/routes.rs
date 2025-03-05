use crate::{database::init_database, Args};
use axum::{routing::get, Router};

pub async fn start_routes(args: &Args) {
    init_database(args.dir.clone());

    let app = Router::new()
        .route("/", get(root))
        .route("/players", get(players_get).post(players_post));

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

async fn players_get() -> String {
    "Get".to_string()
}
async fn players_post() -> String {
    "Post".to_string()
}
