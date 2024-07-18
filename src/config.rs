use std::env;

#[derive(Debug)]
pub struct Config {
    pub db_url: String,
    pub sentry_url: String,
}

fn read_from_env(name: &str) -> String {
    let value = env::var(name);
    if value.is_err() {
        panic!("Can't read {} from env", name);
    }
    value.unwrap()
}

impl Config {
    pub fn init() -> Self {
        let db_url = read_from_env("DB_URL");
        let sentry_url = read_from_env("SENTRY_URL");

        Config { db_url, sentry_url }
    }
}
