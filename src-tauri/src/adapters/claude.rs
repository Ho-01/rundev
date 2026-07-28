use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

const ADAPTER_ID: &str = "claude-otel";
const SOURCE: &str = "claude-otel-api-request";
const ENDPOINT: &str = "http://127.0.0.1:43182/v1/logs";
const LISTEN_ADDRESS: &str = "127.0.0.1:43182";
const ENABLED_SETTING: &str = "ai.claude.enabled";
const TOKEN_SETTING: &str = "ai.claude.collector_token";
const ORIGINAL_ENV_SETTING: &str = "ai.claude.original_env";

const MANAGED_KEYS: [&str; 11] = [
    "CLAUDE_CODE_ENABLE_TELEMETRY",
    "OTEL_LOGS_EXPORTER",
    "OTEL_EXPORTER_OTLP_LOGS_PROTOCOL",
    "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
    "OTEL_EXPORTER_OTLP_HEADERS",
    "OTEL_LOGS_EXPORT_INTERVAL",
    "OTEL_LOG_USER_PROMPTS",
    "OTEL_LOG_ASSISTANT_RESPONSES",
    "OTEL_LOG_TOOL_DETAILS",
    "OTEL_LOG_RAW_API_BODIES",
    "OTEL_LOG_TOOL_CONTENT",
];

#[derive(Clone)]
struct CollectorState {
    pool: SqlitePool,
}

#[derive(Debug)]
pub struct ConnectionPreview {
    pub settings_path: String,
    pub has_conflicts: bool,
}

pub async fn serve(pool: SqlitePool) {
    let listener = match tokio::net::TcpListener::bind(LISTEN_ADDRESS).await {
        Ok(listener) => listener,
        Err(error) => {
            set_error(
                &pool,
                &format!("Claude 로컬 수집기를 시작할 수 없습니다: {error}"),
            )
            .await;
            return;
        }
    };
    let _ = sqlx::query("UPDATE ai_adapter_state SET last_error = NULL WHERE adapter_id = ?")
        .bind(ADAPTER_ID)
        .execute(&pool)
        .await;
    let router = Router::new()
        .route("/v1/logs", post(receive_logs))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(CollectorState { pool });
    if let Err(error) = axum::serve(listener, router).await {
        tracing::warn!(%error, "Claude OTLP collector stopped");
    }
}

pub async fn is_enabled(pool: &SqlitePool) -> Result<bool, String> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(ENABLED_SETTING)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(value.as_deref() == Some("true"))
}

pub fn preview() -> Result<ConnectionPreview, String> {
    let path = settings_path()?;
    let settings = read_settings(&path)?;
    let env = settings.get("env").and_then(Value::as_object);
    let has_conflicts = MANAGED_KEYS
        .iter()
        .any(|key| env.and_then(|values| values.get(*key)).is_some());
    Ok(ConnectionPreview {
        settings_path: path.display().to_string(),
        has_conflicts,
    })
}

pub async fn connect(pool: &SqlitePool) -> Result<(), String> {
    let path = settings_path()?;
    let mut settings = read_settings(&path)?;
    let root = settings
        .as_object_mut()
        .ok_or("Claude settings.json의 최상위 값이 객체가 아닙니다.")?;
    let env = root
        .entry("env")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("Claude settings.json의 env 값이 객체가 아닙니다.")?;

    let original = MANAGED_KEYS
        .iter()
        .map(|key| {
            (
                (*key).to_string(),
                env.get(*key).cloned().unwrap_or(Value::Null),
            )
        })
        .collect::<Map<_, _>>();
    let token = Uuid::new_v4().to_string();
    let configured = configured_env(&token);
    for (key, value) in configured {
        env.insert(key, Value::String(value));
    }

    write_settings(&path, &settings)?;
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, value) in [
        (ENABLED_SETTING, "true".to_string()),
        (TOKEN_SETTING, token),
        (ORIGINAL_ENV_SETTING, Value::Object(original).to_string()),
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
        .map_err(|error| error.to_string())
}

