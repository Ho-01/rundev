use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, USER_AGENT},
    redirect::Policy,
    Client, Method, StatusCode,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteConnection},
    Connection, Row, SqlitePool,
};
use std::{
    path::PathBuf,
    str::FromStr,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

const ADAPTER_ID: &str = "cursor-usage";
const BASE_URL: &str = "https://cursor.com";
const MAX_RESPONSE_BYTES: usize = 1_048_576;
const CONSENT_VERSION: &str = "1";

static SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static LAST_ATTEMPT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static BACKOFF_UNTIL: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

#[derive(Zeroize, ZeroizeOnDrop)]
struct SecretToken(String);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPreview {
    pub account_label: String,
    pub plan_type: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageView {
    pub provider: &'static str,
    pub status: String,
    pub account_label: Option<String>,
    pub used_microusd: Option<i64>,
    pub limit_microusd: Option<i64>,
    pub remaining_microusd: Option<i64>,
    pub used_requests: Option<f64>,
    pub limit_requests: Option<f64>,
    pub remaining_requests: Option<f64>,
    pub today_requests: Option<f64>,
    pub auto_percent: Option<f64>,
    pub api_percent: Option<f64>,
    pub today_microusd: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cycle_ends_at: Option<String>,
    pub last_synced_at: Option<String>,
    pub error_code: Option<String>,
}

struct Account {
    id: String,
    email: Option<String>,
    plan_type: Option<String>,
}

struct Snapshot {
    cycle_started_at: Option<String>,
    cycle_ends_at: Option<String>,
    plan_kind: Option<String>,
    used_microusd: Option<i64>,
    limit_microusd: Option<i64>,
    remaining_microusd: Option<i64>,
    used_requests: Option<f64>,
    limit_requests: Option<f64>,
    remaining_requests: Option<f64>,
    today_requests: Option<f64>,
    auto_percent_basis_points: Option<i64>,
    api_percent_basis_points: Option<i64>,
    today_microusd: Option<i64>,
    total_tokens: Option<i64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cached_tokens: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
enum CursorError {
    ConsentRequired,
    CredentialNotFound,
    CredentialLocked,
    AuthExpired,
    RateLimited,
    NetworkFailed,
    UnsupportedSchema,
    InvalidResponse,
}

impl CursorError {
    fn code(self) -> &'static str {
        match self {
            Self::ConsentRequired => "consent_required",
            Self::CredentialNotFound => "credential_not_found",
            Self::CredentialLocked => "credential_locked",
            Self::AuthExpired => "auth_expired",
            Self::RateLimited => "rate_limited",
            Self::NetworkFailed => "network_failed",
            Self::UnsupportedSchema => "unsupported_schema",
            Self::InvalidResponse => "invalid_response",
        }
    }

    fn user_message(self) -> &'static str {
        match self {
            Self::ConsentRequired => "먼저 Cursor 사용량 조회에 동의해 주세요.",
            Self::CredentialNotFound => {
                "Cursor 로그인 정보를 찾지 못했습니다. Cursor에서 로그인해 주세요."
            }
            Self::CredentialLocked => {
                "Cursor 로그인 정보를 읽을 수 없습니다. 잠시 후 다시 시도해 주세요."
            }
            Self::AuthExpired => "Cursor에서 다시 로그인해 주세요.",
            Self::RateLimited => "Cursor 요청이 제한되었습니다. 잠시 후 다시 시도해 주세요.",
            Self::NetworkFailed => "Cursor 서버에 연결할 수 없습니다.",
            Self::UnsupportedSchema => "Cursor 사용량 형식이 변경되어 동기화를 중단했습니다.",
            Self::InvalidResponse => "Cursor 사용량 응답을 확인할 수 없습니다.",
        }
    }
}

impl std::fmt::Display for CursorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.user_message())
    }
}

