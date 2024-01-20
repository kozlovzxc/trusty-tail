use std::env;

#[derive(Debug)]
pub struct Config {
    pub db_url: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
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
        let db_name = read_from_env("DB_NAME");
        let db_user = read_from_env("DB_USER");
        let db_password = read_from_env("DB_PASSWORD");
        let sentry_url = read_from_env("SENTRY_URL");

        Config {
            db_url,
            db_name,
            db_user,
            db_password,
            sentry_url,
        }
    }
}
