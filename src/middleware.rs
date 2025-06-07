use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::State;
use gotham::state::StateData;
use gotham_derive::NewMiddleware;

use super::config::Config;

#[derive(Clone, NewMiddleware)]
pub struct ConfigMiddleware {
    config: Box<Config>,
}

impl Middleware for ConfigMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.config);

        Box::new(chain(state))
    }
}

impl ConfigMiddleware {
    pub fn new(config: Config) -> ConfigMiddleware {
        ConfigMiddleware {
            config: Box::new(config),
        }
    }
}

pub type ConnectionPool = Arc<Mutex<Connection>>;

#[derive(Clone, NewMiddleware)]
pub struct SqliteMiddleware {
    pool: ConnectionPool,
}

pub struct ConnectionBox {
    pub pool: ConnectionPool,
}

impl StateData for ConnectionBox {}

impl Middleware for SqliteMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(ConnectionBox {
            pool: self.pool.clone(),
        });
        Box::new(chain(state))
    }
}

impl SqliteMiddleware {
    pub fn new(database_url: String) -> SqliteMiddleware {
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
        ).expect("Failed to create url table");

        SqliteMiddleware {
            pool: Arc::new(Mutex::new(conn)),
        }
    }
}