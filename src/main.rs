mod database;
mod model;
mod routes;

use std::{fs, path::Path};

use clap::Parser;
use routes::start_routes;

#[derive(Parser)]
#[command(name = "Osphor", about = "Dead simple multiplayer server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value = "31415")]
    port: u16,

    #[arg(long, default_value = "./server")]
    dir: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let working_dir = args.dir.clone();

    // Check existential of the inputted directory
    if !Path::new(&working_dir).is_dir() {
        fs::create_dir(&working_dir).unwrap();
    }

    start_routes(&args).await;
}
