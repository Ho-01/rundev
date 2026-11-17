use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    label: String,
    used_percent: f64,
    resets_at: Option<i64>,
    window_minutes: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderLimits {
    provider: String,
    status: String,
    windows: Vec<UsageWindow>,
    last_synced_at: Option<String>,
    error: Option<String>,
}

impl ProviderLimits {
    fn empty(provider: &str, status: &str) -> Self {
        Self {
            provider: provider.into(),
            status: status.into(),
            windows: vec![],
            last_synced_at: None,
            error: None,
        }
    }

    fn update(&mut self, result: Result<Vec<UsageWindow>, String>) {
        match result {
            Ok(windows) => {
                self.windows = windows;
                self.status = if self.windows.is_empty() {
                    "unavailable"
                } else {
                    "connected"
                }
                .into();
                self.last_synced_at = Some(Utc::now().to_rfc3339());
                self.error = None;
            }
            Err(error) => {
                self.status = "error".into();
                self.error = Some(error);
            }
        }
    }
}

#[derive(Default)]
struct Cache {
    views: HashMap<String, ProviderLimits>,
    attempts: HashMap<String, Instant>,
}
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
static SYNC: OnceLock<Mutex<()>> = OnceLock::new();
fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}

pub async fn views(pool: &SqlitePool) -> Result<Vec<ProviderLimits>, String> {
    let mut result = vec![];
    for (provider, enabled) in [
        ("codex", super::codex::is_enabled(pool).await?),
        ("claude", super::claude::is_enabled(pool).await?),
    ] {
        let mut state = cache().lock().await;
        if !enabled {
            state.views.remove(provider);
            state.attempts.remove(provider);
            result.push(ProviderLimits::empty(provider, "disconnected"));
        } else {
            result.push(
                state
                    .views
                    .get(provider)
                    .cloned()
                    .unwrap_or_else(|| ProviderLimits::empty(provider, "syncing")),
            );
        }
    }
    result.push(cursor_view(super::cursor::get_usage(pool).await?));
    Ok(result)
}

