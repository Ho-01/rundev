use chrono::{Local, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhipStats {
    pub local_date: String,
    pub whip_count: i64,
}

pub async fn today(pool: &SqlitePool) -> Result<WhipStats, sqlx::Error> {
    let local_date = Local::now().date_naive().to_string();
    let whip_count: i64 = sqlx::query_scalar(
        "SELECT whip_count
         FROM whip_daily_stats
         WHERE local_date = ?",
    )
    .bind(&local_date)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    Ok(WhipStats {
        local_date,
        whip_count,
    })
}

pub async fn record(pool: &SqlitePool) -> Result<WhipStats, sqlx::Error> {
    let local_date = Local::now().date_naive().to_string();
    let updated_at = Utc::now().to_rfc3339();

    let whip_count: i64 = sqlx::query_scalar(
        "INSERT INTO whip_daily_stats (local_date, whip_count, updated_at)
         VALUES (?, 1, ?)
         ON CONFLICT(local_date) DO UPDATE SET
            whip_count = whip_daily_stats.whip_count + 1,
            updated_at = excluded.updated_at
         RETURNING whip_count",
    )
    .bind(&local_date)
    .bind(&updated_at)
    .fetch_one(pool)
    .await?;

    Ok(WhipStats {
        local_date,
        whip_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn record_increments_atomically() {
        let pool = test_pool().await;
        let first = record(&pool).await.unwrap();
        let second = record(&pool).await.unwrap();
        assert_eq!(first.whip_count, 1);
        assert_eq!(second.whip_count, 2);
        assert_eq!(first.local_date, second.local_date);
        assert_eq!(first.local_date, Local::now().date_naive().to_string());
    }

    #[tokio::test]
    async fn today_returns_zero_without_rows() {
        let pool = test_pool().await;
        let stats = today(&pool).await.unwrap();
        assert_eq!(stats.whip_count, 0);
        assert_eq!(stats.local_date, Local::now().date_naive().to_string());
    }
}
