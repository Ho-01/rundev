use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

pub const TRAIT_MAX_LEVEL: i64 = 20;
const TRAIT_PERCENT_BASIS_POINTS_PER_LEVEL: i64 = 500;
const RELOAD_BASIS_POINT_XP_PER_LEVEL: i64 = 20_000;
const EXAMPLE_BASE_REWARD_XP: f64 = 10.0;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitProgress {
    available_points: i64,
    earned_points: i64,
    spent_points: i64,
    next_focus_reward_xp: i64,
    next_keyboard_reward_xp: i64,
    traits: Vec<TraitLevel>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitLevel {
    id: String,
    level: i64,
    max_level: i64,
    effect_value: f64,
    next_effect_value: f64,
    effect_unit: &'static str,
    example_reward_value: f64,
    next_example_reward_value: f64,
    upgrade_cost: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityStats {
    period: String,
    active_seconds: i64,
    xp_earned: i64,
    keyboard_presses: i64,
    xp_sources: Vec<XpSource>,
    hourly: Vec<HourlyActivity>,
    apps: Vec<AppUsage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsage {
    app_name: String,
    active_seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XpSource {
    id: String,
    amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyActivity {
    date: String,
    hour: i64,
    active_seconds: i64,
    xp_earned: i64,
}

pub async fn traits(pool: &SqlitePool) -> Result<TraitProgress, String> {
    let character_level: i64 = sqlx::query_scalar("SELECT level FROM character_state WHERE id = 1")
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())?;
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT traits.trait_id, traits.level,
                COALESCE(accumulators.basis_point_xp, 0)
         FROM character_traits traits
         LEFT JOIN trait_bonus_accumulators accumulators
           ON accumulators.trait_id = traits.trait_id
         WHERE traits.trait_id IN ('focus-ready', 'hot-keyboard', 'reload')
         ORDER BY CASE traits.trait_id
           WHEN 'focus-ready' THEN 1 WHEN 'hot-keyboard' THEN 2
           WHEN 'reload' THEN 3 ELSE 4 END",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let spent_points = rows
        .iter()
        .map(|row| trait_points_spent(row.1))
        .sum::<i64>();
    let earned_points = (character_level - 1).max(0);
    let trait_snapshot = |trait_id: &str| {
        rows.iter()
            .find(|row| row.0 == trait_id)
            .map(|row| (row.1, row.2))
            .unwrap_or((0, 0))
    };
    let (focus_level, focus_accumulator) = trait_snapshot("focus-ready");
    let (keyboard_level, keyboard_accumulator) = trait_snapshot("hot-keyboard");
    let (reload_level, reload_accumulator) = trait_snapshot("reload");
    let today = Local::now().date_naive().to_string();
    let reload_claimed_today: bool = sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM trait_daily_rewards
           WHERE trait_id = 'reload' AND local_date = ?
         )",
    )
    .bind(today)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let next_reload_bonus = if reload_claimed_today {
        0
    } else {
        next_reload_reward(reload_level, reload_accumulator)
    };
    Ok(TraitProgress {
        available_points: (earned_points - spent_points).max(0),
        earned_points,
        spent_points,
        next_focus_reward_xp: next_milestone_reward("focus-ready", focus_level, focus_accumulator)
            + next_reload_bonus,
        next_keyboard_reward_xp: next_milestone_reward(
            "hot-keyboard",
            keyboard_level,
            keyboard_accumulator,
        ),
        traits: rows
            .into_iter()
            .map(|(id, level, _accumulator)| {
                let next_level = (level + 1).min(TRAIT_MAX_LEVEL);
                TraitLevel {
                    effect_value: trait_effect_value(&id, level),
                    next_effect_value: trait_effect_value(&id, next_level),
                    effect_unit: if id == "reload" {
                        "xp-per-active-day"
                    } else {
                        "percent"
                    },
                    example_reward_value: trait_example_reward(&id, level),
                    next_example_reward_value: trait_example_reward(&id, next_level),
                    upgrade_cost: if level >= TRAIT_MAX_LEVEL {
                        0
                    } else {
                        trait_upgrade_cost(level)
                    },
                    id,
                    level,
                    max_level: TRAIT_MAX_LEVEL,
                }
            })
            .collect(),
    })
}

pub async fn upgrade(pool: &SqlitePool, trait_id: &str) -> Result<TraitProgress, String> {
    if !matches!(trait_id, "focus-ready" | "hot-keyboard" | "reload") {
        return Err("알 수 없는 특성입니다.".to_string());
    }
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    let character_level: i64 = sqlx::query_scalar("SELECT level FROM character_state WHERE id = 1")
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    let levels: Vec<i64> = sqlx::query_scalar(
        "SELECT level FROM character_traits
         WHERE trait_id IN ('focus-ready', 'hot-keyboard', 'reload')",
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    let spent = levels.into_iter().map(trait_points_spent).sum::<i64>();
    let current_level: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT level FROM character_traits WHERE trait_id = ?), 0)",
    )
    .bind(trait_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    if current_level >= TRAIT_MAX_LEVEL {
        return Err("이미 최대 레벨인 특성입니다.".to_string());
    }
    let upgrade_cost = trait_upgrade_cost(current_level);
    let earned_points = (character_level - 1).max(0);
    if earned_points - spent < upgrade_cost {
        return Err("사용 가능한 특성 포인트가 없습니다.".to_string());
    }
    let changed = sqlx::query(
        "UPDATE character_traits SET level = level + 1 WHERE trait_id = ? AND level < ?",
    )
    .bind(trait_id)
    .bind(TRAIT_MAX_LEVEL)
    .execute(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    if changed.rows_affected() == 0 {
        return Err("이미 최대 레벨인 특성입니다.".to_string());
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    traits(pool).await
}

fn trait_upgrade_cost(current_level: i64) -> i64 {
    current_level / 5 + 1
}

fn trait_points_spent(level: i64) -> i64 {
    (0..level).map(trait_upgrade_cost).sum()
}

pub(crate) fn trait_bonus_basis_points(trait_id: &str, level: i64) -> i64 {
    if matches!(trait_id, "focus-ready" | "hot-keyboard") {
        level.clamp(0, TRAIT_MAX_LEVEL) * TRAIT_PERCENT_BASIS_POINTS_PER_LEVEL
    } else {
        0
    }
}

pub(crate) fn reload_bonus_basis_point_xp(level: i64) -> i64 {
    level.clamp(0, TRAIT_MAX_LEVEL) * RELOAD_BASIS_POINT_XP_PER_LEVEL
}

fn trait_effect_value(trait_id: &str, level: i64) -> f64 {
    if trait_id == "reload" {
        reload_bonus_basis_point_xp(level) as f64 / 10_000.0
    } else {
        trait_bonus_basis_points(trait_id, level) as f64 / 100.0
    }
}

fn trait_example_reward(trait_id: &str, level: i64) -> f64 {
    if trait_id == "reload" {
        trait_effect_value(trait_id, level)
    } else {
        EXAMPLE_BASE_REWARD_XP * (1.0 + trait_effect_value(trait_id, level) / 100.0)
    }
}

fn next_milestone_reward(trait_id: &str, level: i64, accumulated_basis_points: i64) -> i64 {
    let next_bonus =
        (accumulated_basis_points + 10 * trait_bonus_basis_points(trait_id, level)) / 10_000;
    10 + next_bonus
}

fn next_reload_reward(level: i64, accumulated_basis_points: i64) -> i64 {
    (accumulated_basis_points + reload_bonus_basis_point_xp(level)) / 10_000
}

pub async fn trait_level_in(
    transaction: &mut Transaction<'_, Sqlite>,
    trait_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT level FROM character_traits WHERE trait_id = ?), 0)",
    )
    .bind(trait_id)
    .fetch_one(&mut **transaction)
    .await
}

