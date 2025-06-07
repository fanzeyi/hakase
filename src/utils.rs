use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter;

pub fn generate_code() -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(5)
        .collect()
}