pub async fn disconnect(pool: &SqlitePool) -> Result<(), String> {
    let path = settings_path()?;
    let token: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(TOKEN_SETTING)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
    let original_text: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
            .bind(ORIGINAL_ENV_SETTING)
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;

    if let (Some(token), Some(original_text)) = (token, original_text) {
        let mut settings = read_settings(&path)?;
        let original: Map<String, Value> =
            serde_json::from_str(&original_text).map_err(|error| error.to_string())?;
        if let Some(env) = settings.get_mut("env").and_then(Value::as_object_mut) {
            let configured = configured_env(&token);
            for key in MANAGED_KEYS {
                let expected = configured.get(key).map(String::as_str);
                let current = env.get(key).and_then(Value::as_str);
                if current != expected {
                    continue;
                }
                match original.get(key) {
                    Some(Value::Null) | None => {
                        env.remove(key);
                    }
                    Some(value) => {
                        env.insert(key.to_string(), value.clone());
                    }
                }
            }
        }
        write_settings(&path, &settings)?;
    }

    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, 'false')
         ON CONFLICT(key) DO UPDATE SET value = 'false'",
    )
    .bind(ENABLED_SETTING)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn configured_env(token: &str) -> HashMap<String, String> {
    [
        ("CLAUDE_CODE_ENABLE_TELEMETRY", "1".to_string()),
        ("OTEL_LOGS_EXPORTER", "otlp".to_string()),
        ("OTEL_EXPORTER_OTLP_LOGS_PROTOCOL", "http/json".to_string()),
        ("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT", ENDPOINT.to_string()),
        (
            "OTEL_EXPORTER_OTLP_HEADERS",
            format!("x-rundev-token={token}"),
        ),
        ("OTEL_LOGS_EXPORT_INTERVAL", "5000".to_string()),
        ("OTEL_LOG_USER_PROMPTS", "0".to_string()),
        ("OTEL_LOG_ASSISTANT_RESPONSES", "0".to_string()),
        ("OTEL_LOG_TOOL_DETAILS", "0".to_string()),
        ("OTEL_LOG_RAW_API_BODIES", "0".to_string()),
        ("OTEL_LOG_TOOL_CONTENT", "0".to_string()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value))
    .collect()
}

fn settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or("사용자 홈 디렉터리를 찾을 수 없습니다.")?;
    Ok(home.join(".claude").join("settings.json"))
}

fn read_settings(path: &PathBuf) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("Claude settings.json을 읽을 수 없습니다: {error}"))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("Claude settings.json이 올바른 JSON이 아닙니다: {error}"))
}

fn write_settings(path: &PathBuf, settings: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Claude 설정 디렉터리를 찾을 수 없습니다.")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Claude 설정 디렉터리를 만들 수 없습니다: {error}"))?;
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("Claude settings.json을 저장할 수 없습니다: {error}"))
}