pub async fn stats(pool: &SqlitePool, period: &str) -> Result<ActivityStats, String> {
    if !matches!(period, "day" | "week" | "history") {
        return Err("지원하지 않는 통계 기간입니다.".to_string());
    }
    let today = Local::now().date_naive();
    let start = if period == "history" {
        today - chrono::Duration::days(139)
    } else if period == "week" {
        week_start(today)
    } else {
        today
    };
    let days = match period {
        "history" => 140,
        "week" => 7,
        _ => 1,
    };
    let start_text = start.to_string();
    let end_text = (start + chrono::Duration::days(days - 1)).to_string();
    let activity_sessions: Vec<(String, i64)> = sqlx::query_as(
        "SELECT started_at, active_seconds
         FROM activity_sessions WHERE activity_type = 'development'
           AND date(started_at, 'localtime') BETWEEN ? AND ?",
    )
    .bind(&start_text)
    .bind(&end_text)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let xp_rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT date(occurred_at, 'localtime'), CAST(strftime('%H', occurred_at, 'localtime') AS INTEGER),
                COALESCE(SUM(amount), 0) FROM xp_events
         WHERE date(occurred_at, 'localtime') BETWEEN ? AND ? GROUP BY 1, 2",
    ).bind(&start_text).bind(&end_text).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let xp_source_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT CASE
            WHEN event_type = 'focus_milestone' THEN 'focus'
            WHEN event_type = 'keyboard_milestone' THEN 'keyboard'
            WHEN event_type = 'ai_usage_milestone' THEN 'ai'
            WHEN event_type = 'xp_boost_bonus' THEN 'boost'
            WHEN event_type = 'trait_bonus' THEN 'trait'
            ELSE 'other'
         END AS source, COALESCE(SUM(amount), 0)
         FROM xp_events
         WHERE date(occurred_at, 'localtime') BETWEEN ? AND ?
         GROUP BY source",
    )
    .bind(&start_text)
    .bind(&end_text)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let keyboard_presses: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(press_count), 0) FROM keyboard_daily_stats WHERE local_date BETWEEN ? AND ?",
    ).bind(&start_text).bind(&end_text).fetch_one(pool).await.map_err(|error| error.to_string())?;
    let app_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source, COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') BETWEEN ? AND ?
         GROUP BY source",
    )
    .bind(&start_text)
    .bind(&end_text)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let mut app_totals = HashMap::<String, i64>::new();
    for (source, active_seconds) in app_rows {
        let identifier = source.strip_prefix("foreground:").unwrap_or(&source);
        let app_name = crate::activity::catalog::display_name(identifier);
        *app_totals.entry(app_name).or_default() += active_seconds;
    }
    let mut apps: Vec<_> = app_totals
        .into_iter()
        .map(|(app_name, active_seconds)| AppUsage {
            app_name,
            active_seconds,
        })
        .collect();
    apps.sort_by(|left, right| {
        right
            .active_seconds
            .cmp(&left.active_seconds)
            .then_with(|| left.app_name.cmp(&right.app_name))
    });
    let mut activity_by_hour = HashMap::<(String, i64), i64>::new();
    for (started_at, mut remaining) in activity_sessions {
        let Ok(parsed) = DateTime::parse_from_rfc3339(&started_at) else {
            continue;
        };
        let mut cursor = parsed.with_timezone(&Local);
        while remaining > 0 {
            let seconds_in_hour = 3_600 - i64::from(cursor.minute() * 60 + cursor.second());
            let seconds = remaining.min(seconds_in_hour);
            *activity_by_hour
                .entry((cursor.date_naive().to_string(), i64::from(cursor.hour())))
                .or_default() += seconds;
            remaining -= seconds;
            cursor += chrono::Duration::seconds(seconds);
        }
    }
    let xp_by_hour: HashMap<(String, i64), i64> = xp_rows
        .into_iter()
        .map(|(date, hour, amount)| ((date, hour), amount))
        .collect();
    let mut hourly = Vec::with_capacity((days * 24) as usize);
    for day in 0..days {
        let date = (start + chrono::Duration::days(day)).to_string();
        for hour in 0..24 {
            hourly.push(HourlyActivity {
                active_seconds: activity_by_hour
                    .get(&(date.clone(), hour))
                    .copied()
                    .unwrap_or(0),
                xp_earned: xp_by_hour.get(&(date.clone(), hour)).copied().unwrap_or(0),
                date: date.clone(),
                hour,
            });
        }
    }
    Ok(ActivityStats {
        period: period.to_string(),
        active_seconds: hourly.iter().map(|slot| slot.active_seconds).sum(),
        xp_earned: hourly.iter().map(|slot| slot.xp_earned).sum(),
        keyboard_presses,
        xp_sources: xp_source_rows
            .into_iter()
            .map(|(id, amount)| XpSource { id, amount })
            .collect(),
        hourly,
        apps,
    })
}