pub async fn grant_consent(pool: &SqlitePool) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, value) in [
        ("ai.cursor.consent_version", CONSENT_VERSION.to_string()),
        ("ai.cursor.consent_granted_at", now),
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

pub async fn has_consent(pool: &SqlitePool) -> Result<bool, String> {
    let version: Option<String> = sqlx::query_scalar(
        "SELECT value FROM app_settings WHERE key = 'ai.cursor.consent_version'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(version.as_deref() == Some(CONSENT_VERSION))
}

pub async fn is_enabled(pool: &SqlitePool) -> Result<bool, String> {
    let enabled: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.cursor.enabled'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    Ok(enabled.as_deref() == Some("true"))
}

pub async fn preview(pool: &SqlitePool) -> Result<AccountPreview, String> {
    ensure_consent(pool)
        .await
        .map_err(|error| error.to_string())?;
    let token = read_token().await.map_err(|error| error.to_string())?;
    let account = ClientHandle::new()
        .map_err(|error| error.to_string())?
        .account(&token)
        .await
        .map_err(|error| error.to_string())?;
    Ok(AccountPreview {
        account_label: mask_email(account.email.as_deref()),
        plan_type: account.plan_type,
    })
}

pub async fn connect(pool: &SqlitePool) -> Result<(), String> {
    ensure_consent(pool)
        .await
        .map_err(|error| error.to_string())?;
    let token = read_token().await.map_err(|error| error.to_string())?;
    let client = ClientHandle::new().map_err(|error| error.to_string())?;
    let account = client
        .account(&token)
        .await
        .map_err(|error| error.to_string())?;
    let account_key = account_key(&account.id);
    let snapshot = client
        .snapshot(&token, &account)
        .await
        .map_err(|error| error.to_string())?;
    persist_snapshot(pool, &account_key, snapshot).await?;

    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, value) in [
        ("ai.cursor.enabled", "true".to_string()),
        ("ai.cursor.account_key", account_key),
        (
            "ai.cursor.account_label",
            mask_email(account.email.as_deref()),
        ),
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
    set_success(pool).await
}

pub async fn disconnect(pool: &SqlitePool, revoke_consent: bool) -> Result<(), String> {
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES ('ai.cursor.enabled', 'false')
         ON CONFLICT(key) DO UPDATE SET value = 'false'",
    )
    .execute(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    for key in ["ai.cursor.account_key", "ai.cursor.account_label"] {
        sqlx::query("DELETE FROM app_settings WHERE key = ?")
            .bind(key)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    if revoke_consent {
        sqlx::query("DELETE FROM app_settings WHERE key LIKE 'ai.cursor.consent_%'")
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
}

pub async fn sync(pool: &SqlitePool) -> Result<(), String> {
    sync_if_due(pool, Duration::ZERO).await
}

pub async fn sync_if_due(pool: &SqlitePool, minimum_interval: Duration) -> Result<(), String> {
    sync_with_policy(pool, minimum_interval, true).await
}

pub async fn manual_sync_if_due(
    pool: &SqlitePool,
    minimum_interval: Duration,
) -> Result<(), String> {
    sync_with_policy(pool, minimum_interval, false).await
}

async fn sync_with_policy(
    pool: &SqlitePool,
    minimum_interval: Duration,
    require_automatic_permission: bool,
) -> Result<(), String> {
    if !is_enabled(pool).await? {
        return Ok(());
    }
    if require_automatic_permission && !automatic_sync_allowed(pool).await? {
        return Ok(());
    }
    {
        let backoff = BACKOFF_UNTIL.get_or_init(|| Mutex::new(None));
        if backoff
            .lock()
            .await
            .as_ref()
            .is_some_and(|until| *until > Instant::now())
        {
            return Ok(());
        }
    }
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

    let result = sync_inner(pool).await;
    if let Err(error) = result {
        if matches!(error, CursorError::RateLimited) {
            let backoff = BACKOFF_UNTIL.get_or_init(|| Mutex::new(None));
            *backoff.lock().await = Some(Instant::now() + Duration::from_secs(15 * 60));
        }
        set_error(pool, error.code()).await;
        return Err(error.to_string());
    }
    set_success(pool).await
}

pub async fn automatic_sync_allowed(pool: &SqlitePool) -> Result<bool, String> {
    if !is_enabled(pool).await? {
        return Ok(false);
    }
    let error: Option<String> =
        sqlx::query_scalar("SELECT last_error FROM ai_adapter_state WHERE adapter_id = ?")
            .bind(ADAPTER_ID)
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?
            .flatten();
    Ok(!matches!(
        error.as_deref(),
        Some("auth_expired" | "unsupported_schema")
    ))
}

async fn sync_inner(pool: &SqlitePool) -> Result<(), CursorError> {
    ensure_consent(pool).await?;
    let token = read_token().await?;
    let client = ClientHandle::new()?;
    let account = client.account(&token).await?;
    let key = account_key(&account.id);
    let expected: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.cursor.account_key'")
            .fetch_optional(pool)
            .await
            .map_err(|_| CursorError::InvalidResponse)?;
    if expected.as_deref() != Some(key.as_str()) {
        return Err(CursorError::AuthExpired);
    }
    let snapshot = client.snapshot(&token, &account).await?;
    persist_snapshot(pool, &key, snapshot)
        .await
        .map_err(|_| CursorError::InvalidResponse)
}

pub async fn get_usage(pool: &SqlitePool) -> Result<UsageView, String> {
    if !is_enabled(pool).await? {
        return Ok(disconnected_view());
    }
    let account_key: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.cursor.account_key'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    let account_label: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'ai.cursor.account_label'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    let adapter: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT last_success_at, last_error FROM ai_adapter_state WHERE adapter_id = ?",
    )
    .bind(ADAPTER_ID)
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    let snapshot = if let Some(key) = account_key {
        sqlx::query(
            "SELECT used_microusd, limit_microusd, remaining_microusd,
                    used_requests, limit_requests, remaining_requests, today_requests,
                    auto_percent_basis_points, api_percent_basis_points,
                    today_microusd, total_tokens, cycle_ends_at
             FROM cursor_usage_snapshots
             WHERE account_key = ? ORDER BY observed_at DESC LIMIT 1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?
    } else {
        None
    };
    let (last_synced_at, error_code) = adapter.unwrap_or((None, None));
    let status = match error_code.as_deref() {
        Some("auth_expired") => "reauthRequired",
        Some("rate_limited") => "rateLimited",
        Some("unsupported_schema") => "unsupportedSchema",
        Some(_) if snapshot.is_some() => "stale",
        Some(_) => "error",
        None if snapshot.is_some() => "connected",
        None => "syncing",
    };
    Ok(UsageView {
        provider: "cursor",
        status: status.to_string(),
        account_label,
        used_microusd: snapshot.as_ref().and_then(|row| row.try_get(0).ok()),
        limit_microusd: snapshot.as_ref().and_then(|row| row.try_get(1).ok()),
        remaining_microusd: snapshot.as_ref().and_then(|row| row.try_get(2).ok()),
        used_requests: snapshot.as_ref().and_then(|row| row.try_get(3).ok()),
        limit_requests: snapshot.as_ref().and_then(|row| row.try_get(4).ok()),
        remaining_requests: snapshot.as_ref().and_then(|row| row.try_get(5).ok()),
        today_requests: snapshot.as_ref().and_then(|row| row.try_get(6).ok()),
        auto_percent: snapshot
            .as_ref()
            .and_then(|row| row.try_get::<Option<i64>, _>(7).ok().flatten())
            .map(|value| value as f64 / 100.0),
        api_percent: snapshot
            .as_ref()
            .and_then(|row| row.try_get::<Option<i64>, _>(8).ok().flatten())
            .map(|value| value as f64 / 100.0),
        today_microusd: snapshot.as_ref().and_then(|row| row.try_get(9).ok()),
        total_tokens: snapshot.as_ref().and_then(|row| row.try_get(10).ok()),
        cycle_ends_at: snapshot.as_ref().and_then(|row| row.try_get(11).ok()),
        last_synced_at,
        error_code,
    })
}

pub async fn refresh_codex_and_cursor_on_open(pool: SqlitePool) {
    let codex_pool = pool.clone();
    let cursor_pool = pool;
    let _ = tokio::join!(
        async move {
            if super::codex::is_enabled(&codex_pool).await.unwrap_or(false) {
                super::codex::sync_if_due(&codex_pool, Duration::from_secs(60)).await
            } else {
                Ok(())
            }
        },
        async move { manual_sync_if_due(&cursor_pool, Duration::from_secs(60)).await }
    );
}

fn disconnected_view() -> UsageView {
    UsageView {
        provider: "cursor",
        status: "disconnected".to_string(),
        account_label: None,
        used_microusd: None,
        limit_microusd: None,
        remaining_microusd: None,
        used_requests: None,
        limit_requests: None,
        remaining_requests: None,
        today_requests: None,
        auto_percent: None,
        api_percent: None,
        today_microusd: None,
        total_tokens: None,
        cycle_ends_at: None,
        last_synced_at: None,
        error_code: None,
    }
}

async fn ensure_consent(pool: &SqlitePool) -> Result<(), CursorError> {
    has_consent(pool)
        .await
        .map_err(|_| CursorError::ConsentRequired)?
        .then_some(())
        .ok_or(CursorError::ConsentRequired)
}

async fn read_token() -> Result<SecretToken, CursorError> {
    let path = state_db_path().ok_or(CursorError::CredentialNotFound)?;
    if !path.is_file() {
        return Err(CursorError::CredentialNotFound);
    }
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .map_err(|_| CursorError::CredentialLocked)?
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(Duration::from_millis(750));
    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(|_| CursorError::CredentialLocked)?;
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM ItemTable WHERE key = ? LIMIT 1")
            .bind("cursorAuth/accessToken")
            .fetch_optional(&mut connection)
            .await
            .map_err(|_| CursorError::CredentialLocked)?;
    let token = value
        .map(|value| value.trim().trim_matches('"').to_string())
        .filter(|value| (32..=16_384).contains(&value.len()))
        .ok_or(CursorError::CredentialNotFound)?;
    Ok(SecretToken(token))
}

fn state_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|base| {
            PathBuf::from(base)
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        })
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

struct ClientHandle {
    client: Client,
}

impl ClientHandle {
    fn new() -> Result<Self, CursorError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(12))
            .build()
            .map_err(|_| CursorError::NetworkFailed)?;
        Ok(Self { client })
    }

    async fn account(&self, token: &SecretToken) -> Result<Account, CursorError> {
        let payload = self
            .request(token, Method::GET, "/api/auth/me", None)
            .await?;
        let id = payload
            .get("id")
            .or_else(|| payload.get("sub"))
            .and_then(value_string)
            .filter(|value| !value.is_empty())
            .ok_or(CursorError::InvalidResponse)?;
        Ok(Account {
            id,
            email: payload
                .get("email")
                .and_then(Value::as_str)
                .map(str::to_string),
            plan_type: payload
                .get("membershipType")
                .or_else(|| payload.get("planType"))
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    async fn snapshot(
        &self,
        token: &SecretToken,
        _account: &Account,
    ) -> Result<Snapshot, CursorError> {
        let current_period = self
            .request(
                token,
                Method::POST,
                "/api/dashboard/get-current-period-usage",
                Some(json!({})),
            )
            .await;
        let cycle = match current_period {
            Ok(payload) if has_plan_usage(&payload) => payload,
            Ok(_) | Err(CursorError::InvalidResponse | CursorError::UnsupportedSchema) => {
                self.request(token, Method::GET, "/api/usage-summary", None)
                    .await?
            }
            Err(error) => return Err(error),
        };
        let (start_ms, end_ms) = local_day_bounds_ms()?;
        let events = self.usage_events(token, start_ms, end_ms).await?;
        parse_snapshot(&cycle, &events)
    }

    async fn usage_events(
        &self,
        token: &SecretToken,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Value, CursorError> {
        const PAGE_SIZE: usize = 100;
        const MAX_PAGES: usize = 5;
        let mut all_events = Vec::new();
        for page in 1..=MAX_PAGES {
            let payload = self
                .request(
                    token,
                    Method::POST,
                    "/api/dashboard/get-filtered-usage-events",
                    Some(json!({
                        "startDate": start_ms.to_string(),
                        "endDate": end_ms.to_string(),
                        "page": page,
                        "pageSize": PAGE_SIZE
                    })),
                )
                .await?;
            let events = usage_events_page(&payload)?;
            let page_len = events.len();
            all_events.extend(events.iter().cloned());
            let total = integer(payload.get("totalUsageEventsCount"))
                .and_then(|value| usize::try_from(value).ok());
            if page_len < PAGE_SIZE || total.is_some_and(|value| all_events.len() >= value) {
                break;
            }
        }
        Ok(json!({ "usageEventsDisplay": all_events }))
    }

    async fn request(
        &self,
        token: &SecretToken,
        method: Method,
        path: &'static str,
        body: Option<Value>,
    ) -> Result<Value, CursorError> {
        let cookie = cookie_value(token)?;
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static("RunDev/0.3"));
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("WorkosCursorSessionToken={cookie}"))
                .map_err(|_| CursorError::CredentialNotFound)?,
        );
        let mut request = self
            .client
            .request(method, format!("{BASE_URL}{path}"))
            .headers(headers);
        if let Some(body) = body {
            request = request
                .header(CONTENT_TYPE, "application/json")
                .header(ORIGIN, BASE_URL)
                .json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| CursorError::NetworkFailed)?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(CursorError::AuthExpired)
            }
            StatusCode::TOO_MANY_REQUESTS => return Err(CursorError::RateLimited),
            status if status.is_redirection() => return Err(CursorError::NetworkFailed),
            status if !status.is_success() => return Err(CursorError::InvalidResponse),
            _ => {}
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(CursorError::InvalidResponse);
        }
        let is_json = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json"));
        if !is_json {
            return Err(CursorError::InvalidResponse);
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| CursorError::NetworkFailed)?;
        if bytes.is_empty() || bytes.len() > MAX_RESPONSE_BYTES {
            return Err(CursorError::InvalidResponse);
        }
        serde_json::from_slice(&bytes).map_err(|_| CursorError::UnsupportedSchema)
    }
}

