use rand::{rngs::OsRng, seq::IteratorRandom};

const ID_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

fn random_id(prefix: &str, length: usize) -> String {
    prefix
        .chars()
        .chain((0..length).map(|_| ID_CHARS.chars().choose(&mut OsRng).unwrap()))
        .collect()
}

pub fn random_runner_token() -> String {
    random_id("t", 30)
}