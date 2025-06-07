use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn generate_code() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(5)
        .map(char::from)
        .collect()
}
