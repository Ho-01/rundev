use crate::{
    activity, adapters, ai_xp, database::AppState, diagnostics, host_metrics, keyboard,
    progression, tray, whip, xp_boost,
};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    date: String,
    active_seconds: i64,
    xp_earned: i64,
    ai_events: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusAppUsage {
    app_name: String,
    active_seconds: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusActivityToday {
    last_app_name: Option<String>,
    apps: Vec<FocusAppUsage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityHistoryDay {
    date: String,
    active_seconds: i64,
    intensity: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterState {
    level: i64,
    total_xp: i64,
    current_form: String,
    xp_into_level: i64,
    xp_for_next_level: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageToday {
    provider: String,
    total_tokens: Option<i64>,
    week_tokens: Option<i64>,
    source: Option<String>,
    last_synced_at: Option<String>,
    status: String,
    error: Option<String>,
    account_label: Option<String>,
    environment: Option<String>,
    latest_available_date: Option<String>,
    latest_available_tokens: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountPreview {
    account_label: String,
    auth_type: String,
    plan_type: Option<String>,
    environment: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeConnectionPreview {
    settings_path: String,
    has_conflicts: bool,
}

#[tauri::command]
pub fn preview_xp_coupon(code: String) -> Result<xp_boost::CouponPreview, String> {
    xp_boost::preview(&code)
}

#[tauri::command]
pub async fn redeem_xp_coupon(
    code: String,
    state: State<'_, AppState>,
) -> Result<xp_boost::XpBoostStatus, String> {
    xp_boost::redeem(&state.pool, &code).await
}

#[tauri::command]
pub async fn get_xp_boost_status(
    state: State<'_, AppState>,
) -> Result<xp_boost::XpBoostStatus, String> {
    xp_boost::status(&state.pool).await
}

#[tauri::command]
pub async fn sync_ai_weekly_xp(state: State<'_, AppState>) -> Result<ai_xp::AiWeeklyXp, String> {
    ai_xp::sync(&state.pool).await
}

#[tauri::command]
pub async fn get_trait_progress(
    state: State<'_, AppState>,
) -> Result<progression::TraitProgress, String> {
    progression::traits(&state.pool).await
}

#[tauri::command]
pub async fn upgrade_trait(
    trait_id: String,
    state: State<'_, AppState>,
) -> Result<progression::TraitProgress, String> {
    progression::upgrade(&state.pool, &trait_id).await
}

#[tauri::command]
pub async fn get_activity_stats(
    period: String,
    state: State<'_, AppState>,
) -> Result<progression::ActivityStats, String> {
    progression::stats(&state.pool, &period).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageToday {
    provider: String,
    total_tokens: i64,
    week_tokens: i64,
    input_tokens: i64,
    output_tokens: i64,
    cached_tokens: i64,
    cache_write_tokens: i64,
    session_count: i64,
    last_received_at: Option<String>,
    status: String,
    error: Option<String>,
}

#[tauri::command]
pub async fn grant_cursor_usage_consent(state: State<'_, AppState>) -> Result<(), String> {
    adapters::cursor::grant_consent(&state.pool).await
}

#[tauri::command]
pub async fn preview_cursor_account(
    state: State<'_, AppState>,
) -> Result<adapters::cursor::AccountPreview, String> {
    adapters::cursor::preview(&state.pool).await
}

#[tauri::command]
pub async fn connect_cursor_account(state: State<'_, AppState>) -> Result<(), String> {
    adapters::cursor::connect(&state.pool).await
}

#[tauri::command]
pub async fn disconnect_cursor_account(
    revoke_consent: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    adapters::cursor::disconnect(&state.pool, revoke_consent).await
}

#[tauri::command]
pub async fn get_cursor_usage(
    state: State<'_, AppState>,
) -> Result<adapters::cursor::UsageView, String> {
    adapters::cursor::get_usage(&state.pool).await
}

#[tauri::command]
pub async fn refresh_cursor_usage(state: State<'_, AppState>) -> Result<(), String> {
    adapters::cursor::manual_sync_if_due(&state.pool, std::time::Duration::from_secs(30)).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiActivityStatus {
    active_provider_count: i64,
    codex_active: bool,
    claude_active: bool,
    claude_active_sessions: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSelection {
    runner_id: String,
}

#[tauri::command]
pub async fn get_keyboard_activity_today(
    state: State<'_, AppState>,
) -> Result<keyboard::KeyboardActivityToday, String> {
    keyboard::today(&state.pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_keyboard_permission_settings() -> Result<(), String> {
    keyboard::open_permission_settings()
}

#[tauri::command]
pub async fn reset_keyboard_permission(state: State<'_, AppState>) -> Result<(), String> {
    keyboard::reset_permission(&state.pool).await
}

#[tauri::command]
pub fn open_diagnostics_folder() -> Result<(), String> {
    diagnostics::open_folder()
}

#[tauri::command]
pub fn get_system_stats(app: AppHandle) -> host_metrics::SystemStats {
    host_metrics::current_stats(&app)
}

#[tauri::command]
pub fn set_system_panel_expanded(
    app: AppHandle,
    expanded: bool,
    expansion_side: Option<String>,
    previous_expansion_side: Option<String>,
) -> Result<(), String> {
    tray::set_system_panel_expanded(
        &app,
        expanded,
        expansion_side.as_deref(),
        previous_expansion_side.as_deref(),
    )
}

#[tauri::command]
pub async fn get_whip_stats(state: State<'_, AppState>) -> Result<whip::WhipStats, String> {
    whip::today(&state.pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn record_whip(state: State<'_, AppState>) -> Result<whip::WhipStats, String> {
    whip::record(&state.pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_runner_selection(state: State<'_, AppState>) -> Result<RunnerSelection, String> {
    let runner_id =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'runner.selected'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| "coding-cat".to_string());
    let runner_id = if runner_id == "coding-white-cat" {
        "coding-shrimp".to_string()
    } else {
        runner_id
    };
    Ok(RunnerSelection { runner_id })
}

#[tauri::command]
pub async fn set_runner_selection(
    runner_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !is_supported_runner(&runner_id) {
        return Err("지원하지 않는 개발자 캐릭터입니다.".to_string());
    }
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES ('runner.selected', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&runner_id)
    .execute(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    tray::set_runner(&runner_id);
    let _ = app.emit("runner-selection-changed", RunnerSelection { runner_id });
    Ok(())
}

fn is_supported_runner(runner_id: &str) -> bool {
    matches!(
        runner_id,
        "coding-cat" | "coding-fish" | "coding-orange-cat" | "coding-shrimp" | "coding-vtuber"
    )
}

#[cfg(test)]
mod tests {
    use super::{activity_intensity, is_supported_runner, week_start};
    use chrono::NaiveDate;

    #[test]
    fn accepts_only_packaged_runner_ids() {
        assert!(is_supported_runner("coding-cat"));
        assert!(is_supported_runner("coding-fish"));
        assert!(is_supported_runner("coding-orange-cat"));
        assert!(is_supported_runner("coding-shrimp"));
        assert!(is_supported_runner("coding-vtuber"));
        assert!(!is_supported_runner("../custom"));
    }

    #[test]
    fn maps_focus_seconds_to_grass_intensity() {
        assert_eq!(activity_intensity(0), 0);
        assert_eq!(activity_intensity(1), 1);
        assert_eq!(activity_intensity(1_799), 1);
        assert_eq!(activity_intensity(1_800), 2);
        assert_eq!(activity_intensity(3_600), 3);
        assert_eq!(activity_intensity(7_200), 4);
    }

    #[test]
    fn starts_usage_week_on_monday() {
        assert_eq!(
            week_start(NaiveDate::from_ymd_opt(2026, 7, 30).unwrap()),
            NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()
        );
        assert_eq!(
            week_start(NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()),
            NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()
        );
    }
}

fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

#[tauri::command]
pub async fn get_daily_summary(state: State<'_, AppState>) -> Result<DailySummary, String> {
    let date = Local::now().date_naive();
    let date_text = date.to_string();

    let active_seconds: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE date(started_at, 'localtime') = ?",
    )
    .bind(&date_text)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let xp_earned: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)
         FROM xp_events
         WHERE date(occurred_at, 'localtime') = ?",
    )
    .bind(&date_text)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let ai_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_usage_events WHERE date(occurred_at) = ?")
            .bind(&date_text)
            .fetch_one(&state.pool)
            .await
            .map_err(|error| error.to_string())?;

    Ok(DailySummary {
        date: date_text,
        active_seconds,
        xp_earned,
        ai_events,
    })
}

#[tauri::command]
pub async fn get_focus_activity_today(
    state: State<'_, AppState>,
) -> Result<FocusActivityToday, String> {
    let date = Local::now().date_naive().to_string();
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT source, COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') = ?
         GROUP BY source
         ORDER BY SUM(active_seconds) DESC, MAX(started_at) DESC",
    )
    .bind(&date)
    .fetch_all(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let last_source: Option<String> = sqlx::query_scalar(
        "SELECT source
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') = ?
         ORDER BY started_at DESC
         LIMIT 1",
    )
    .bind(&date)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let mut totals = HashMap::<String, i64>::new();
    for (source, active_seconds) in rows {
        let app_name = activity::catalog::display_name(source_identifier(&source));
        *totals.entry(app_name).or_default() += active_seconds;
    }
    let mut apps: Vec<_> = totals
        .into_iter()
        .map(|(app_name, active_seconds)| FocusAppUsage {
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

    Ok(FocusActivityToday {
        last_app_name: last_source
            .as_deref()
            .map(source_identifier)
            .map(activity::catalog::display_name),
        apps,
    })
}

#[tauri::command]
pub async fn get_activity_history(
    state: State<'_, AppState>,
) -> Result<Vec<ActivityHistoryDay>, String> {
    const HISTORY_DAYS: i64 = 20 * 7;

    let today = Local::now().date_naive();
    let start = today - Duration::days(HISTORY_DAYS - 1);
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT date(started_at, 'localtime'), COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'
           AND date(started_at, 'localtime') >= ?
         GROUP BY date(started_at, 'localtime')",
    )
    .bind(start.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let totals: HashMap<_, _> = rows.into_iter().collect();

    Ok((0..HISTORY_DAYS)
        .map(|offset| {
            let date = start + Duration::days(offset);
            let date_text = date.to_string();
            let active_seconds = totals.get(&date_text).copied().unwrap_or(0);
            ActivityHistoryDay {
                date: date_text,
                active_seconds,
                intensity: activity_intensity(active_seconds),
            }
        })
        .collect())
}

fn activity_intensity(active_seconds: i64) -> u8 {
    match active_seconds {
        0 => 0,
        1..1_800 => 1,
        1_800..3_600 => 2,
        3_600..7_200 => 3,
        _ => 4,
    }
}

fn source_identifier(source: &str) -> &str {
    source.strip_prefix("foreground:").unwrap_or(source)
}

#[tauri::command]
pub async fn get_character_state(state: State<'_, AppState>) -> Result<CharacterState, String> {
    let row: (i64, i64, String) =
        sqlx::query_as("SELECT level, total_xp, current_form FROM character_state WHERE id = 1")
            .fetch_one(&state.pool)
            .await
            .map_err(|error| error.to_string())?;

    let level_floor = (row.0 - 1) * 100;
    Ok(CharacterState {
        level: row.0,
        total_xp: row.1,
        current_form: row.2,
        xp_into_level: row.1 - level_floor,
        xp_for_next_level: 100,
    })
}

#[tauri::command]
pub async fn get_ai_usage_today(state: State<'_, AppState>) -> Result<AiUsageToday, String> {
    if !adapters::codex::is_enabled(&state.pool).await? {
        return Ok(AiUsageToday {
            provider: "codex".to_string(),
            total_tokens: None,
            week_tokens: None,
            source: None,
            last_synced_at: None,
            status: "disconnected".to_string(),
            error: None,
            account_label: None,
            environment: None,
            latest_available_date: None,
            latest_available_tokens: None,
        });
    }

    let today = Local::now().date_naive();
    let date = today.to_string();
    let week_started_at = week_start(today).to_string();
    let snapshot: Option<(i64, String)> = sqlx::query_as(
        "SELECT total_tokens, source
         FROM ai_usage_snapshots
         WHERE provider = 'codex' AND scope = 'account-day' AND bucket_started_at = ?
         ORDER BY observed_at DESC
         LIMIT 1",
    )
    .bind(&date)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let adapter: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT last_success_at, last_error
         FROM ai_adapter_state WHERE adapter_id = 'codex-account-usage'",
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let latest: Option<(String, i64)> = sqlx::query_as(
        "SELECT bucket_started_at, total_tokens
         FROM ai_usage_snapshots
         WHERE provider = 'codex' AND scope = 'account-day'
         ORDER BY bucket_started_at DESC, observed_at DESC
         LIMIT 1",
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let week_tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_tokens), 0)
         FROM (
           SELECT total_tokens,
                  ROW_NUMBER() OVER (
                    PARTITION BY bucket_started_at ORDER BY observed_at DESC
                  ) AS row_number
           FROM ai_usage_snapshots
           WHERE provider = 'codex'
             AND scope = 'account-day'
             AND bucket_started_at BETWEEN ? AND ?
         )
         WHERE row_number = 1",
    )
    .bind(&week_started_at)
    .bind(&date)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let (last_synced_at, error) = adapter.unwrap_or((None, None));
    let status = if error.is_some() {
        "error"
    } else if last_synced_at.is_some() && snapshot.is_none() && latest.is_some() {
        "delayed"
    } else if last_synced_at.is_some() {
        "connected"
    } else {
        "syncing"
    };
    let account_label: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.codex.account_label'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| error.to_string())?;
    let environment: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.codex.environment'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| error.to_string())?;

    Ok(AiUsageToday {
        provider: "codex".to_string(),
        total_tokens: snapshot.as_ref().map(|row| row.0),
        week_tokens: Some(week_tokens),
        source: snapshot.map(|row| row.1),
        last_synced_at,
        status: status.to_string(),
        error,
        account_label,
        environment,
        latest_available_date: latest.as_ref().map(|row| row.0.clone()),
        latest_available_tokens: latest.map(|row| row.1),
    })
}

#[tauri::command]
pub async fn set_codex_usage_enabled(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if enabled {
        return Err("계정 확인 후 연동해야 합니다.".to_string());
    }
    adapters::codex::set_enabled(&state.pool, enabled).await?;
    Ok(())
}

#[tauri::command]
pub async fn preview_codex_account() -> Result<CodexAccountPreview, String> {
    let account = adapters::codex::read_account().await?;
    let environment = codex_environment();

    match account.account {
        Some(adapters::codex::Account::ChatGpt { email, plan_type }) => Ok(CodexAccountPreview {
            account_label: email.unwrap_or_else(|| "이메일 정보 없음".to_string()),
            auth_type: "ChatGPT".to_string(),
            plan_type: Some(plan_type),
            environment,
        }),
        Some(adapters::codex::Account::ApiKey) => {
            Err("API 키 로그인은 계정 사용량 조회를 지원하지 않습니다.".to_string())
        }
        Some(adapters::codex::Account::AmazonBedrock) => {
            Err("Amazon Bedrock 로그인은 계정 사용량 조회를 지원하지 않습니다.".to_string())
        }
        None if account.requires_openai_auth => {
            Err("Codex에 로그인되어 있지 않습니다. Codex CLI에서 먼저 로그인해 주세요.".to_string())
        }
        None => Err("사용 가능한 Codex 계정을 찾지 못했습니다.".to_string()),
    }
}

#[tauri::command]
pub async fn connect_codex_account(state: State<'_, AppState>) -> Result<(), String> {
    let preview = preview_codex_account().await?;
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|error| error.to_string())?;
    for (key, value) in [
        ("ai.codex.enabled", "true".to_string()),
        ("ai.codex.account_label", preview.account_label),
        ("ai.codex.environment", preview.environment),
    ] {
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;

    if let Err(error) = adapters::codex::sync(&state.pool).await {
        adapters::codex::set_enabled(&state.pool, false).await?;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn preview_claude_connection() -> Result<ClaudeConnectionPreview, String> {
    let preview = adapters::claude::preview()?;
    Ok(ClaudeConnectionPreview {
        settings_path: preview.settings_path,
        has_conflicts: preview.has_conflicts,
    })
}

#[tauri::command]
pub async fn connect_claude(state: State<'_, AppState>) -> Result<(), String> {
    adapters::claude::connect(&state.pool).await
}

#[tauri::command]
pub async fn disconnect_claude(state: State<'_, AppState>) -> Result<(), String> {
    adapters::claude::disconnect(&state.pool).await
}

#[tauri::command]
pub async fn get_claude_usage_today(
    state: State<'_, AppState>,
) -> Result<ClaudeUsageToday, String> {
    if !adapters::claude::is_enabled(&state.pool).await? {
        return Ok(ClaudeUsageToday {
            provider: "claude".to_string(),
            total_tokens: 0,
            week_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            session_count: 0,
            last_received_at: None,
            status: "disconnected".to_string(),
            error: None,
        });
    }

    let today = Local::now().date_naive();
    let date = today.to_string();
    let week_started_at = week_start(today).to_string();
    let totals: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(cached_tokens), 0),
            COALESCE(SUM(cache_write_input_tokens), 0)
         FROM ai_usage_events
         WHERE provider = 'claude' AND date(occurred_at, 'localtime') = ?",
    )
    .bind(&date)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let week_tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_tokens), 0)
         FROM ai_usage_events
         WHERE provider = 'claude'
           AND date(occurred_at, 'localtime') BETWEEN ? AND ?",
    )
    .bind(&week_started_at)
    .bind(&date)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let adapter: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT last_success_at, last_error
         FROM ai_adapter_state WHERE adapter_id = 'claude-otel'",
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let (last_received_at, error) = adapter.unwrap_or((None, None));
    let session_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT session_key)
         FROM ai_usage_events
         WHERE provider = 'claude'
           AND session_key IS NOT NULL
           AND date(occurred_at, 'localtime') = ?",
    )
    .bind(&date)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;
    let status = if error.is_some() {
        "error"
    } else if last_received_at.is_some() {
        "connected"
    } else {
        "waiting"
    };

    Ok(ClaudeUsageToday {
        provider: "claude".to_string(),
        total_tokens: totals.0,
        week_tokens,
        input_tokens: totals.1,
        output_tokens: totals.2,
        cached_tokens: totals.3,
        cache_write_tokens: totals.4,
        session_count,
        last_received_at,
        status: status.to_string(),
        error,
    })
}

#[tauri::command]
pub async fn get_ai_activity_status(
    state: State<'_, AppState>,
) -> Result<AiActivityStatus, String> {
    let cutoff = Utc::now() - Duration::minutes(15);
    let claude_active_sessions: i64 = if adapters::claude::is_enabled(&state.pool).await? {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT session_key)
             FROM ai_usage_events
             WHERE provider = 'claude'
               AND session_key IS NOT NULL
               AND datetime(occurred_at) >= datetime(?)",
        )
        .bind(cutoff.to_rfc3339())
        .fetch_one(&state.pool)
        .await
        .map_err(|error| error.to_string())?
    } else {
        0
    };
    let claude_active = claude_active_sessions > 0;

    let codex_active = if adapters::codex::is_enabled(&state.pool).await? {
        let date = Local::now().date_naive().to_string();
        let snapshots: Vec<(i64, String)> = sqlx::query_as(
            "SELECT total_tokens, observed_at
             FROM ai_usage_snapshots
             WHERE provider = 'codex'
               AND scope = 'account-day'
               AND bucket_started_at = ?
             ORDER BY observed_at DESC
             LIMIT 2",
        )
        .bind(date)
        .fetch_all(&state.pool)
        .await
        .map_err(|error| error.to_string())?;
        match snapshots.as_slice() {
            [latest, previous] => {
                latest.0 > previous.0
                    && DateTime::parse_from_rfc3339(&latest.1)
                        .map(|observed| observed.with_timezone(&Utc) >= cutoff)
                        .unwrap_or(false)
            }
            _ => false,
        }
    } else {
        false
    };

    Ok(AiActivityStatus {
        active_provider_count: i64::from(codex_active) + i64::from(claude_active),
        codex_active,
        claude_active,
        claude_active_sessions,
    })
}

fn codex_environment() -> String {
    if std::env::var_os("CODEX_HOME").is_some() {
        "사용자 지정 CODEX_HOME".to_string()
    } else {
        "기본 Codex 환경".to_string()
    }
}
