mod database;
mod model;
mod packets;

use clap::Parser;
use database::Database;
use packets::Packets;
use tokio::join;

#[derive(Parser)]
#[command(name = "Osphor", about = "Dead simple multiplayer server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value = "3145")]
    port: u16,

    #[arg(long, default_value = "./server")]
    dir: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    let db_task = Database::init(args.ip.clone(), args.port);
    let packet_task = Packets::init(address);

    println!("Osphor is running at {}", args.dir);
    let (_, _) = join!(db_task, packet_task);
}
