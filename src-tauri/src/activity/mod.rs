#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub(crate) mod catalog;

use chrono::{Local, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::{Instant, MissedTickBehavior};
use uuid::Uuid;

const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const DATABASE_FLUSH_INTERVAL: i64 = 10;
const FOCUS_SECONDS_PER_REWARD: i64 = 30 * 60;
const XP_PER_REWARD: i64 = 10;

#[derive(Debug)]
struct PlatformSnapshot {
    app_identifier: Option<String>,
    app_name: Option<String>,
    idle_for: Duration,
    locked: bool,
}

impl PlatformSnapshot {
    fn eligible_identifier(&self) -> Option<&str> {
        if self.locked || self.idle_for >= IDLE_TIMEOUT {
            return None;
        }
        self.app_identifier
            .as_deref()
            .filter(|identifier| catalog::is_developer_app(identifier))
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusActivityUpdate {
    active_seconds: i64,
    focused: bool,
    active: bool,
    app_name: Option<String>,
}

struct ActiveSession {
    id: String,
    app_identifier: String,
    local_date: String,
    started_at: String,
    active_seconds: i64,
    persisted_seconds: i64,
}

pub fn start(pool: SqlitePool, app: AppHandle) {
    tauri::async_runtime::spawn(run(pool, app));
}

async fn run(pool: SqlitePool, app: AppHandle) {
    if let Err(error) = close_interrupted_sessions(&pool).await {
        tracing::warn!(%error, "Interrupted focus session cleanup failed");
    }
    let mut interval = tokio::time::interval_at(Instant::now() + SAMPLE_INTERVAL, SAMPLE_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut session: Option<ActiveSession> = None;
    let mut current_date = Local::now().date_naive().to_string();
    if let Err(error) = sync_focus_xp(&pool, &current_date).await {
        tracing::warn!(%error, "Focus XP startup synchronization failed");
    }
    let mut persisted_today = load_today_total(&pool).await.unwrap_or(0);

    loop {
        interval.tick().await;
        let snapshot = platform_snapshot();
        let rundev_is_foreground = snapshot
            .app_identifier
            .as_deref()
            .is_some_and(catalog::is_rundev);
        let next_identifier = snapshot.eligible_identifier().map(str::to_owned);
        let next_date = Local::now().date_naive().to_string();

        if next_date != current_date {
            if let Some(active) = session.take() {
                if let Err(error) = close_session(&pool, &active).await {
                    tracing::warn!(%error, "Focus session date-boundary persistence failed");
                }
            }
            current_date = next_date;
            persisted_today = load_today_total(&pool).await.unwrap_or(0);
        }

        let changed = session
            .as_ref()
            .map(|active| Some(active.app_identifier.as_str()) != next_identifier.as_deref())
            .unwrap_or(next_identifier.is_some());

        if changed {
            if let Some(active) = session.take() {
                match close_session(&pool, &active).await {
                    Ok(()) => persisted_today += active.active_seconds - active.persisted_seconds,
                    Err(error) => tracing::warn!(%error, "Focus session close failed"),
                }
            }
            if let Some(identifier) = next_identifier.as_deref() {
                match create_session(&pool, identifier, &current_date).await {
                    Ok(active) => session = Some(active),
                    Err(error) => tracing::warn!(%error, "Focus session creation failed"),
                }
            }
        }

        if let Some(active) = session.as_mut() {
            active.active_seconds += 1;
            if active.active_seconds - active.persisted_seconds >= DATABASE_FLUSH_INTERVAL {
                match persist_session(&pool, active, false).await {
                    Ok(()) => {
                        persisted_today += active.active_seconds - active.persisted_seconds;
                        active.persisted_seconds = active.active_seconds;
                    }
                    Err(error) => tracing::warn!(%error, "Focus session persistence failed"),
                }
            }
        }

        let projected = persisted_today
            + session
                .as_ref()
                .map_or(0, |active| active.active_seconds - active.persisted_seconds);
        if !rundev_is_foreground {
            let update = FocusActivityUpdate {
                active_seconds: projected,
                focused: session.is_some(),
                active: !snapshot.locked && snapshot.idle_for < IDLE_TIMEOUT,
                app_name: snapshot.app_name,
            };
            let _ = app.emit("focus-activity-updated", update);
        }
    }
}

async fn close_interrupted_sessions(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE activity_sessions
         SET ended_at = datetime(started_at, '+' || active_seconds || ' seconds')
         WHERE activity_type = 'development' AND ended_at IS NULL",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn create_session(
    pool: &SqlitePool,
    app_identifier: &str,
    local_date: &str,
) -> Result<ActiveSession, sqlx::Error> {
    let session = ActiveSession {
        id: Uuid::new_v4().to_string(),
        app_identifier: app_identifier.to_owned(),
        local_date: local_date.to_owned(),
        started_at: Utc::now().to_rfc3339(),
        active_seconds: 0,
        persisted_seconds: 0,
    };
    sqlx::query(
        "INSERT INTO activity_sessions
            (id, started_at, active_seconds, activity_type, source)
         VALUES (?, ?, 0, 'development', ?)",
    )
    .bind(&session.id)
    .bind(&session.started_at)
    .bind(format!("foreground:{}", session.app_identifier))
    .execute(pool)
    .await?;
    sync_focus_xp(pool, &session.local_date).await?;
    Ok(session)
}

async fn persist_session(
    pool: &SqlitePool,
    session: &ActiveSession,
    ended: bool,
) -> Result<(), sqlx::Error> {
    let ended_at = ended.then(|| Utc::now().to_rfc3339());
    sqlx::query(
        "UPDATE activity_sessions
         SET active_seconds = ?, ended_at = ?
         WHERE id = ?",
    )
    .bind(session.active_seconds)
    .bind(ended_at)
    .bind(&session.id)
    .execute(pool)
    .await?;
    sync_focus_xp(pool, &session.local_date).await?;
    Ok(())
}

async fn sync_focus_xp(pool: &SqlitePool, local_date: &str) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let active_seconds: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') = ?",
    )
    .bind(local_date)
    .fetch_one(&mut *transaction)
    .await?;
    let earned_milestones = active_seconds / FOCUS_SECONDS_PER_REWARD;
    let now = Utc::now().to_rfc3339();

    for milestone in 1..=earned_milestones {
        let source_event_id = format!("focus:{local_date}:{milestone}");
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO xp_events
                (id, occurred_at, event_type, amount, source_event_id)
             VALUES (?, ?, 'focus_milestone', ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&now)
        .bind(XP_PER_REWARD)
        .bind(source_event_id)
        .execute(&mut *transaction)
        .await?;

        if inserted.rows_affected() == 1 {
            sqlx::query(
                "UPDATE character_state
                 SET total_xp = total_xp + ?,
                     level = ((total_xp + ?) / 100) + 1
                 WHERE id = 1",
            )
            .bind(XP_PER_REWARD)
            .bind(XP_PER_REWARD)
            .execute(&mut *transaction)
            .await?;
        }
    }

    transaction.commit().await
}

async fn close_session(pool: &SqlitePool, session: &ActiveSession) -> Result<(), sqlx::Error> {
    persist_session(pool, session, true).await
}

async fn load_today_total(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let date = Local::now().date_naive().to_string();
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') = ?",
    )
    .bind(date)
    .fetch_one(pool)
    .await
}

fn platform_snapshot() -> PlatformSnapshot {
    #[cfg(windows)]
    return windows::snapshot();

    #[cfg(target_os = "macos")]
    return macos::snapshot();

    #[cfg(not(any(windows, target_os = "macos")))]
    PlatformSnapshot {
        app_identifier: None,
        app_name: None,
        idle_for: Duration::MAX,
        locked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        catalog, close_interrupted_sessions, create_session, load_today_total, persist_session,
        sync_focus_xp, PlatformSnapshot, IDLE_TIMEOUT,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use std::time::Duration;

    fn snapshot(identifier: Option<&str>, idle_for: Duration, locked: bool) -> PlatformSnapshot {
        PlatformSnapshot {
            app_identifier: identifier.map(str::to_owned),
            app_name: identifier.map(catalog::display_name),
            idle_for,
            locked,
        }
    }

    #[test]
    fn counts_known_developer_app_before_idle_timeout() {
        #[cfg(windows)]
        let identifier = "code.exe";
        #[cfg(target_os = "macos")]
        let identifier = "com.microsoft.VSCode";

        assert!(catalog::is_developer_app(identifier));
        assert_eq!(
            snapshot(Some(identifier), Duration::from_secs(299), false).eligible_identifier(),
            Some(identifier)
        );
    }

    #[test]
    fn stops_at_idle_timeout_or_lock() {
        #[cfg(windows)]
        let identifier = "code.exe";
        #[cfg(target_os = "macos")]
        let identifier = "com.microsoft.VSCode";

        assert!(snapshot(Some(identifier), IDLE_TIMEOUT, false)
            .eligible_identifier()
            .is_none());
        assert!(snapshot(Some(identifier), Duration::ZERO, true)
            .eligible_identifier()
            .is_none());
    }

    #[test]
    fn rejects_non_developer_foreground_app() {
        assert!(snapshot(Some("chrome.exe"), Duration::ZERO, false)
            .eligible_identifier()
            .is_none());
    }

    #[tokio::test]
    async fn persists_one_session_row_and_closes_interrupted_sessions() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let date = chrono::Local::now().date_naive().to_string();
        let mut session = create_session(&pool, "code.exe", &date).await.unwrap();
        session.active_seconds = 12;
        persist_session(&pool, &session, false).await.unwrap();
        close_interrupted_sessions(&pool).await.unwrap();

        let ended_at: Option<String> =
            sqlx::query_scalar("SELECT ended_at FROM activity_sessions WHERE id = ?")
                .bind(&session.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(ended_at.is_some());
        assert_eq!(load_today_total(&pool).await.unwrap(), 12);
    }

    #[tokio::test]
    async fn awards_ten_xp_once_per_thirty_focus_minutes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let date = chrono::Local::now().date_naive().to_string();
        let mut session = create_session(&pool, "code.exe", &date).await.unwrap();
        session.active_seconds = 3_600;
        persist_session(&pool, &session, false).await.unwrap();
        sync_focus_xp(&pool, &date).await.unwrap();

        let xp: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0)
             FROM xp_events
             WHERE event_type = 'focus_milestone'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let total_xp: i64 = sqlx::query_scalar("SELECT total_xp FROM character_state WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(xp, 20);
        assert_eq!(total_xp, 20);
    }
}