fn usage_events_page(payload: &Value) -> Result<&[Value], CursorError> {
    if let Some(events) = payload.get("usageEventsDisplay").and_then(Value::as_array) {
        return Ok(events);
    }
    if payload.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    Err(CursorError::UnsupportedSchema)
}

fn cookie_value(token: &SecretToken) -> Result<String, CursorError> {
    let normalized = token.0.trim().replace("%3A%3A", "::");
    let full = if normalized.contains("::") {
        normalized
    } else {
        let payload = normalized
            .split('.')
            .nth(1)
            .ok_or(CursorError::CredentialNotFound)?;
        let decoded = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| CursorError::CredentialNotFound)?;
        let claims: Value =
            serde_json::from_slice(&decoded).map_err(|_| CursorError::CredentialNotFound)?;
        if claims.get("type").and_then(Value::as_str) == Some("api_key_token") {
            return Err(CursorError::CredentialNotFound);
        }
        let subject = claims
            .get("sub")
            .and_then(Value::as_str)
            .and_then(|value| value.rsplit('|').next())
            .filter(|value| !value.is_empty())
            .ok_or(CursorError::CredentialNotFound)?;
        format!("{subject}::{normalized}")
    };
    Ok(full.replace("::", "%3A%3A"))
}

fn parse_snapshot(cycle: &Value, event_payload: &Value) -> Result<Snapshot, CursorError> {
    let plan = cycle
        .get("planUsage")
        .or_else(|| cycle.pointer("/individualUsage/plan"))
        .unwrap_or(cycle);
    let on_demand = cycle.pointer("/individualUsage/onDemand");
    let on_demand_enabled = on_demand
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let used_microusd = on_demand_enabled
        .then(|| money_microusd(on_demand.and_then(|value| value.get("used"))))
        .flatten();
    let limit_microusd = on_demand_enabled
        .then(|| money_microusd(on_demand.and_then(|value| value.get("limit"))))
        .flatten();
    let remaining_microusd = on_demand_enabled
        .then(|| money_microusd(on_demand.and_then(|value| value.get("remaining"))))
        .flatten()
        .or_else(|| {
            limit_microusd
                .zip(used_microusd)
                .map(|(limit, used)| (limit - used).max(0))
        });
    let request_based = cycle.get("limitType").and_then(Value::as_str) == Some("team");
    let request_scale = if request_based { 4.0 } else { 1.0 };
    let used_requests = request_based
        .then(|| number(plan.get("used")).map(|value| value / request_scale))
        .flatten();
    let limit_requests = request_based
        .then(|| number(plan.get("limit")).map(|value| value / request_scale))
        .flatten();
    let remaining_requests = request_based
        .then(|| number(plan.get("remaining")).map(|value| value / request_scale))
        .flatten()
        .or_else(|| {
            limit_requests
                .zip(used_requests)
                .map(|(limit, used)| (limit - used).max(0.0))
        });
    let events = event_payload
        .get("usageEventsDisplay")
        .and_then(Value::as_array)
        .ok_or(CursorError::UnsupportedSchema)?;
    let token_totals = events.iter().fold(
        (0_i64, 0_i64, 0_i64, 0_i64),
        |(cost, input, output, cached), event| {
            let usage = event.get("tokenUsage").unwrap_or(&Value::Null);
            let charged = if event.get("isChargeable").and_then(Value::as_bool) == Some(true) {
                money_microusd(event.get("chargedCents")).unwrap_or(0)
            } else {
                0
            };
            (
                cost + charged,
                input + integer(usage.get("inputTokens")).unwrap_or(0),
                output + integer(usage.get("outputTokens")).unwrap_or(0),
                cached
                    + integer(usage.get("cacheReadTokens")).unwrap_or(0)
                    + integer(usage.get("cacheWriteTokens")).unwrap_or(0),
            )
        },
    );
    let today_requests = events
        .iter()
        .filter_map(|event| number(event.get("requestsCosts")))
        .sum::<f64>();
    Ok(Snapshot {
        cycle_started_at: date_value(cycle.get("billingCycleStart")),
        cycle_ends_at: date_value(cycle.get("billingCycleEnd")),
        plan_kind: cycle
            .get("membershipType")
            .and_then(Value::as_str)
            .map(str::to_string),
        used_microusd,
        limit_microusd,
        remaining_microusd,
        used_requests,
        limit_requests,
        remaining_requests,
        today_requests: Some(today_requests),
        auto_percent_basis_points: number(plan.get("autoPercentUsed"))
            .map(|value| (value * 100.0).round() as i64),
        api_percent_basis_points: number(plan.get("apiPercentUsed"))
            .map(|value| (value * 100.0).round() as i64),
        today_microusd: on_demand_enabled.then_some(token_totals.0),
        total_tokens: Some(token_totals.1 + token_totals.2 + token_totals.3),
        input_tokens: Some(token_totals.1),
        output_tokens: Some(token_totals.2),
        cached_tokens: Some(token_totals.3),
    })
}

