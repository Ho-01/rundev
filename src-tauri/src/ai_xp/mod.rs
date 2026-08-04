use chrono::{Datelike, Local, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};

const XP_PER_MILESTONE: i64 = 10;
const WEEKLY_MILESTONE_LIMIT: i64 = 21;

// Initial provider-specific calibration. Claude's OTel total includes cache-heavy
// agent traffic, so its block is deliberately larger than the account totals.
const PROVIDERS: [ProviderRule; 3] = [
    ProviderRule {
        id: "codex",
        tokens_per_milestone: 100_000,
    },
    ProviderRule {
        id: "claude",
        tokens_per_milestone: 200_000,
    },
    ProviderRule {
        id: "cursor",
        tokens_per_milestone: 100_000,
    },
];

#[derive(Clone, Copy, Debug)]
struct ProviderRule {
    id: &'static str,
    tokens_per_milestone: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWeeklyXp {
    pub week_started_on: String,
    pub earned_xp: i64,
    pub max_xp: i64,
    pub codex_xp: i64,
    pub claude_xp: i64,
    pub cursor_xp: i64,
}

#[derive(Clone, Copy, Debug)]
struct ProviderProgress {
    rule: ProviderRule,
    tokens: i64,
    existing: i64,
    target: i64,
}

pub async fn sync(pool: &SqlitePool) -> Result<AiWeeklyXp, String> {
    let today = Local::now().date_naive();
    let week_started_on = week_start(today);
    let week = week_started_on.to_string();
    let date = today.to_string();
    let usage = weekly_usage(pool, &week, &date).await?;
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;

    let existing_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_weekly_xp_milestones WHERE week_started_on = ?",
    )
    .bind(&week)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;

    let mut progress = Vec::with_capacity(PROVIDERS.len());
    for (rule, tokens) in PROVIDERS.into_iter().zip(usage) {
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_weekly_xp_milestones
             WHERE week_started_on = ? AND provider = ?",
        )
        .bind(&week)
        .bind(rule.id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        progress.push(ProviderProgress {
            rule,
            tokens,
            existing,
            target: (tokens / rule.tokens_per_milestone).min(WEEKLY_MILESTONE_LIMIT),
        });
    }

    let capacity = (WEEKLY_MILESTONE_LIMIT - existing_total).max(0);
    for provider_index in round_robin_awards(&progress, capacity) {
        award_milestone(&mut transaction, &week, &mut progress[provider_index]).await?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    view(pool, &week).await
}

async fn award_milestone(
    transaction: &mut Transaction<'_, Sqlite>,
    week: &str,
    progress: &mut ProviderProgress,
) -> Result<(), String> {
    let milestone = progress.existing + 1;
    let now = Utc::now().to_rfc3339();
    let source_event_id = format!("ai-xp:{week}:{}:{milestone}", progress.rule.id);
    crate::xp_boost::award_xp(
        transaction,
        "ai_usage_milestone",
        XP_PER_MILESTONE,
        &source_event_id,
        &now,
    )
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO ai_weekly_xp_milestones
         (provider, week_started_on, milestone_index, usage_tokens, awarded_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(progress.rule.id)
    .bind(week)
    .bind(milestone)
    .bind(progress.tokens)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(|error| error.to_string())?;
    progress.existing = milestone;
    Ok(())
}

fn round_robin_awards(progress: &[ProviderProgress], capacity: i64) -> Vec<usize> {
    let mut remaining: Vec<i64> = progress
        .iter()
        .map(|item| (item.target - item.existing).max(0))
        .collect();
    let mut awards = Vec::new();
    while (awards.len() as i64) < capacity && remaining.iter().any(|count| *count > 0) {
        for (index, count) in remaining.iter_mut().enumerate() {
            if *count == 0 || (awards.len() as i64) >= capacity {
                continue;
            }
            awards.push(index);
            *count -= 1;
        }
    }
    awards
}

async fn weekly_usage(pool: &SqlitePool, week: &str, date: &str) -> Result<[i64; 3], String> {
    let codex: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM (
           SELECT total_tokens, ROW_NUMBER() OVER (
             PARTITION BY bucket_started_at ORDER BY observed_at DESC
           ) AS row_number
           FROM ai_usage_snapshots
           WHERE provider = 'codex' AND scope = 'account-day'
             AND bucket_started_at BETWEEN ? AND ?
         ) WHERE row_number = 1",
    )
    .bind(week)
    .bind(date)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let claude: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM ai_usage_events
         WHERE provider = 'claude' AND date(occurred_at, 'localtime') BETWEEN ? AND ?",
    )
    .bind(week)
    .bind(date)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let cursor_account: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.cursor.account_key'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    let cursor: i64 = if let Some(account_key) = cursor_account {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_tokens), 0) FROM (
           SELECT total_tokens, ROW_NUMBER() OVER (
             PARTITION BY date(observed_at, 'localtime') ORDER BY observed_at DESC
           ) AS row_number
           FROM cursor_usage_snapshots
           WHERE account_key = ? AND date(observed_at, 'localtime') BETWEEN ? AND ?
         ) WHERE row_number = 1",
        )
        .bind(account_key)
        .bind(week)
        .bind(date)
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())?
    } else {
        0
    };
    Ok([codex, claude, cursor])
}

async fn view(pool: &SqlitePool, week: &str) -> Result<AiWeeklyXp, String> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT provider, COUNT(*) * ? FROM ai_weekly_xp_milestones
         WHERE week_started_on = ? GROUP BY provider",
    )
    .bind(XP_PER_MILESTONE)
    .bind(week)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let provider_xp = |provider: &str| {
        rows.iter()
            .find(|row| row.0 == provider)
            .map_or(0, |row| row.1)
    };
    let codex_xp = provider_xp("codex");
    let claude_xp = provider_xp("claude");
    let cursor_xp = provider_xp("cursor");
    Ok(AiWeeklyXp {
        week_started_on: week.to_string(),
        earned_xp: codex_xp + claude_xp + cursor_xp,
        max_xp: WEEKLY_MILESTONE_LIMIT * XP_PER_MILESTONE,
        codex_xp,
        claude_xp,
        cursor_xp,
    })
}

fn week_start(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(existing: i64, target: i64) -> ProviderProgress {
        ProviderProgress {
            rule: PROVIDERS[0],
            tokens: 0,
            existing,
            target,
        }
    }

    #[test]
    fn shares_initial_capacity_across_providers() {
        let awards = round_robin_awards(&[progress(0, 20), progress(0, 10), progress(0, 3)], 9);
        assert_eq!(awards, vec![0, 1, 2, 0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn lets_one_provider_fill_unused_capacity() {
        let awards = round_robin_awards(&[progress(0, 5), progress(0, 1), progress(0, 0)], 6);
        assert_eq!(awards, vec![0, 1, 0, 0, 0, 0]);
    }
}
