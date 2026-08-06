use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition};

use crate::database::AppState;

const VISIBLE_KEY: &str = "character.window.visible";
const X_KEY: &str = "character.window.x";
const Y_KEY: &str = "character.window.y";
const LAYOUT_KEY: &str = "character.window.layout";
const COMPACT_LAYOUT: &str = "compact-v2";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWindowState {
    pub visible: bool,
}

pub async fn restore(app: &AppHandle, pool: &SqlitePool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("character") else {
        return Err("character window is unavailable".to_string());
    };
    window
        .set_size(LogicalSize::new(48.0, 48.0))
        .map_err(|error| error.to_string())?;
    let compact_layout = setting(pool, LAYOUT_KEY).await?.as_deref() == Some(COMPACT_LAYOUT);
    let x = (if compact_layout {
        setting(pool, X_KEY).await?
    } else {
        None
    })
    .and_then(|value| value.parse::<i32>().ok());
    let y = (if compact_layout {
        setting(pool, Y_KEY).await?
    } else {
        None
    })
    .and_then(|value| value.parse::<i32>().ok());
    if let (Some(x), Some(y)) = (x, y) {
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())?;
    }
    if setting(pool, VISIBLE_KEY).await?.as_deref() == Some("true") {
        window.show().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: tauri::State<'_, AppState>) -> Result<CharacterWindowState, String> {
    Ok(CharacterWindowState {
        visible: setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true"),
    })
}

#[tauri::command]
pub async fn set_visible(
    visible: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<CharacterWindowState, String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    window
        .set_size(LogicalSize::new(48.0, 48.0))
        .map_err(|error| error.to_string())?;
    if visible {
        if setting(&state.pool, LAYOUT_KEY).await?.as_deref() != Some(COMPACT_LAYOUT)
            || setting(&state.pool, X_KEY).await?.is_none()
            || setting(&state.pool, Y_KEY).await?.is_none()
        {
            position_near_main(&app, &window);
            let position = window.outer_position().map_err(|error| error.to_string())?;
            save_setting(&state.pool, X_KEY, position.x.to_string()).await?;
            save_setting(&state.pool, Y_KEY, position.y.to_string()).await?;
            save_setting(&state.pool, LAYOUT_KEY, COMPACT_LAYOUT.to_string()).await?;
        }
        window.show()
    } else {
        window.hide()
    }
    .map_err(|error| error.to_string())?;
    save_setting(&state.pool, VISIBLE_KEY, visible.to_string()).await?;
    let next = CharacterWindowState { visible };
    let _ = app.emit("character-window-state-changed", &next);
    Ok(next)
}

fn position_near_main(app: &AppHandle, window: &tauri::WebviewWindow) {
    const GAP: i32 = 6;
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    let (Ok(main_position), Ok(main_size), Ok(character_size)) = (
        main.outer_position(),
        main.outer_size(),
        window.outer_size(),
    ) else {
        return;
    };

    let mut x = main_position
        .x
        .saturating_add(i32::try_from(main_size.width).unwrap_or(i32::MAX))
        .saturating_add(GAP);
    let mut y = main_position
        .y
        .saturating_add(i32::try_from(main_size.height).unwrap_or(i32::MAX))
        .saturating_sub(i32::try_from(character_size.height).unwrap_or(i32::MAX));

    if let Ok(Some(monitor)) = main.current_monitor() {
        let work = monitor.work_area();
        let min_x = work.position.x.saturating_add(GAP);
        let min_y = work.position.y.saturating_add(GAP);
        let max_x = work
            .position
            .x
            .saturating_add(i32::try_from(work.size.width).unwrap_or(i32::MAX))
            .saturating_sub(i32::try_from(character_size.width).unwrap_or(i32::MAX))
            .saturating_sub(GAP);
        let max_y = work
            .position
            .y
            .saturating_add(i32::try_from(work.size.height).unwrap_or(i32::MAX))
            .saturating_sub(i32::try_from(character_size.height).unwrap_or(i32::MAX))
            .saturating_sub(GAP);
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

#[tauri::command]
pub async fn save_position(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    save_setting(&state.pool, X_KEY, position.x.to_string()).await?;
    save_setting(&state.pool, Y_KEY, position.y.to_string()).await
}

pub async fn toggle(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let visible = setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true");
    set_visible(!visible, app.clone(), state).await?;
    Ok(())
}

async fn setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())
}

async fn save_setting(pool: &SqlitePool, key: &str, value: String) -> Result<(), String> {
    sqlx::query("INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key).bind(value).execute(pool).await.map_err(|error| error.to_string())?;
    Ok(())
}
