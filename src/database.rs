use std::path::Path;

use redb::{Database, TableDefinition};

pub const PLAYERS: TableDefinition<u32, &str> = TableDefinition::new("players");

pub fn init_database(path: String) -> Database {
    let working_path = format!("{}/data.db", path);
    let db_path = Path::new(&working_path);

    if db_path.exists() {
        Database::open(db_path).expect("Failed to open database.")
    } else {
        Database::create(db_path).expect("Failed to create database.")
    }
}
