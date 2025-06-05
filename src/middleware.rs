use std::sync::{Arc, Mutex};

use diesel::mysql::MysqlConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;

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

pub type ConnectionPool = Arc<Mutex<Pool<ConnectionManager<MysqlConnection>>>>;

#[derive(Clone, NewMiddleware)]
pub struct DieselMiddleware {
    pool: ConnectionPool,
}

pub struct ConnectionBox {
    pub pool: ConnectionPool,
}

impl StateData for ConnectionBox {}

impl Middleware for DieselMiddleware {
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

impl DieselMiddleware {
    pub fn new(database_url: String, thread: usize) -> DieselMiddleware {
        let manager = ConnectionManager::new(database_url);
        let pool = Pool::builder()
            .max_size(thread as u32)
            .build(manager)
            .unwrap();

        DieselMiddleware {
            pool: Arc::new(Mutex::new(pool)),
        }
    }
}
