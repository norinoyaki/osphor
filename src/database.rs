use chrono::Utc;
use rand::Rng;
use rusqlite::{params, Connection, Result};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use warp::Filter;

use crate::model::Player;

pub struct Database {}

impl Database {
    pub async fn init(ip: String, port: u16) {
        let address = ([0, 0, 0, 0], port);
        Self::init_api(address.into(), ip, port).await;
    }

    async fn init_api(address: SocketAddr, ip: String, port: u16) {
        let db = Arc::new(Mutex::new(
            Self::connect_db().expect("Failed to connect to database"),
        ));
        let players_api = Self::player_routes(db);

        println!("REST API running on http://{}:{}", ip, port);
        warp::serve(players_api).run(address).await;
    }

    fn connect_db() -> Result<Connection> {
        let conn = Connection::open("data.db")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS players (
                timetag BIGINT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                display TEXT NOT NULL,
                avatar TEXT NOT NULL,
                password TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                username TEXT NOT NULL,
                token TEXT PRIMARY KEY,
                expiration INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(conn)
    }

    fn player_routes(
        db: Arc<Mutex<Connection>>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let get_players = {
            let db = Arc::clone(&db);
            warp::path!("players")
                .and(warp::path::end())
                .and(warp::get())
                .and_then(move || {
                    let db = Arc::clone(&db);
                    async move {
                        let conn = db.lock().await;
                        let mut stmt = conn
                            .prepare(
                                "SELECT timetag, username, display, avatar, password FROM players",
                            )
                            .unwrap();
                        let players_iter = stmt
                            .query_map([], |row| {
                                Ok(Player {
                                    timetag: row.get(0)?,
                                    username: row.get(1)?,
                                    display: row.get(2)?,
                                    avatar: row.get(3)?,
                                    password: row.get(4)?,
                                })
                            })
                            .unwrap();
                        let players: Vec<Player> = players_iter.filter_map(|p| p.ok()).collect();
                        Ok::<_, warp::Rejection>(warp::reply::json(&players))
                    }
                })
        };

        let get_player = {
            let db = Arc::clone(&db);
            warp::path!("players" / String)
                .and(warp::get())
                .and_then(move |username: String| {
                    let db = Arc::clone(&db);
                    async move {
                        let conn = db.lock().await;
                        let player = conn
                            .prepare(
                                "SELECT timetag, username, display FROM players WHERE username = ?1",
                            )
                            .unwrap()
                            .query_row(params![username], |row| {
                                Ok(Player {
                                    timetag: row.get(0)?,
                                    username: row.get(1)?,
                                    display: row.get(2)?,
                                    avatar: row.get(3)?,
                                    password: row.get(4)?,
                                })
                            });

                        match player {
                            Ok(player) => Ok::<_, warp::Rejection>(warp::reply::json(&player)),
                            Err(_) => Ok::<_, warp::Rejection>(warp::reply::json(&None::<Player>)),
                        }
                    }
                })
        };

        let post_player = {
            let db = Arc::clone(&db);
            warp::path("players")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |mut player: Player| {
                    let db = Arc::clone(&db);
                    let random_offset: u16 = rand::rng().random_range(0..10);
                    player.timetag = Some(
                        Utc::now().timestamp_millis() as u64 / 100 * 10 + random_offset as u64,
                    );
                    async move {
                        let conn = db.lock().await;
                        conn.execute(
                            "INSERT INTO players (timetag, username, display, avatar, password) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![player.timetag, player.username, player.display, player.avatar, player.password],
                        )
                        .unwrap();
                        Ok::<_, warp::Rejection>(warp::reply::json(&format!(
                            "Player {} added",
                            player.timetag.unwrap()
                        )))
                    }
                })
        };

        let put_player = {
            let db = Arc::clone(&db);
            warp::path!("players" / String)
                .and(warp::put())
                .and(warp::body::json())
                .and_then(move |timetag: String, player: Player| {
                    let db = Arc::clone(&db);
                    async move {
                        let conn = db.lock().await;
                        conn.execute(
                            "UPDATE players SET display = ?1 WHERE username = ?2",
                            params![player.display, player.username],
                        )
                        .unwrap();
                        Ok::<_, warp::Rejection>(warp::reply::json(&format!(
                            "Updated player {}",
                            timetag
                        )))
                    }
                })
        };

        let delete_player = {
            let db = Arc::clone(&db);
            warp::path!("players" / String)
                .and(warp::delete())
                .and_then(move |timetag: String| {
                    let db = Arc::clone(&db);
                    async move {
                        let conn = db.lock().await;
                        conn.execute("DELETE FROM players WHERE username = ?1", params![timetag])
                            .unwrap();
                        Ok::<_, warp::Rejection>(warp::reply::json(&format!(
                            "Deleted player {}",
                            timetag
                        )))
                    }
                })
        };

        get_players
            .or(get_player)
            .or(post_player)
            .or(put_player)
            .or(delete_player)
    }
}
