use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
#[cfg(any(target_os = "macos", test))]
use std::path::Path;
use std::{path::PathBuf, process::Stdio, sync::OnceLock, time::Instant};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdout, Command},
    sync::Mutex,
    time::{timeout, Duration},
};
use uuid::Uuid;

const ADAPTER_ID: &str = "codex-account-usage";
const SOURCE: &str = "codex-app-server";
const ENABLED_SETTING: &str = "ai.codex.enabled";
static SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static LAST_ATTEMPT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountUsage {
    daily_usage_buckets: Option<Vec<DailyUsageBucket>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyUsageBucket {
    start_date: NaiveDate,
    tokens: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub account: Option<Account>,
    pub requires_openai_auth: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Account {
    #[serde(rename = "chatgpt")]
    ChatGpt {
        email: Option<String>,
        #[serde(rename = "planType")]
        plan_type: String,
    },
    #[serde(rename = "apiKey")]
    ApiKey,
    #[serde(rename = "amazonBedrock")]
    AmazonBedrock,
}

pub async fn sync(pool: &SqlitePool) -> Result<(), String> {
    sync_if_due(pool, Duration::ZERO).await
}

pub async fn sync_if_due(pool: &SqlitePool, minimum_interval: Duration) -> Result<(), String> {
    let lock = SYNC_LOCK.get_or_init(|| Mutex::new(()));
    let Ok(_guard) = lock.try_lock() else {
        return Ok(());
    };
    {
        let attempts = LAST_ATTEMPT.get_or_init(|| Mutex::new(None));
        let mut last = attempts.lock().await;
        if last
            .as_ref()
            .is_some_and(|instant| instant.elapsed() < minimum_interval)
        {
            return Ok(());
        }
        *last = Some(Instant::now());
    }
    let result = fetch().await;
    match result {
        Ok(usage) => {
            persist(pool, usage).await?;
            set_success(pool).await
        }
        Err(error) => {
            set_error(pool, &error).await;
            Err(error)
        }
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

pub async fn set_enabled(pool: &SqlitePool, enabled: bool) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(ENABLED_SETTING)
    .bind(if enabled { "true" } else { "false" })
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn fetch() -> Result<AccountUsage, String> {
    let value = request("account/usage/read", json!({})).await?;
    serde_json::from_value(value)
        .map_err(|error| format!("Codex 사용량 응답 형식이 올바르지 않습니다: {error}"))
}

pub async fn read_account() -> Result<AccountInfo, String> {
    let value = request("account/read", json!({ "refreshToken": false })).await?;
    serde_json::from_value(value)
        .map_err(|error| format!("Codex 계정 응답 형식이 올바르지 않습니다: {error}"))
}

async fn request(method: &str, params: Value) -> Result<Value, String> {
    let program = find_codex_program().ok_or_else(|| {
        "Codex CLI를 찾을 수 없습니다. Codex를 설치하거나 CODEX_PATH를 설정해 주세요.".to_string()
    })?;
    let mut command = Command::new(program);
    command
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    #[cfg(windows)]
    command.creation_flags(0x08000000);

    let mut child = command
        .spawn()
        .map_err(|error| format!("Codex를 실행할 수 없습니다: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("Codex stdin을 열 수 없습니다.")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Codex stdout을 열 수 없습니다.")?;
    let mut reader = BufReader::new(stdout);

    send(
        &mut stdin,
        json!({
            "method": "initialize",
            "id": 1,
            "params": {
                "clientInfo": {
                    "name": "rundev",
                    "title": "RunDev",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )
    .await?;
    read_result(&mut reader, 1).await?;

    send(&mut stdin, json!({ "method": "initialized", "params": {} })).await?;
    send(
        &mut stdin,
        json!({ "method": method, "id": 2, "params": params }),
    )
    .await?;

    let value = read_result(&mut reader, 2).await?;
    let _ = child.kill().await;
    Ok(value)
}

fn find_codex_program() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CODEX_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let executable = if cfg!(windows) { "codex.cmd" } else { "codex" };
    if let Some(path) = find_in_path(executable) {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    if let Some(path) = find_codex_on_macos() {
        return Some(path);
    }

    None
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

#[cfg(target_os = "macos")]
fn find_codex_on_macos() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    macos_codex_candidates(home.as_deref())
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| find_version_managed_codex(home.as_deref()))
}

#[cfg(any(target_os = "macos", test))]
fn macos_codex_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ];
    if let Some(home) = home {
        for relative in [
            ".local/bin/codex",
            ".npm-global/bin/codex",
            ".bun/bin/codex",
            ".volta/bin/codex",
            ".asdf/shims/codex",
            ".nix-profile/bin/codex",
            "Library/pnpm/codex",
        ] {
            candidates.push(home.join(relative));
        }
    }
    candidates
}

#[cfg(any(target_os = "macos", test))]
#[cfg_attr(all(test, not(target_os = "macos")), allow(dead_code))]
fn find_version_managed_codex(home: Option<&Path>) -> Option<PathBuf> {
    let home = home?;
    let mut roots = vec![home.join(".nvm/versions/node")];
    roots.push(home.join("Library/Application Support/fnm/node-versions"));

    let mut candidates = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            candidates.push(entry.path().join("bin/codex"));
            candidates.push(entry.path().join("installation/bin/codex"));
        }
    }
    candidates.sort();
    candidates.into_iter().rev().find(|path| path.is_file())
}

async fn send(stdin: &mut tokio::process::ChildStdin, message: Value) -> Result<(), String> {
    stdin
        .write_all(format!("{message}\n").as_bytes())
        .await
        .map_err(|error| format!("Codex에 요청을 보낼 수 없습니다: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Codex 요청을 전송할 수 없습니다: {error}"))
}

async fn read_result(reader: &mut BufReader<ChildStdout>, id: i64) -> Result<Value, String> {
    timeout(Duration::from_secs(15), async {
        let mut line = String::new();
        loop {
            line.clear();
            let count = reader
                .read_line(&mut line)
                .await
                .map_err(|error| format!("Codex 응답을 읽을 수 없습니다: {error}"))?;
            if count == 0 {
                return Err("Codex가 응답 없이 종료되었습니다.".to_string());
            }
            let message: Value = match serde_json::from_str(&line) {
                Ok(message) => message,
                Err(_) => continue,
            };
            if message.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("Codex 사용량 요청이 실패했습니다: {error}"));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex 응답에 result가 없습니다.".to_string());
        }
    })
    .await
    .map_err(|_| "Codex 사용량 요청 시간이 초과되었습니다.".to_string())?
}

async fn persist(pool: &SqlitePool, usage: AccountUsage) -> Result<(), String> {
    let observed_at: DateTime<Utc> = Utc::now();
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;

    for bucket in usage.daily_usage_buckets.unwrap_or_default() {
        if bucket.tokens < 0 {
            continue;
        }
        sqlx::query(
            "INSERT OR IGNORE INTO ai_usage_snapshots
             (id, provider, source, scope, bucket_started_at, observed_at, total_tokens, confidence)
             VALUES (?, 'codex', ?, 'account-day', ?, ?, ?, 'verified')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(SOURCE)
        .bind(bucket.start_date.to_string())
        .bind(observed_at.to_rfc3339())
        .bind(bucket.tokens)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
}

async fn set_success(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO ai_adapter_state (adapter_id, last_success_at, last_error)
         VALUES (?, ?, NULL)
         ON CONFLICT(adapter_id) DO UPDATE SET
           last_success_at = excluded.last_success_at,
           last_error = NULL",
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
    fn parses_account_usage_response() {
        let usage: AccountUsage = serde_json::from_value(json!({
            "summary": { "lifetimeTokens": 42 },
            "dailyUsageBuckets": [
                { "startDate": "2026-07-28", "tokens": 12345 }
            ]
        }))
        .unwrap();

        let bucket = &usage.daily_usage_buckets.unwrap()[0];
        assert_eq!(bucket.start_date.to_string(), "2026-07-28");
        assert_eq!(bucket.tokens, 12345);
    }

    #[test]
    fn includes_common_macos_codex_locations() {
        let home = Path::new("/Users/tester");
        let candidates = macos_codex_candidates(Some(home));

        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/codex")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/codex")));
        assert!(candidates.contains(&home.join(".npm-global/bin/codex")));
        assert!(candidates.contains(&home.join(".volta/bin/codex")));
        assert!(candidates.contains(&home.join("Library/pnpm/codex")));
    }

    #[tokio::test]
    #[ignore = "requires an installed and authenticated Codex CLI"]
    async fn fetches_usage_from_installed_codex() {
        let account = read_account().await.unwrap();
        assert!(account.account.is_some());
        let usage = fetch().await.unwrap();
        assert!(usage
            .daily_usage_buckets
            .unwrap_or_default()
            .iter()
            .all(|bucket| bucket.tokens >= 0));
    }
}
