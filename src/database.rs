use std::{collections::HashMap, path::Path};

use mlua::{Integer, Lua, Table};
use redb::{Database, TableDefinition};
use serde_json::Value;

// Database Table Definitions
pub const ACCOUNTS: TableDefinition<&str, &str> = TableDefinition::new("accounts");
pub const PLAYERS: TableDefinition<&str, &str> = TableDefinition::new("players");
pub const SESSIONS: TableDefinition<u32, &str> = TableDefinition::new("sessions");
// pub const ROOMS: TableDefinition<u32, &str> = TableDefinition::new("rooms");

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

// /// Initializes the players database, creating tables if they don't exist.
// pub fn init_rooms_db(path: String) -> Database {
//     let rooms_format = &format!("{}/rooms.db", path);
//     let rooms_path = Path::new(rooms_format);
//     let rooms_db = if rooms_path.exists() {
//         Database::open(rooms_path).expect("Failed to open database.")
//     } else {
//         Database::create(rooms_path).expect("Failed to create database.")
//     };
//
//     let txn = rooms_db.begin_write().unwrap();
//     {
//         txn.open_table(ROOMS).unwrap();
//     }
//     txn.commit().unwrap();
//
//     rooms_db
// }

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