fn has_plan_usage(payload: &Value) -> bool {
    payload.get("planUsage").is_some() || payload.pointer("/individualUsage/plan").is_some()
}

async fn persist_snapshot(
    pool: &SqlitePool,
    account_key: &str,
    snapshot: Snapshot,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO cursor_usage_snapshots
         (id, account_key, observed_at, cycle_started_at, cycle_ends_at, plan_kind,
          used_microusd, limit_microusd, remaining_microusd,
          used_requests, limit_requests, remaining_requests, today_requests,
          auto_percent_basis_points, api_percent_basis_points,
          today_microusd, total_tokens, input_tokens, output_tokens, cached_tokens,
          source, confidence)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'cursor-dashboard', 'unofficial')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(account_key)
    .bind(Utc::now().to_rfc3339())
    .bind(snapshot.cycle_started_at)
    .bind(snapshot.cycle_ends_at)
    .bind(snapshot.plan_kind)
    .bind(snapshot.used_microusd)
    .bind(snapshot.limit_microusd)
    .bind(snapshot.remaining_microusd)
    .bind(snapshot.used_requests)
    .bind(snapshot.limit_requests)
    .bind(snapshot.remaining_requests)
    .bind(snapshot.today_requests)
    .bind(snapshot.auto_percent_basis_points)
    .bind(snapshot.api_percent_basis_points)
    .bind(snapshot.today_microusd)
    .bind(snapshot.total_tokens)
    .bind(snapshot.input_tokens)
    .bind(snapshot.output_tokens)
    .bind(snapshot.cached_tokens)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

