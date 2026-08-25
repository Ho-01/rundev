use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
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
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        // RunDev has several background producers but a single local ledger. One pooled
        // connection serializes their writes inside the process, while WAL keeps external
        // readers from blocking those writes.
        .busy_timeout(Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::connect;
    use sqlx::{Connection, Row, SqliteConnection};

    #[tokio::test]
    async fn connection_policy_allows_writes_while_an_external_reader_is_open() {
        let directory =
            std::env::temp_dir().join(format!("rundev-database-policy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("rundev.db");
        let database_url = format!("sqlite://{}", path.display());
        let pool = connect(&database_url).await.unwrap();

        sqlx::query("CREATE TABLE concurrency_probe (value INTEGER NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO concurrency_probe (value) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();

        let journal_mode: String = sqlx::query("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get(0);
        assert_eq!(journal_mode, "wal");

        let mut reader = SqliteConnection::connect(&database_url).await.unwrap();
        sqlx::query("BEGIN").execute(&mut reader).await.unwrap();
        let value: i64 = sqlx::query("SELECT value FROM concurrency_probe")
            .fetch_one(&mut reader)
            .await
            .unwrap()
            .get(0);
        assert_eq!(value, 1);

        sqlx::query("UPDATE concurrency_probe SET value = 2")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("ROLLBACK").execute(&mut reader).await.unwrap();
        reader.close().await.unwrap();
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