fn week_start(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

#[cfg(test)]
mod tests {
    use super::{
        next_milestone_reward, next_reload_reward, reload_bonus_basis_point_xp,
        trait_bonus_basis_points, trait_example_reward, trait_points_spent, trait_upgrade_cost,
        traits, upgrade,
    };
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn history_stats_include_exactly_the_last_140_local_dates() {
        use chrono::{Duration, Local, TimeZone};
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let today = Local::now().date_naive();
        for (index, offset) in [0, 139, 140].into_iter().enumerate() {
            let date = today - Duration::days(offset);
            let timestamp = Local
                .from_local_datetime(&date.and_hms_opt(12, 0, 0).unwrap())
                .earliest()
                .unwrap()
                .to_rfc3339();
            sqlx::query("INSERT INTO activity_sessions (id, started_at, active_seconds, activity_type, source) VALUES (?, ?, 600, 'development', 'foreground:code.exe')")
                .bind(format!("history-session-{index}")).bind(&timestamp).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO xp_events (id, occurred_at, event_type, amount) VALUES (?, ?, 'focus_milestone', 10)")
                .bind(format!("history-xp-{index}")).bind(&timestamp).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO keyboard_daily_stats (local_date, press_count, updated_at) VALUES (?, 100, ?)")
                .bind(date.to_string()).bind(&timestamp).execute(&pool).await.unwrap();
        }
        let stats = super::stats(&pool, "history").await.unwrap();
        assert_eq!(stats.period, "history");
        assert_eq!(stats.hourly.len(), 140 * 24);
        assert_eq!(
            stats.hourly.first().unwrap().date,
            (today - Duration::days(139)).to_string()
        );
        assert_eq!(stats.hourly.last().unwrap().date, today.to_string());
        assert_eq!(stats.active_seconds, 1200);
        assert_eq!(stats.xp_earned, 20);
        assert_eq!(stats.keyboard_presses, 200);
        assert_eq!(
            stats
                .xp_sources
                .iter()
                .map(|source| source.amount)
                .sum::<i64>(),
            20
        );
        assert_eq!(
            stats.apps.iter().map(|app| app.active_seconds).sum::<i64>(),
            1200
        );
        assert!(super::stats(&pool, "unsupported").await.is_err());
    }

    #[tokio::test]
    async fn retired_ai_trait_returns_spent_points_without_changing_history() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("UPDATE character_state SET level = 6, total_xp = 500 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE character_traits SET level = 5 WHERE trait_id = 'context-runner'")
            .execute(&pool)
            .await
            .unwrap();
        let progress = traits(&pool).await.unwrap();
        assert_eq!(progress.available_points, 5);
        assert_eq!(progress.spent_points, 0);
        assert_eq!(progress.traits.len(), 3);
        assert!(progress.traits.iter().all(|t| t.id != "context-runner"));
        assert!(upgrade(&pool, "context-runner").await.is_err());
        let upgraded = upgrade(&pool, "focus-ready").await.unwrap();
        assert_eq!(upgraded.available_points, 4);
        let saved_level: i64 = sqlx::query_scalar(
            "SELECT level FROM character_traits WHERE trait_id = 'context-runner'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let xp: i64 = sqlx::query_scalar("SELECT total_xp FROM character_state WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(saved_level, 5);
        assert_eq!(xp, 500);
    }

    #[test]
    fn trait_upgrade_costs_increase_every_five_levels() {
        assert_eq!(trait_upgrade_cost(0), 1);
        assert_eq!(trait_upgrade_cost(4), 1);
        assert_eq!(trait_upgrade_cost(5), 2);
        assert_eq!(trait_upgrade_cost(10), 3);
        assert_eq!(trait_upgrade_cost(15), 4);
    }

    #[test]
    fn maxing_one_trait_costs_fifty_points() {
        assert_eq!(trait_points_spent(0), 0);
        assert_eq!(trait_points_spent(5), 5);
        assert_eq!(trait_points_spent(10), 15);
        assert_eq!(trait_points_spent(15), 30);
        assert_eq!(trait_points_spent(20), 50);
    }

    #[test]
    fn level_sixteen_traits_have_material_rewards() {
        assert_eq!(trait_bonus_basis_points("hot-keyboard", 16), 8_000);
        assert_eq!(trait_example_reward("hot-keyboard", 16), 18.0);
        assert_eq!(trait_example_reward("focus-ready", 20), 20.0);
        assert_eq!(reload_bonus_basis_point_xp(16), 320_000);
        assert_eq!(trait_example_reward("reload", 16), 32.0);
        assert_eq!(next_milestone_reward("hot-keyboard", 16, 0), 18);
        assert_eq!(next_milestone_reward("focus-ready", 1, 0), 10);
        assert_eq!(next_milestone_reward("focus-ready", 1, 5_000), 11);
        assert_eq!(next_reload_reward(4, 0), 8);
    }

    #[tokio::test]
    async fn grants_one_point_per_level_after_the_starting_level() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("UPDATE character_state SET level = 16 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        for _ in 0..10 {
            upgrade(&pool, "focus-ready").await.unwrap();
        }
        let progress = traits(&pool).await.unwrap();
        assert_eq!(progress.earned_points, 15);
        assert_eq!(progress.spent_points, 15);
        assert_eq!(progress.available_points, 0);
        assert!(upgrade(&pool, "focus-ready").await.is_err());
    }

    #[tokio::test]
    async fn reports_the_exact_next_rewards_from_accumulators_and_daily_reload_state() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        for (trait_id, level) in [
            ("focus-ready", 1),
            ("hot-keyboard", 16),
            ("reload", 4),
            ("context-runner", 2),
        ] {
            sqlx::query("UPDATE character_traits SET level = ? WHERE trait_id = ?")
                .bind(level)
                .bind(trait_id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO trait_bonus_accumulators (trait_id, basis_point_xp)
             VALUES ('focus-ready', 5000)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let progress = traits(&pool).await.unwrap();
        assert_eq!(progress.next_focus_reward_xp, 19);
        assert_eq!(progress.next_keyboard_reward_xp, 18);

        let today = chrono::Local::now().date_naive().to_string();
        sqlx::query(
            "INSERT INTO trait_daily_rewards
             (trait_id, local_date, awarded_at, basis_point_xp, xp_awarded)
             VALUES ('reload', ?, ?, 80000, 8)",
        )
        .bind(today)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
        let after_reload = traits(&pool).await.unwrap();
        assert_eq!(after_reload.next_focus_reward_xp, 11);
    }
}