async fn receive_logs(
    State(state): State<CollectorState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if !authorized(&state.pool, &headers).await {
        return StatusCode::UNAUTHORIZED;
    }
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    match persist_api_requests(&state.pool, &payload).await {
        Ok(_) => StatusCode::OK,
        Err(error) => {
            tracing::warn!(%error, "Failed to persist Claude usage event");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn authorized(pool: &SqlitePool, headers: &HeaderMap) -> bool {
    if !is_enabled(pool).await.unwrap_or(false) {
        return false;
    }
    let expected: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
            .bind(TOKEN_SETTING)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    headers
        .get("x-rundev-token")
        .and_then(|value| value.to_str().ok())
        .zip(expected.as_deref())
        .is_some_and(|(actual, expected)| actual == expected)
}

async fn persist_api_requests(pool: &SqlitePool, payload: &Value) -> Result<usize, String> {
    let mut inserted = 0;
    for resource_log in payload
        .get("resourceLogs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let resource_attributes = attributes(
            resource_log
                .pointer("/resource/attributes")
                .and_then(Value::as_array),
        );
        for scope_log in resource_log
            .get("scopeLogs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for record in scope_log
                .get("logRecords")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let mut values = resource_attributes.clone();
                values.extend(attributes(
                    record.get("attributes").and_then(Value::as_array),
                ));
                let body_name = record.pointer("/body/stringValue").and_then(Value::as_str);
                let event_name = values.get("event.name").and_then(Value::as_str);
                if body_name != Some("claude_code.api_request")
                    && event_name != Some("api_request")
                    && event_name != Some("claude_code.api_request")
                {
                    continue;
                }

                let external_id = string_attr(&values, "request_id")
                    .or_else(|| string_attr(&values, "client_request_id"))
                    .or_else(|| {
                        Some(format!(
                            "{}:{}",
                            string_attr(&values, "session.id")?,
                            string_attr(&values, "event.sequence")?
                        ))
                    });
                let Some(external_id) = external_id else {
                    continue;
                };
                let input = int_attr(&values, "input_tokens").unwrap_or(0);
                let output = int_attr(&values, "output_tokens").unwrap_or(0);
                let cache_read = int_attr(&values, "cache_read_tokens").unwrap_or(0);
                let cache_write = int_attr(&values, "cache_creation_tokens").unwrap_or(0);
                let total = input + output + cache_read + cache_write;
                if total < 0 {
                    continue;
                }
                let occurred_at = string_attr(&values, "event.timestamp")
                    .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
                    .map(|value| value.with_timezone(&Utc))
                    .or_else(|| timestamp_from_record(record))
                    .unwrap_or_else(Utc::now);

                let result = sqlx::query(
                    "INSERT OR IGNORE INTO ai_usage_events
                     (id, provider, occurred_at, input_tokens, output_tokens, cached_tokens,
                      source, confidence, external_event_id, model,
                      cache_write_input_tokens, total_tokens, cost_usd_micros)
                     VALUES (?, 'claude', ?, ?, ?, ?, ?, 'verified', ?, ?, ?, ?, ?)",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(occurred_at.to_rfc3339())
                .bind(input)
                .bind(output)
                .bind(cache_read)
                .bind(SOURCE)
                .bind(external_id)
                .bind(string_attr(&values, "model"))
                .bind(cache_write)
                .bind(total)
                .bind(int_attr(&values, "cost_usd_micros"))
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
                inserted += result.rows_affected() as usize;
            }
        }
    }
    if inserted > 0 {
        set_success(pool).await?;
    }
    Ok(inserted)
}

fn attributes(items: Option<&Vec<Value>>) -> Map<String, Value> {
    items
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let key = item.get("key")?.as_str()?.to_string();
            let value = item.get("value")?;
            let normalized = value
                .get("stringValue")
                .cloned()
                .or_else(|| value.get("intValue").cloned())
                .or_else(|| value.get("doubleValue").cloned())
                .or_else(|| value.get("boolValue").cloned())?;
            Some((key, normalized))
        })
        .collect()
}

fn string_attr(values: &Map<String, Value>, key: &str) -> Option<String> {
    values.get(key).and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    })
}

fn int_attr(values: &Map<String, Value>, key: &str) -> Option<i64> {
    values.get(key).and_then(|value| match value {
        Value::String(value) => value.parse().ok(),
        Value::Number(value) => value.as_i64(),
        _ => None,
    })
}

fn timestamp_from_record(record: &Value) -> Option<DateTime<Utc>> {
    let nanos: i64 = record.get("timeUnixNano")?.as_str()?.parse().ok()?;
    Utc.timestamp_opt(nanos / 1_000_000_000, (nanos % 1_000_000_000) as u32)
        .single()
}

async fn set_success(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO ai_adapter_state (adapter_id, last_success_at, last_error)
         VALUES (?, ?, NULL)
         ON CONFLICT(adapter_id) DO UPDATE SET
           last_success_at = excluded.last_success_at, last_error = NULL",
    )
    .bind(ADAPTER_ID)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn set_error(pool: &SqlitePool, error: &str) {
    let _ = sqlx::query(
        "INSERT INTO ai_adapter_state (adapter_id, last_error)
         VALUES (?, ?)
         ON CONFLICT(adapter_id) DO UPDATE SET last_error = excluded.last_error",
    )
    .bind(ADAPTER_ID)
    .bind(error)
    .execute(pool)
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_otlp_json_attributes() {
        let items = vec![
            json!({"key": "input_tokens", "value": {"intValue": "42"}}),
            json!({"key": "model", "value": {"stringValue": "claude-test"}}),
        ];
        let values = attributes(Some(&items));
        assert_eq!(int_attr(&values, "input_tokens"), Some(42));
        assert_eq!(
            string_attr(&values, "model").as_deref(),
            Some("claude-test")
        );
    }
}
