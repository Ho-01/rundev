use crate::database::AppState;
use chrono::Local;
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