pub fn start(pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        loop {
            refresh(&pool).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

fn cursor_view(usage: super::cursor::UsageView) -> ProviderLimits {
    if usage.status == "disconnected" {
        return ProviderLimits::empty("cursor", "disconnected");
    }
    let ratio = usage
        .limit_requests
        .filter(|v| *v > 0.0)
        .zip(usage.used_requests)
        .map(|(limit, used)| used / limit * 100.0)
        .or_else(|| {
            usage
                .limit_microusd
                .filter(|v| *v > 0)
                .zip(usage.used_microusd)
                .map(|(limit, used)| used as f64 / limit as f64 * 100.0)
        });
    let percent = ratio.or_else(|| {
        usage
            .auto_percent
            .into_iter()
            .chain(usage.api_percent)
            .filter(|v| v.is_finite())
            .reduce(f64::max)
    });
    let reset = usage
        .cycle_ends_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|v| v.timestamp());
    let windows: Vec<_> = window("월간 한도".into(), percent, reset, 0)
        .into_iter()
        .collect();
    let status = match usage.status.as_str() {
        "syncing" => "syncing",
        "connected" if windows.is_empty() => "unavailable",
        "connected" => "connected",
        _ => "error",
    };
    ProviderLimits {
        provider: "cursor".into(),
        status: status.into(),
        windows,
        last_synced_at: usage.last_synced_at,
        error: usage
            .error_code
            .map(|_| "Cursor 계정 연결과 갱신 상태를 확인해 주세요.".into()),
    }
}

async fn due(provider: &str) -> bool {
    let mut state = cache().lock().await;
    let rate_limited = state.views.get(provider).and_then(|v| v.error.as_deref())
        == Some("요청 제한 · 잠시 후 재시도합니다.");
    let interval = Duration::from_secs(if rate_limited { 300 } else { 60 });
    if state
        .attempts
        .get(provider)
        .is_some_and(|t| t.elapsed() < interval)
    {
        return false;
    }
    state.attempts.insert(provider.into(), Instant::now());
    true
}

pub async fn refresh(pool: &SqlitePool) {
    let Ok(_guard) = SYNC.get_or_init(|| Mutex::new(())).try_lock() else {
        return;
    };
    let codex = async {
        if super::codex::is_enabled(pool).await.unwrap_or(false) && due("codex").await {
            let result = match tokio::time::timeout(
                Duration::from_secs(20),
                super::codex::request("account/rateLimits/read", json!({})),
            )
            .await
            {
                Ok(Ok(value)) => parse_codex(&value),
                // Never expose CLI responses or credential-bearing messages to React/logs.
                _ => Err(
                    "Codex 한도를 조회하지 못했습니다. Codex 로그인 상태를 확인해 주세요.".into(),
                ),
            };
            store(pool, "codex", result).await;
        }
    };
    let claude = async {
        if super::claude::is_enabled(pool).await.unwrap_or(false) && due("claude").await {
            store(pool, "claude", fetch_claude().await).await;
        }
    };
    tokio::join!(codex, claude);
}

async fn store(pool: &SqlitePool, provider: &str, result: Result<Vec<UsageWindow>, String>) {
    let enabled = if provider == "codex" {
        super::codex::is_enabled(pool).await
    } else {
        super::claude::is_enabled(pool).await
    };
    if !enabled.unwrap_or(false) {
        return;
    }
    cache()
        .lock()
        .await
        .views
        .entry(provider.into())
        .or_insert_with(|| ProviderLimits::empty(provider, "syncing"))
        .update(result);
}

fn window(
    label: String,
    percent: Option<f64>,
    reset: Option<i64>,
    minutes: i64,
) -> Option<UsageWindow> {
    let percent = percent.filter(|v| v.is_finite() && *v >= 0.0)?;
    Some(UsageWindow {
        label,
        used_percent: percent.clamp(0.0, 100.0),
        resets_at: reset.filter(|v| *v > 0),
        window_minutes: minutes,
    })
}

fn parse_codex(value: &Value) -> Result<Vec<UsageWindow>, String> {
    // Prefer the authoritative named buckets. Do not also add the legacy duplicate.
    let buckets: Vec<(&str, &Value)> = if let Some(map) = value
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .filter(|map| !map.is_empty())
    {
        map.iter().map(|(key, v)| (key.as_str(), v)).collect()
    } else {
        value
            .get("rateLimits")
            .filter(|v| v.is_object())
            .map(|v| vec![("codex", v)])
            .unwrap_or_default()
    };
    let mut result = vec![];
    for (id, bucket) in buckets {
        for field in ["primary", "secondary"] {
            let v = &bucket[field];
            let minutes = v["windowDurationMins"].as_i64().unwrap_or(0);
            let period = match minutes {
                300 => "5시간".into(),
                10080 => "주간".into(),
                0 => "한도".into(),
                m => format!("{}분", m),
            };
            let label = if id == "codex" {
                period
            } else {
                format!(
                    "{} · {}",
                    bucket["limitName"].as_str().unwrap_or(id),
                    period
                )
            };
            if let Some(w) = window(
                label,
                v["usedPercent"].as_f64(),
                v["resetsAt"].as_i64(),
                minutes,
            ) {
                result.push(w);
            }
        }
    }
    if value.get("rateLimits").is_none() && value.get("rateLimitsByLimitId").is_none() {
        return Err("Codex 한도 응답 형식을 확인할 수 없습니다.".into());
    }
    Ok(result)
}

fn parse_claude(value: &Value) -> Result<Vec<UsageWindow>, String> {
    if value.get("five_hour").is_none() && value.get("seven_day").is_none() {
        return Err("Claude 한도 응답 형식을 확인할 수 없습니다.".into());
    }
    Ok([("five_hour", "5시간", 300), ("seven_day", "주간", 10080)]
        .into_iter()
        .filter_map(|(key, label, minutes)| {
            let v = &value[key];
            let reset = v["resets_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|t| t.timestamp());
            window(label.into(), v["utilization"].as_f64(), reset, minutes)
        })
        .collect())
}

async fn claude_credentials() -> Result<Zeroizing<String>, String> {
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let directory = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| home.map(|p| p.join(".claude")));
    if let Some(path) = directory.map(|p| p.join(".credentials.json")) {
        if let Ok(text) = std::fs::read_to_string(path) {
            return Ok(Zeroizing::new(text));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("security")
                .args([
                    "find-generic-password",
                    "-s",
                    "Claude Code-credentials",
                    "-w",
                ])
                .output(),
        )
        .await;
        if let Ok(Ok(output)) = output {
            if output.status.success() {
                return String::from_utf8(output.stdout)
                    .map(Zeroizing::new)
                    .map_err(|_| "Claude 로그인 정보를 읽지 못했습니다.".into());
            }
        }
    }
    Err("Claude Code에서 로그인해 주세요. 로컬 로그인 정보를 찾지 못했습니다.".into())
}

async fn fetch_claude() -> Result<Vec<UsageWindow>, String> {
    let credentials = claude_credentials().await?;
    #[derive(Deserialize)]
    struct Credentials<'a> {
        #[serde(rename = "claudeAiOauth", borrow)]
        oauth: OAuth<'a>,
    }
    #[derive(Deserialize)]
    struct OAuth<'a> {
        #[serde(rename = "accessToken", borrow)]
        access_token: &'a str,
    }
    // Borrow only the required field; never allocate copies of refresh tokens or other credentials.
    let value: Credentials<'_> = serde_json::from_str(&credentials)
        .map_err(|_| "Claude 로그인 정보 형식을 확인해 주세요.")?;
    if value.oauth.access_token.is_empty() {
        return Err("Claude Code에서 구독 계정으로 로그인해 주세요.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "한도 조회를 시작할 수 없습니다.")?;
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(value.oauth.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/2.1.0")
        .send()
        .await
        .map_err(|_| "Claude 한도 서버에 연결하지 못했습니다.")?;
    match response.status().as_u16() {
        200 => {}
        401 | 403 => {
            return Err(
                "Claude Code에서 다시 로그인해 주세요. 계정 한도 조회 권한이 필요합니다.".into(),
            )
        }
        429 => return Err("요청 제한 · 잠시 후 재시도합니다.".into()),
        _ => return Err("Claude 한도 서버가 응답하지 않습니다.".into()),
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|_| "Claude 한도 응답을 읽지 못했습니다.")?;
    parse_claude(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Requires locally signed-in Codex and Claude accounts; reads quota metadata only"]
    async fn live_connected_account_limits() {
        let value = super::super::codex::request("account/rateLimits/read", json!({}))
            .await
            .unwrap();
        let codex = parse_codex(&value).unwrap();
        let claude = fetch_claude().await.unwrap();
        assert!(!codex.is_empty());
        assert!(!claude.is_empty());
        println!(
            "Codex quota metadata: {}",
            serde_json::to_string(&codex).unwrap()
        );
        println!(
            "Claude quota metadata: {}",
            serde_json::to_string(&claude).unwrap()
        );
    }

    #[tokio::test]
    async fn repeated_polling_does_not_repeat_provider_requests() {
        assert!(due("test-throttle").await);
        assert!(!due("test-throttle").await);
        cache().lock().await.attempts.remove("test-throttle");
    }

    #[test]
    fn cursor_uses_overall_plan_ratio_and_preserves_real_zero() {
        let mut usage = super::super::cursor::UsageView {
            provider: "cursor",
            status: "connected".into(),
            account_label: None,
            used_microusd: Some(0),
            limit_microusd: Some(20000000),
            remaining_microusd: None,
            used_requests: None,
            limit_requests: None,
            remaining_requests: None,
            today_requests: None,
            auto_percent: None,
            api_percent: None,
            today_microusd: None,
            total_tokens: None,
            week_tokens: None,
            cycle_ends_at: Some("2026-12-01T00:00:00Z".into()),
            last_synced_at: None,
            error_code: None,
        };
        assert_eq!(cursor_view(usage.clone()).windows[0].used_percent, 0.0);
        usage.used_microusd = Some(10000000);
        usage.api_percent = Some(10.0);
        let view = cursor_view(usage.clone());
        assert_eq!(view.windows[0].used_percent, 50.0);
        assert!(view.windows[0].resets_at.is_some());
        usage.status = "disconnected".into();
        assert!(cursor_view(usage).windows.is_empty());
    }
    #[test]
    fn codex_named_buckets_override_legacy_and_use_actual_window() {
        let windows = parse_codex(&json!({"rateLimits":{"primary":{"usedPercent":99}},"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":8,"windowDurationMins":10080,"resetsAt":1795493211},"secondary":null}}})).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].used_percent, 8.0);
        assert_eq!(windows[0].label, "주간");
        assert_eq!(windows[0].resets_at, Some(1795493211));
    }
    #[test]
    fn missing_quota_is_not_reported_as_zero() {
        assert!(parse_codex(&json!({})).is_err());
        assert!(parse_claude(&json!({"error":"denied"})).is_err());
        assert!(parse_claude(&json!({"five_hour":null,"seven_day":null}))
            .unwrap()
            .is_empty());
    }
    #[test]
    fn claude_preserves_zero_and_parses_reset_timestamp() {
        let windows = parse_claude(&json!({"five_hour":{"utilization":0,"resets_at":"2026-11-17T05:29:59Z"},"seven_day":{"utilization":56,"resets_at":null}})).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].used_percent, 0.0);
        assert!(windows[0].resets_at.is_some());
        assert_eq!(windows[1].used_percent, 56.0);
        assert_eq!(windows[1].resets_at, None);
    }
    #[test]
    fn failure_keeps_last_success_but_marks_error() {
        let mut view = ProviderLimits::empty("claude", "syncing");
        view.update(parse_claude(&json!({"five_hour":{"utilization":56}})));
        let synced = view.last_synced_at.clone();
        view.update(Err("조회 실패".into()));
        assert_eq!(view.status, "error");
        assert_eq!(view.windows[0].used_percent, 56.0);
        assert_eq!(view.last_synced_at, synced);
    }
}
