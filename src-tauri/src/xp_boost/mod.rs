use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Local, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

const COUPON_PREFIX: &str = "RDC1";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CouponPayload {
    coupon_id: String,
    multiplier: i64,
    duration_minutes: i64,
    redeem_before: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CouponPreview {
    pub coupon_id: String,
    pub multiplier: i64,
    pub duration_minutes: i64,
    pub redeem_before: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XpBoostStatus {
    pub active: bool,
    pub multiplier: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

pub fn preview(code: &str) -> Result<CouponPreview, String> {
    let payload = verify(code, configured_public_key()?)?;
    validate_payload(&payload, Utc::now())?;
    Ok(to_preview(payload))
}

pub async fn redeem(pool: &SqlitePool, code: &str) -> Result<XpBoostStatus, String> {
    let now = Utc::now();
    let payload = verify(code, configured_public_key()?)?;
    validate_payload(&payload, now)?;
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    let already_used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM xp_coupon_redemptions WHERE coupon_id = ?)",
    )
    .bind(&payload.coupon_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    if already_used {
        return Err("이미 사용한 쿠폰입니다.".to_string());
    }

    let latest_end: Option<String> =
        sqlx::query_scalar("SELECT MAX(ends_at) FROM xp_coupon_redemptions WHERE ends_at > ?")
            .bind(now.to_rfc3339())
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
    let starts_at = latest_end
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .filter(|value| *value > now)
        .unwrap_or(now);
    let ends_at = starts_at + Duration::minutes(payload.duration_minutes);
    sqlx::query(
        "INSERT INTO xp_coupon_redemptions
         (coupon_id, multiplier, redeemed_at, starts_at, ends_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&payload.coupon_id)
    .bind(payload.multiplier)
    .bind(now.to_rfc3339())
    .bind(starts_at.to_rfc3339())
    .bind(ends_at.to_rfc3339())
    .execute(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    status(pool).await
}

pub async fn status(pool: &SqlitePool) -> Result<XpBoostStatus, String> {
    let now = Utc::now().to_rfc3339();
    let row: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT multiplier, starts_at, ends_at
         FROM xp_coupon_redemptions
         WHERE starts_at <= ? AND ends_at > ?
         ORDER BY starts_at ASC LIMIT 1",
    )
    .bind(&now)
    .bind(&now)
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(match row {
        Some((multiplier, starts_at, ends_at)) => XpBoostStatus {
            active: true,
            multiplier: Some(multiplier),
            starts_at: Some(starts_at),
            ends_at: Some(ends_at),
        },
        None => XpBoostStatus {
            active: false,
            multiplier: None,
            starts_at: None,
            ends_at: None,
        },
    })
}

pub async fn award_xp(
    transaction: &mut Transaction<'_, Sqlite>,
    event_type: &str,
    amount: i64,
    source_event_id: &str,
    occurred_at: &str,
) -> Result<i64, sqlx::Error> {
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO xp_events
         (id, occurred_at, event_type, amount, source_event_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(occurred_at)
    .bind(event_type)
    .bind(amount)
    .bind(source_event_id)
    .execute(&mut **transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        return Ok(0);
    }

    let multiplier: i64 = sqlx::query_scalar(
        "SELECT COALESCE((
            SELECT multiplier FROM xp_coupon_redemptions
            WHERE starts_at <= ? AND ends_at > ?
            ORDER BY starts_at ASC LIMIT 1
         ), 1)",
    )
    .bind(occurred_at)
    .bind(occurred_at)
    .fetch_one(&mut **transaction)
    .await?;
    let bonus = amount * (multiplier - 1);
    if bonus > 0 {
        sqlx::query(
            "INSERT INTO xp_events
             (id, occurred_at, event_type, amount, source_event_id)
             VALUES (?, ?, 'xp_boost_bonus', ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(occurred_at)
        .bind(bonus)
        .bind(format!("boost:{source_event_id}"))
        .execute(&mut **transaction)
        .await?;
    }
    let trait_id = match event_type {
        "focus_milestone" => Some("focus-ready"),
        "keyboard_milestone" => Some("hot-keyboard"),
        "ai_usage_milestone" => Some("context-runner"),
        _ => None,
    };
    let mut trait_bonus = 0;
    if let Some(trait_id) = trait_id {
        let level = crate::progression::trait_level_in(transaction, trait_id).await?;
        if level > 0 {
            let previous: i64 = sqlx::query_scalar(
                "SELECT COALESCE((SELECT basis_point_xp FROM trait_bonus_accumulators WHERE trait_id = ?), 0)",
            ).bind(trait_id).fetch_one(&mut **transaction).await?;
            let accumulated = previous + amount * level * 50;
            trait_bonus = accumulated / 10_000;
            sqlx::query(
                "INSERT INTO trait_bonus_accumulators (trait_id, basis_point_xp) VALUES (?, ?)
                 ON CONFLICT(trait_id) DO UPDATE SET basis_point_xp = excluded.basis_point_xp",
            )
            .bind(trait_id)
            .bind(accumulated % 10_000)
            .execute(&mut **transaction)
            .await?;
            if trait_bonus > 0 {
                sqlx::query(
                    "INSERT INTO xp_events (id, occurred_at, event_type, amount, source_event_id)
                     VALUES (?, ?, 'trait_bonus', ?, ?)",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(occurred_at)
                .bind(trait_bonus)
                .bind(format!("trait:{trait_id}:{source_event_id}"))
                .execute(&mut **transaction)
                .await?;
            }
        }
    }
    if event_type == "focus_milestone" {
        trait_bonus += award_reload_daily_bonus(transaction, occurred_at).await?;
    }
    let total = amount + bonus + trait_bonus;
    sqlx::query(
        "UPDATE character_state
         SET total_xp = total_xp + ?, level = ((total_xp + ?) / 100) + 1
         WHERE id = 1",
    )
    .bind(total)
    .bind(total)
    .execute(&mut **transaction)
    .await?;
    Ok(total)
}

async fn award_reload_daily_bonus(
    transaction: &mut Transaction<'_, Sqlite>,
    occurred_at: &str,
) -> Result<i64, sqlx::Error> {
    let level = crate::progression::trait_level_in(transaction, "reload").await?;
    if level == 0 {
        return Ok(0);
    }
    let Ok(parsed) = DateTime::parse_from_rfc3339(occurred_at) else {
        return Ok(0);
    };
    let local_date = parsed.with_timezone(&Local).date_naive().to_string();
    let basis_point_xp = level * 5_000;
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO trait_daily_rewards
         (trait_id, local_date, awarded_at, basis_point_xp, xp_awarded)
         VALUES ('reload', ?, ?, ?, 0)",
    )
    .bind(&local_date)
    .bind(occurred_at)
    .bind(basis_point_xp)
    .execute(&mut **transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        return Ok(0);
    }

    let previous: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT basis_point_xp FROM trait_bonus_accumulators
         WHERE trait_id = 'reload'), 0)",
    )
    .fetch_one(&mut **transaction)
    .await?;
    let accumulated = previous + basis_point_xp;
    let awarded_xp = accumulated / 10_000;
    sqlx::query(
        "INSERT INTO trait_bonus_accumulators (trait_id, basis_point_xp)
         VALUES ('reload', ?)
         ON CONFLICT(trait_id) DO UPDATE SET basis_point_xp = excluded.basis_point_xp",
    )
    .bind(accumulated % 10_000)
    .execute(&mut **transaction)
    .await?;
    if awarded_xp > 0 {
        sqlx::query(
            "INSERT INTO xp_events (id, occurred_at, event_type, amount, source_event_id)
             VALUES (?, ?, 'trait_bonus', ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(occurred_at)
        .bind(awarded_xp)
        .bind(format!("trait:reload:{local_date}"))
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE trait_daily_rewards SET xp_awarded = ?
             WHERE trait_id = 'reload' AND local_date = ?",
        )
        .bind(awarded_xp)
        .bind(&local_date)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(awarded_xp)
}

fn configured_public_key() -> Result<VerifyingKey, String> {
    let encoded = option_env!("RUNDEV_COUPON_PUBLIC_KEY")
        .ok_or("이 빌드에는 쿠폰 검증 키가 설정되지 않았습니다.")?;
    decode_public_key(encoded)
}

fn decode_public_key(encoded: &str) -> Result<VerifyingKey, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded.trim())
        .map_err(|_| "쿠폰 검증 키 형식이 잘못되었습니다.".to_string())?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "쿠폰 검증 키 길이가 잘못되었습니다.".to_string())?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| "쿠폰 검증 키가 올바르지 않습니다.".to_string())
}