async fn set_success(pool: &SqlitePool) -> Result<(), String> {
    let backoff = BACKOFF_UNTIL.get_or_init(|| Mutex::new(None));
    *backoff.lock().await = None;
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

async fn set_error(pool: &SqlitePool, code: &str) {
    let _ = sqlx::query(
        "INSERT INTO ai_adapter_state (adapter_id, last_error)
         VALUES (?, ?)
         ON CONFLICT(adapter_id) DO UPDATE SET last_error = excluded.last_error",
    )
    .bind(ADAPTER_ID)
    .bind(code)
    .execute(pool)
    .await;
}

fn account_key(id: &str) -> String {
    format!("{:x}", Sha256::digest(format!("cursor:{id}").as_bytes()))
}

fn mask_email(email: Option<&str>) -> String {
    let Some((local, domain)) = email.and_then(|value| value.split_once('@')) else {
        return "Cursor 계정".to_string();
    };
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}

fn value_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
}

fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn integer(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn money_microusd(value: Option<&Value>) -> Option<i64> {
    let raw = match value? {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        _ => return None,
    };
    let normalized = raw.trim().trim_start_matches('$').replace(',', "");
    if normalized.starts_with('-') {
        return Some(0);
    }
    let mut parts = normalized.split('.');
    let whole = parts.next()?.parse::<i128>().ok()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() || !fraction.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }
    let mut fraction_digits = fraction.chars();
    let mut fraction_micros = 0_i128;
    for _ in 0..4 {
        fraction_micros = fraction_micros * 10
            + fraction_digits
                .next()
                .and_then(|char| char.to_digit(10))
                .unwrap_or(0) as i128;
    }
    if fraction_digits
        .next()
        .and_then(|char| char.to_digit(10))
        .is_some_and(|digit| digit >= 5)
    {
        fraction_micros += 1;
    }
    let micros = whole.checked_mul(10_000)?.checked_add(fraction_micros)?;
    i64::try_from(micros).ok()
}

