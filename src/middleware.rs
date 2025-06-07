use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use super::config::Config;

pub type ConnectionPool = Arc<Mutex<Connection>>;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: ConnectionPool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let database_url = config.database_url.clone();
        let conn = Connection::open(&database_url).expect("Failed to open database");

        // Create the table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS url (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code VARCHAR(20) NOT NULL UNIQUE,
                url VARCHAR(5000) NOT NULL,
                create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .expect("Failed to create url table");

        Self {
            config,
            pool: Arc::new(Mutex::new(conn)),
        }
    }
}