fn verify(code: &str, key: VerifyingKey) -> Result<CouponPayload, String> {
    let parts: Vec<&str> = code.trim().split('.').collect();
    if parts.len() != 3 || parts[0] != COUPON_PREFIX {
        return Err("쿠폰 번호 형식이 올바르지 않습니다.".to_string());
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| "쿠폰 내용을 읽을 수 없습니다.".to_string())?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| "쿠폰 서명을 읽을 수 없습니다.".to_string())?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| "쿠폰 서명 형식이 올바르지 않습니다.".to_string())?;
    key.verify(parts[1].as_bytes(), &signature)
        .map_err(|_| "유효하지 않은 쿠폰입니다.".to_string())?;
    serde_json::from_slice(&payload_bytes).map_err(|_| "쿠폰 내용이 올바르지 않습니다.".to_string())
}

fn validate_payload(payload: &CouponPayload, now: DateTime<Utc>) -> Result<(), String> {
    if payload.coupon_id.trim().is_empty() || payload.coupon_id.len() > 100 {
        return Err("쿠폰 ID가 올바르지 않습니다.".to_string());
    }
    if !matches!(payload.multiplier, 2 | 3) {
        return Err("지원하지 않는 경험치 배수입니다.".to_string());
    }
    if !(1..=43_200).contains(&payload.duration_minutes) {
        return Err("쿠폰 적용 시간이 올바르지 않습니다.".to_string());
    }
    if payload.redeem_before < now {
        return Err("사용 기한이 지난 쿠폰입니다.".to_string());
    }
    Ok(())
}

