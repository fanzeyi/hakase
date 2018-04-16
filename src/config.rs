
use gotham::state::StateData;

#[derive(Clone, StateData, Debug)]
pub struct Config {
    pub secret: Option<String>,
    pub database_url: String,
}

impl Config {
    pub fn new(secret: Option<String>, database_url: String) -> Config {
        Config {
            secret,
            database_url,
        }
    }
}

impl StateData for Box<Config> {}