fn date_value(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        if let Ok(milliseconds) = text.parse::<i64>() {
            return DateTime::<Utc>::from_timestamp_millis(milliseconds)
                .map(|date| date.to_rfc3339());
        }
        if let Ok(date) = DateTime::parse_from_rfc3339(text) {
            return Some(date.with_timezone(&Utc).to_rfc3339());
        }
    }
    value
        .as_i64()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|date| date.to_rfc3339())
}

fn local_day_bounds_ms() -> Result<(i64, i64), CursorError> {
    let today = Local::now().date_naive();
    let start = Local
        .from_local_datetime(
            &today
                .and_hms_opt(0, 0, 0)
                .ok_or(CursorError::InvalidResponse)?,
        )
        .earliest()
        .ok_or(CursorError::InvalidResponse)?;
    let tomorrow = today.succ_opt().ok_or(CursorError::InvalidResponse)?;
    let end = Local
        .from_local_datetime(
            &tomorrow
                .and_hms_opt(0, 0, 0)
                .ok_or(CursorError::InvalidResponse)?,
        )
        .latest()
        .ok_or(CursorError::InvalidResponse)?;
    Ok((start.timestamp_millis(), end.timestamp_millis() - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_email() {
        assert_eq!(mask_email(Some("hello@example.com")), "h***@example.com");
        assert_eq!(mask_email(None), "Cursor 계정");
    }

    #[test]
    fn parses_usage_snapshot() {
        let cycle = json!({
            "billingCycleEnd": "2026-08-01T00:00:00Z",
            "limitType": "team",
            "individualUsage": {
                "plan": {
                    "used": 2000,
                    "limit": 2000,
                    "autoPercentUsed": 12.5,
                    "apiPercentUsed": 18.25
                },
                "onDemand": {
                    "enabled": true,
                    "used": 1234,
                    "limit": 7000
                }
            }
        });
        let events = json!({
            "usageEventsDisplay": [{
                "requestsCosts": 1,
                "isChargeable": true,
                "chargedCents": 25.5,
                "tokenUsage": {
                    "inputTokens": 100,
                    "outputTokens": 20,
                    "cacheReadTokens": 50,
                    "cacheWriteTokens": 10
                }
            }]
        });
        let snapshot = parse_snapshot(&cycle, &events).unwrap();
        assert_eq!(snapshot.used_microusd, Some(12_340_000));
        assert_eq!(snapshot.limit_microusd, Some(70_000_000));
        assert_eq!(snapshot.today_microusd, Some(255_000));
        assert_eq!(snapshot.used_requests, Some(500.0));
        assert_eq!(snapshot.limit_requests, Some(500.0));
        assert_eq!(snapshot.today_requests, Some(1.0));
        assert_eq!(snapshot.total_tokens, Some(180));
        assert_eq!(snapshot.auto_percent_basis_points, Some(1250));
    }

    #[test]
    fn accepts_summary_without_token_breakdown() {
        let cycle = json!({
            "billingCycleStart": "2026-07-01T00:00:00Z",
            "billingCycleEnd": "2026-08-01T00:00:00Z",
            "membershipType": "pro",
            "limitType": "team",
            "individualUsage": {
                "plan": {
                    "used": 1200,
                    "limit": 5000
                },
                "onDemand": { "enabled": false, "used": 0 }
            }
        });
        let snapshot = parse_snapshot(&cycle, &json!({ "usageEventsDisplay": [] })).unwrap();

        assert!(has_plan_usage(&cycle));
        assert_eq!(snapshot.plan_kind.as_deref(), Some("pro"));
        assert_eq!(snapshot.used_microusd, None);
        assert_eq!(snapshot.limit_microusd, None);
        assert_eq!(snapshot.used_requests, Some(300.0));
        assert_eq!(snapshot.limit_requests, Some(1250.0));
        assert_eq!(snapshot.today_microusd, None);
        assert_eq!(snapshot.total_tokens, Some(0));
    }

    #[test]
    fn treats_empty_usage_event_response_as_zero_events() {
        assert!(usage_events_page(&json!({})).unwrap().is_empty());
        assert!(matches!(
            usage_events_page(&json!({ "unexpected": [] })),
            Err(CursorError::UnsupportedSchema)
        ));
    }

    #[test]
    fn creates_cookie_from_full_value() {
        let token = SecretToken("user_123::jwt.value.here".to_string());
        assert_eq!(
            cookie_value(&token).unwrap(),
            "user_123%3A%3Ajwt.value.here"
        );
    }

    #[test]
    fn creates_cookie_from_session_jwt_without_exposing_connection_prefix() {
        let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"github|user_123","type":"session"}"#);
        let token = SecretToken(format!("header.{claims}.signature"));
        assert_eq!(
            cookie_value(&token).unwrap(),
            format!("user_123%3A%3Aheader.{claims}.signature")
        );
    }

    #[test]
    fn rejects_agent_api_tokens() {
        let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"github|user_123","type":"api_key_token"}"#);
        let token = SecretToken(format!("header.{claims}.signature"));
        assert!(matches!(
            cookie_value(&token),
            Err(CursorError::CredentialNotFound)
        ));
    }

    #[test]
    fn converts_fractional_cents_without_floating_point() {
        assert_eq!(money_microusd(Some(&json!(25.5))), Some(255_000));
        assert_eq!(money_microusd(Some(&json!("1.23456"))), Some(12_346));
        assert_eq!(money_microusd(Some(&json!(-2))), Some(0));
    }

    #[tokio::test]
    #[ignore = "requires an installed and authenticated Cursor app"]
    async fn fetches_live_cursor_snapshot() {
        let token = read_token().await.unwrap();
        let client = ClientHandle::new().unwrap();
        let account = client.account(&token).await.unwrap();
        client.snapshot(&token, &account).await.unwrap();
    }
}
