use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        // Several local background services can write during application startup.
        // Wait briefly for a writer instead of failing a harmless startup sync on SQLite's
        // immediate "database is locked" response.
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
