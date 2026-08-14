use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

pub const TRAIT_MAX_LEVEL: i64 = 20;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitProgress {
    available_points: i64,
    earned_points: i64,
    spent_points: i64,
    traits: Vec<TraitLevel>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitLevel {
    id: String,
    level: i64,
    max_level: i64,
    effect_value: f64,
    effect_unit: &'static str,
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
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT trait_id, level FROM character_traits ORDER BY CASE trait_id
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
    Ok(TraitProgress {
        available_points: (earned_points - spent_points).max(0),
        earned_points,
        spent_points,
        traits: rows
            .into_iter()
            .map(|(id, level)| TraitLevel {
                effect_value: level as f64 * 0.5,
                effect_unit: if id == "reload" {
                    "xp-per-active-day"
                } else {
                    "percent"
                },
                upgrade_cost: if level >= TRAIT_MAX_LEVEL {
                    0
                } else {
                    trait_upgrade_cost(level)
                },
                id,
                level,
                max_level: TRAIT_MAX_LEVEL,
            })
            .collect(),
    })
}

pub async fn upgrade(pool: &SqlitePool, trait_id: &str) -> Result<TraitProgress, String> {
    if !matches!(
        trait_id,
        "focus-ready" | "hot-keyboard" | "reload" | "context-runner"
    ) {
        return Err("알 수 없는 특성입니다.".to_string());
    }
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    let character_level: i64 = sqlx::query_scalar("SELECT level FROM character_state WHERE id = 1")
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    let levels: Vec<i64> = sqlx::query_scalar("SELECT level FROM character_traits")
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
    if !matches!(period, "day" | "week") {
        return Err("지원하지 않는 통계 기간입니다.".to_string());
    }
    let today = Local::now().date_naive();
    let start = if period == "week" {
        week_start(today)
    } else {
        today
    };
    let days = if period == "week" { 7 } else { 1 };
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
    use super::{trait_points_spent, trait_upgrade_cost, traits, upgrade};
    use sqlx::SqlitePool;

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
}
