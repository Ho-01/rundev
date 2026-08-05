#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use chrono::{Local, Utc};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU8, Ordering},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

const PRESSES_PER_REWARD: i64 = 2_000;
const XP_PER_REWARD: i64 = 10;

const STATUS_STARTING: u8 = 0;
const STATUS_ACTIVE: u8 = 1;
const STATUS_PERMISSION_REQUIRED: u8 = 2;
const STATUS_ERROR: u8 = 3;
const STATUS_UNAVAILABLE: u8 = 4;

static STATUS: AtomicU8 = AtomicU8::new(STATUS_STARTING);

#[derive(Debug)]
pub(crate) enum KeyEvent {
    Down(u32),
    Up(u32),
    #[cfg(target_os = "macos")]
    Press,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardActivityToday {
    local_date: String,
    press_count: i64,
    rewarded_milestones: i64,
    xp_earned: i64,
    next_reward_at: i64,
    presses_per_reward: i64,
    status: &'static str,
    permission_required: bool,
}

pub fn start(pool: SqlitePool, app: AppHandle) {
    let (sender, receiver) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(process_events(pool.clone(), receiver, app));

    #[cfg(windows)]
    windows::start(sender);

    #[cfg(target_os = "macos")]
    tauri::async_runtime::spawn(macos::start(sender, pool));

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = sender;
        set_status(STATUS_UNAVAILABLE);
    }
}

pub async fn today(pool: &SqlitePool) -> Result<KeyboardActivityToday, sqlx::Error> {
    #[cfg(target_os = "macos")]
    macos::refresh_permission_status();

    let local_date = Local::now().date_naive().to_string();
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT press_count, rewarded_milestones
         FROM keyboard_daily_stats
         WHERE local_date = ?",
    )
    .bind(&local_date)
    .fetch_optional(pool)
    .await?;
    let (press_count, rewarded_milestones) = row.unwrap_or((0, 0));
    let reload_level = crate::progression::trait_level(pool, "reload").await?;
    let presses_per_reward = adjusted_presses_per_reward(reload_level);
    Ok(activity_snapshot(
        local_date,
        press_count,
        rewarded_milestones,
        presses_per_reward,
    ))
}

pub fn open_permission_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos::open_permission_settings();
    }

    #[cfg(not(target_os = "macos"))]
    Ok(())
}

pub async fn reset_permission(pool: &SqlitePool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        set_status(STATUS_PERMISSION_REQUIRED);
        return macos::reset_permission(pool).await;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = pool;
        Err("macOS에서만 입력 모니터링 권한을 초기화할 수 있습니다.".to_string())
    }
}

