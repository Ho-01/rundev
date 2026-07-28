use crate::{adapters, database::AppState, tray};
use chrono::{DateTime, Duration, Local, Utc};
use serde::Serialize;
use tauri::State;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageToday {
    provider: String,
    total_tokens: i64,
    input_tokens: i64,
    output_tokens: i64,
    cached_tokens: i64,
    cache_write_tokens: i64,
    session_count: i64,
    last_received_at: Option<String>,
    status: String,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiActivityStatus {
    active_provider_count: i64,
    codex_active: bool,
    claude_active: bool,
    claude_active_sessions: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSelection {
    runner_id: String,
}

#[tauri::command]
pub async fn get_runner_selection(state: State<'_, AppState>) -> Result<RunnerSelection, String> {
    let runner_id =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'runner.selected'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| "coding-cat".to_string());
    Ok(RunnerSelection { runner_id })
}

#[tauri::command]
pub async fn set_runner_selection(
    runner_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !is_supported_runner(&runner_id) {
        return Err("지원하지 않는 러너입니다.".to_string());
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
    Ok(())
}

fn is_supported_runner(runner_id: &str) -> bool {
    matches!(
        runner_id,
        "coding-cat" | "coding-fish" | "coding-orange-cat" | "coding-white-cat" | "coding-vtuber"
    )
}

#[cfg(test)]
mod tests {
    use super::is_supported_runner;

    #[test]
    fn accepts_only_packaged_runner_ids() {
        assert!(is_supported_runner("coding-cat"));
        assert!(is_supported_runner("coding-fish"));
        assert!(is_supported_runner("coding-orange-cat"));
        assert!(is_supported_runner("coding-white-cat"));
        assert!(is_supported_runner("coding-vtuber"));
        assert!(!is_supported_runner("../custom"));
    }
}

#[tauri::command]
pub async fn get_daily_summary(state: State<'_, AppState>) -> Result<DailySummary, String> {
    let date = Local::now().date_naive();
    let date_text = date.to_string();

    let active_seconds: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(active_seconds), 0) FROM activity_sessions WHERE date(started_at) = ?",
    )
    .bind(&date_text)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let xp_earned: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM xp_events WHERE date(occurred_at) = ?",
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

    let date = Local::now().date_naive().to_string();
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

    let date = Local::now().date_naive().to_string();
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