fn to_preview(payload: CouponPayload) -> CouponPreview {
    CouponPreview {
        coupon_id: payload.coupon_id,
        multiplier: payload.multiplier,
        duration_minutes: payload.duration_minutes,
        redeem_before: payload.redeem_before.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_code(payload: serde_json::Value) -> (String, VerifyingKey) {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        let signature = key.sign(encoded.as_bytes());
        (
            format!(
                "RDC1.{encoded}.{}",
                URL_SAFE_NO_PAD.encode(signature.to_bytes())
            ),
            key.verifying_key(),
        )
    }

    #[test]
    fn verifies_signed_coupon_without_exposing_private_key() {
        let (code, public_key) = signed_code(serde_json::json!({
            "couponId": "launch-user-1", "multiplier": 3, "durationMinutes": 90,
            "redeemBefore": "2099-12-31T00:00:00Z"
        }));
        let payload = verify(&code, public_key).unwrap();
        assert_eq!(payload.coupon_id, "launch-user-1");
        assert_eq!(payload.multiplier, 3);
        assert_eq!(payload.duration_minutes, 90);
    }

    #[test]
    fn rejects_modified_coupon_payload() {
        let (code, public_key) = signed_code(serde_json::json!({
            "couponId": "launch-user-2", "multiplier": 2, "durationMinutes": 60,
            "redeemBefore": "2099-12-31T00:00:00Z"
        }));
        let tampered = code.replace("RDC1.", "RDC1.A");
        assert!(verify(&tampered, public_key).is_err());
    }

    #[tokio::test]
    async fn records_boost_bonus_as_a_separate_xp_event() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO xp_coupon_redemptions
             (coupon_id, multiplier, redeemed_at, starts_at, ends_at)
             VALUES ('test', 3, ?, ?, ?)",
        )
        .bind(now.to_rfc3339())
        .bind((now - Duration::minutes(1)).to_rfc3339())
        .bind((now + Duration::minutes(10)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
        let mut transaction = pool.begin().await.unwrap();
        let awarded = award_xp(
            &mut transaction,
            "test_event",
            10,
            "test:1",
            &now.to_rfc3339(),
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();

        let amounts: Vec<i64> = sqlx::query_scalar("SELECT amount FROM xp_events ORDER BY amount")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(awarded, 30);
        assert_eq!(amounts, vec![10, 20]);
    }

    #[tokio::test]
    async fn reload_rewards_only_the_first_focus_milestone_of_each_day() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("UPDATE character_traits SET level = 1 WHERE trait_id = 'reload'")
            .execute(&pool)
            .await
            .unwrap();

        let awards = [
            ("focus:2026-08-04:1", "2026-08-04T12:00:00Z"),
            ("focus:2026-08-04:2", "2026-08-04T12:20:00Z"),
            ("focus:2026-08-05:1", "2026-08-05T12:00:00Z"),
        ];
        for (source, occurred_at) in awards {
            let mut transaction = pool.begin().await.unwrap();
            award_xp(&mut transaction, "focus_milestone", 10, source, occurred_at)
                .await
                .unwrap();
            transaction.commit().await.unwrap();
        }

        let reload_claims: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trait_daily_rewards")
            .fetch_one(&pool)
            .await
            .unwrap();
        let trait_xp: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_events WHERE event_type = 'trait_bonus'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(reload_claims, 2);
        assert_eq!(trait_xp, 1);
    }

    #[tokio::test]
    async fn context_runner_adds_ai_xp_beyond_the_base_award() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("UPDATE character_traits SET level = 20 WHERE trait_id = 'context-runner'")
            .execute(&pool)
            .await
            .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        let awarded = award_xp(
            &mut transaction,
            "ai_usage_milestone",
            10,
            "ai-xp:2026-08-10:codex:1",
            "2026-08-14T12:00:00Z",
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();

        let trait_xp: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_events WHERE event_type = 'trait_bonus'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(awarded, 11);
        assert_eq!(trait_xp, 1);
    }
}