async fn process_events(
    pool: SqlitePool,
    mut receiver: mpsc::UnboundedReceiver<KeyEvent>,
    app: AppHandle,
) {
    let mut pressed = HashSet::new();
    let mut pending = 0_i64;
    let initial = today(&pool).await.ok();
    let mut presses_per_reward = initial
        .as_ref()
        .map_or(PRESSES_PER_REWARD, |value| value.presses_per_reward);
    let mut persisted = initial.as_ref().map_or(0, |value| value.press_count);
    let mut pending_date = Local::now().date_naive().to_string();
    let mut last_emitted = persisted;
    let mut ui_tick = tokio::time::interval(std::time::Duration::from_millis(250));
    let mut database_flush = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut first_event_recorded = false;
    let mut first_persist_recorded = false;

    loop {
        tokio::select! {
            event = receiver.recv() => {
                let current_date = Local::now().date_naive().to_string();
                if current_date != pending_date {
                    if pending > 0 {
                        if let Err(error) = apply_count_for_date(&pool, pending, &pending_date).await {
                            tracing::warn!(%error, "Keyboard count persistence failed at date boundary");
                        }
                    }
                    pending = 0;
                    pending_date = current_date;
                    persisted = today(&pool).await.map_or(0, |value| value.press_count);
                    last_emitted = persisted;
                    pressed.clear();
                }
                match event {
                    Some(KeyEvent::Down(key)) if !is_modifier(key) && pressed.insert(key) => {
                        if !first_event_recorded {
                            first_event_recorded = true;
                            crate::diagnostics::record("keyboard_first_event_processed", &[]);
                        }
                        pending += 1;
                    }
                    Some(KeyEvent::Up(key)) => {
                        pressed.remove(&key);
                    }
                    #[cfg(target_os = "macos")]
                    Some(KeyEvent::Press) => {
                        if !first_event_recorded {
                            first_event_recorded = true;
                            crate::diagnostics::record("keyboard_first_event_processed", &[]);
                        }
                        pending += 1;
                    }
                    Some(KeyEvent::Down(_)) => {}
                    None => {
                        if pending > 0 {
                            let _ = apply_count_for_date(&pool, pending, &pending_date).await;
                        }
                        break;
                    },
                }
            }
            _ = ui_tick.tick() => {
                let projected = persisted + pending;
                if projected != last_emitted {
                    last_emitted = projected;
                    let milestones = projected / presses_per_reward;
                    let activity = activity_snapshot(
                        pending_date.clone(),
                        projected,
                        milestones,
                        presses_per_reward,
                    );
                    let _ = app.emit("keyboard-activity-updated", activity);
                }
            }
            _ = database_flush.tick() => {
                if pending == 0 {
                    continue;
                }
                let count = std::mem::take(&mut pending);
                if let Err(error) = apply_count_for_date(&pool, count, &pending_date).await {
                    pending += count;
                    tracing::warn!(%error, "Keyboard count persistence failed");
                } else {
                    persisted += count;
                    if let Ok(level) = crate::progression::trait_level(&pool, "reload").await {
                        presses_per_reward = adjusted_presses_per_reward(level);
                    }
                    if !first_persist_recorded {
                        first_persist_recorded = true;
                        crate::diagnostics::record(
                            "keyboard_first_count_persisted",
                            &[("count", count.to_string())],
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
async fn apply_count(pool: &SqlitePool, count: i64) -> Result<(), sqlx::Error> {
    let local_date = Local::now().date_naive().to_string();
    apply_count_for_date(pool, count, &local_date).await
}

async fn apply_count_for_date(
    pool: &SqlitePool,
    count: i64,
    local_date: &str,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let mut transaction = pool.begin().await?;
    let (press_count, rewarded_milestones): (i64, i64) = sqlx::query_as(
        "INSERT INTO keyboard_daily_stats
            (local_date, press_count, rewarded_milestones, updated_at)
         VALUES (?, ?, 0, ?)
         ON CONFLICT(local_date) DO UPDATE SET
            press_count = keyboard_daily_stats.press_count + excluded.press_count,
            updated_at = excluded.updated_at
         RETURNING press_count, rewarded_milestones",
    )
    .bind(local_date)
    .bind(count)
    .bind(&now)
    .fetch_one(&mut *transaction)
    .await?;

    let reload_level = crate::progression::trait_level_in(&mut transaction, "reload").await?;
    let presses_per_reward = adjusted_presses_per_reward(reload_level);
    let earned_milestones = press_count / presses_per_reward;
    for milestone in (rewarded_milestones + 1)..=earned_milestones {
        award_keyboard_xp(&mut transaction, local_date, milestone, &now).await?;
    }

    sqlx::query(
        "UPDATE keyboard_daily_stats
         SET rewarded_milestones = ?, updated_at = ?
         WHERE local_date = ?",
    )
    .bind(earned_milestones)
    .bind(&now)
    .bind(local_date)
    .execute(&mut *transaction)
    .await?;

    upsert_metric(
        &mut transaction,
        local_date,
        "keyboard_presses",
        press_count,
        &now,
    )
    .await?;
    upsert_metric(
        &mut transaction,
        local_date,
        "xp_earned",
        earned_milestones * XP_PER_REWARD,
        &now,
    )
    .await?;
    transaction.commit().await
}

fn activity_snapshot(
    local_date: String,
    press_count: i64,
    rewarded_milestones: i64,
    presses_per_reward: i64,
) -> KeyboardActivityToday {
    let status = current_status();
    KeyboardActivityToday {
        local_date,
        press_count,
        rewarded_milestones,
        xp_earned: rewarded_milestones * XP_PER_REWARD,
        next_reward_at: (press_count / presses_per_reward + 1) * presses_per_reward,
        presses_per_reward,
        status,
        permission_required: status == "permission-required",
    }
}

async fn award_keyboard_xp(
    transaction: &mut Transaction<'_, Sqlite>,
    local_date: &str,
    milestone: i64,
    occurred_at: &str,
) -> Result<(), sqlx::Error> {
    let source_event_id = format!("keyboard:{local_date}:{milestone}");
    crate::xp_boost::award_xp(
        transaction,
        "keyboard_milestone",
        XP_PER_REWARD,
        &source_event_id,
        occurred_at,
    )
    .await?;
    Ok(())
}

async fn upsert_metric(
    transaction: &mut Transaction<'_, Sqlite>,
    local_date: &str,
    metric_type: &str,
    value: i64,
    updated_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO daily_activity_metrics
            (local_date, metric_type, source, value, updated_at)
         VALUES (?, ?, 'keyboard', ?, ?)
         ON CONFLICT(local_date, metric_type, source) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(local_date)
    .bind(metric_type)
    .bind(value)
    .bind(updated_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn is_modifier(key: u32) -> bool {
    matches!(
        key,
        0x10 | 0x11 | 0x12 | 0x14 | 0x5B | 0x5C | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5
    )
}

pub(crate) fn set_status(status: u8) {
    let previous = STATUS.swap(status, Ordering::Relaxed);
    if previous != status {
        crate::diagnostics::record(
            "keyboard_status_changed",
            &[
                ("from", status_label(previous).to_string()),
                ("to", status_label(status).to_string()),
            ],
        );
    }
}

fn adjusted_presses_per_reward(level: i64) -> i64 {
    (PRESSES_PER_REWARD * (10_000 - level * 50) / 10_000).max(1)
}

fn current_status() -> &'static str {
    status_label(STATUS.load(Ordering::Relaxed))
}

#[cfg(target_os = "macos")]
pub(super) fn is_active() -> bool {
    STATUS.load(Ordering::Relaxed) == STATUS_ACTIVE
}

fn status_label(status: u8) -> &'static str {
    match status {
        STATUS_ACTIVE => "active",
        STATUS_PERMISSION_REQUIRED => "permission-required",
        STATUS_ERROR => "error",
        STATUS_UNAVAILABLE => "unavailable",
        _ => "starting",
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_count, is_modifier};
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn excludes_only_modifier_keys() {
        assert!(is_modifier(0x10));
        assert!(is_modifier(0x5B));
        assert!(!is_modifier(0x41));
        assert!(!is_modifier(0x08));
    }

    #[tokio::test]
    async fn awards_ten_xp_once_per_two_thousand_presses() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        apply_count(&pool, 1_999).await.unwrap();
        apply_count(&pool, 1).await.unwrap();
        apply_count(&pool, 2_000).await.unwrap();

        let xp: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0)
             FROM xp_events
             WHERE event_type = 'keyboard_milestone'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let state: (i64, i64) =
            sqlx::query_as("SELECT level, total_xp FROM character_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        let stats: (i64, i64) =
            sqlx::query_as("SELECT press_count, rewarded_milestones FROM keyboard_daily_stats")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(xp, 20);
        assert_eq!(state, (1, 20));
        assert_eq!(stats, (4_000, 2));
    }
}
