use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition,
};

use crate::database::AppState;
use crate::file_drop;

const VISIBLE_KEY: &str = "character.window.visible";
const X_KEY: &str = "character.window.x";
const Y_KEY: &str = "character.window.y";
const LAYOUT_KEY: &str = "character.window.layout";
const COMPACT_LAYOUT: &str = "compact-v2";
const FOLLOW_POINTER_KEY: &str = "character.window.follow_pointer";
const BASE_WINDOW_LOGICAL_SIZE: f64 = 48.0;
const FILE_DROP_WINDOW_LOGICAL_SIZE: f64 = 88.0;
const FILE_DROP_GROW_MS: u64 = 35;
const FILE_DROP_SHRINK_MS: u64 = 20;
const FILE_DROP_RESIZE_FRAME_MS: u64 = 14;

static FOLLOW_POINTER: AtomicBool = AtomicBool::new(false);
static CONTEXT_MENU_OPEN: AtomicBool = AtomicBool::new(false);
static FILE_DROP_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_pointer_following() -> bool {
    FOLLOW_POINTER.load(Ordering::Relaxed)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWindowState {
    pub visible: bool,
    pub follow_pointer: bool,
}

pub async fn restore(app: &AppHandle, pool: &SqlitePool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("character") else {
        return Err("character window is unavailable".to_string());
    };
    FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
    window
        .set_size(LogicalSize::new(
            BASE_WINDOW_LOGICAL_SIZE,
            BASE_WINDOW_LOGICAL_SIZE,
        ))
        .map_err(|error| error.to_string())?;
    FOLLOW_POINTER.store(
        setting(pool, FOLLOW_POINTER_KEY).await?.as_deref() == Some("true"),
        Ordering::Relaxed,
    );
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
        follow_pointer: FOLLOW_POINTER.load(Ordering::Relaxed),
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
        .set_size(LogicalSize::new(
            BASE_WINDOW_LOGICAL_SIZE,
            BASE_WINDOW_LOGICAL_SIZE,
        ))
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
        FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
        window.hide()
    }
    .map_err(|error| error.to_string())?;
    save_setting(&state.pool, VISIBLE_KEY, visible.to_string()).await?;
    let next = CharacterWindowState {
        visible,
        follow_pointer: FOLLOW_POINTER.load(Ordering::Relaxed),
    };
    let _ = app.emit("character-window-state-changed", &next);
    Ok(next)
}

pub fn start_pointer_follower(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_position = None;
        loop {
            if !FOLLOW_POINTER.load(Ordering::Relaxed)
                || CONTEXT_MENU_OPEN.load(Ordering::Relaxed)
                || FILE_DROP_ACTIVE.load(Ordering::Relaxed)
            {
                last_position = None;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            }
            let Some(window) = app.get_webview_window("character") else {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            };
            if !window.is_visible().unwrap_or(false) {
                last_position = None;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            }
            if let Some(position) = pointer_follow_position(&window) {
                if last_position != Some(position) {
                    let _ = window.set_position(position);
                    last_position = Some(position);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
        }
    });
}

#[tauri::command]
pub async fn begin_character_file_drop(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    FILE_DROP_ACTIVE.store(true, Ordering::Relaxed);
    if let Err(error) =
        animate_window_size(&window, FILE_DROP_WINDOW_LOGICAL_SIZE, FILE_DROP_GROW_MS).await
    {
        FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn end_character_file_drop(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("character").ok_or_else(|| {
        FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
        "character window is unavailable".to_string()
    })?;
    let result = animate_window_size(&window, BASE_WINDOW_LOGICAL_SIZE, FILE_DROP_SHRINK_MS).await;
    FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
    result
}

#[tauri::command]
pub fn trash_dropped_files(paths: Vec<String>) -> Result<u32, String> {
    if !FILE_DROP_ACTIVE.load(Ordering::Relaxed) {
        return Err("파일 드롭 상태가 활성화되어 있지 않습니다.".to_string());
    }
    file_drop::trash_paths(&paths)
}

#[tauri::command]
pub fn show_context_menu(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    let follow = CheckMenuItem::with_id(
        &app,
        "character-follow-pointer",
        "마우스 따라다니기",
        true,
        FOLLOW_POINTER.load(Ordering::Relaxed),
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let hide = MenuItem::with_id(
        &app,
        "character-context-hide",
        "캐릭터 숨기기",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(&app, &[&follow, &hide]).map_err(|error| error.to_string())?;
    CONTEXT_MENU_OPEN.store(true, Ordering::Relaxed);
    let result = window.popup_menu(&menu).map_err(|error| error.to_string());
    CONTEXT_MENU_OPEN.store(false, Ordering::Relaxed);
    result
}

pub async fn toggle_pointer_following(app: &AppHandle) -> Result<(), String> {
    let follow = !FOLLOW_POINTER.load(Ordering::Relaxed);
    FOLLOW_POINTER.store(follow, Ordering::Relaxed);
    crate::tray::set_pointer_following(follow);
    let state = app.state::<AppState>();
    save_setting(&state.pool, FOLLOW_POINTER_KEY, follow.to_string()).await?;
    if !follow {
        save_current_position(app, &state.pool).await?;
    }
    let visible = setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true");
    let _ = app.emit(
        "character-window-state-changed",
        CharacterWindowState {
            visible,
            follow_pointer: follow,
        },
    );
    Ok(())
}

#[cfg(windows)]
fn pointer_follow_position(_window: &tauri::WebviewWindow) -> Option<PhysicalPosition<i32>> {
    use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

    let mut pointer = POINT::default();
    if unsafe { GetCursorPos(&mut pointer) }.is_err() {
        return None;
    }
    Some(PhysicalPosition::new(pointer.x + 8, pointer.y + 12))
}

#[cfg(target_os = "macos")]
fn pointer_follow_position(_window: &tauri::WebviewWindow) -> Option<tauri::LogicalPosition<f64>> {
    use objc2_core_graphics::CGEvent;

    let event = CGEvent::new(None)?;
    let pointer = CGEvent::location(Some(&event));
    Some(tauri::LogicalPosition::new(
        pointer.x + 8.0,
        pointer.y + 12.0,
    ))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn pointer_follow_position(_window: &tauri::WebviewWindow) -> Option<PhysicalPosition<i32>> {
    None
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

fn resize_around_fixed_center(
    window: &tauri::WebviewWindow,
    logical_size: f64,
    center_twice: (i64, i64),
    scale_factor: f64,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

        let physical_size = (logical_size * scale_factor).round() as i64;
        let next_x = (center_twice.0 - physical_size).div_euclid(2);
        let next_y = (center_twice.1 - physical_size).div_euclid(2);
        let x = i32::try_from(next_x)
            .map_err(|_| "캐릭터 창 위치가 범위를 벗어났습니다.".to_string())?;
        let y = i32::try_from(next_y)
            .map_err(|_| "캐릭터 창 위치가 범위를 벗어났습니다.".to_string())?;
        let size = i32::try_from(physical_size)
            .map_err(|_| "캐릭터 창 크기가 범위를 벗어났습니다.".to_string())?;
        let hwnd = window.hwnd().map_err(|error| error.to_string())?;

        unsafe {
            SetWindowPos(hwnd, None, x, y, size, size, SWP_NOACTIVATE | SWP_NOZORDER)
                .map_err(|error| error.to_string())
        }
    }

    #[cfg(not(windows))]
    {
        window
            .set_size(LogicalSize::new(logical_size, logical_size))
            .map_err(|error| error.to_string())?;
        let next_size = window.outer_size().map_err(|error| error.to_string())?;
        let next_x = (center_twice.0 - i64::from(next_size.width)).div_euclid(2);
        let next_y = (center_twice.1 - i64::from(next_size.height)).div_euclid(2);
        let x = i32::try_from(next_x)
            .map_err(|_| "캐릭터 창 위치가 범위를 벗어났습니다.".to_string())?;
        let y = i32::try_from(next_y)
            .map_err(|_| "캐릭터 창 위치가 범위를 벗어났습니다.".to_string())?;
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())
    }
}

async fn animate_window_size(
    window: &tauri::WebviewWindow,
    target_logical_size: f64,
    duration_ms: u64,
) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let current_size = window.outer_size().map_err(|error| error.to_string())?;
    let center_twice = (
        i64::from(position.x) * 2 + i64::from(current_size.width),
        i64::from(position.y) * 2 + i64::from(current_size.height),
    );
    let start_logical_size = f64::from(current_size.width) / scale_factor;
    if (start_logical_size - target_logical_size).abs() < 0.5 {
        return resize_around_fixed_center(window, target_logical_size, center_twice, scale_factor);
    }

    let steps = (duration_ms / FILE_DROP_RESIZE_FRAME_MS).max(1);
    let frame_ms = (duration_ms / steps).max(1);
    let growing = target_logical_size > start_logical_size;

    for step in 1..=steps {
        let progress = step as f64 / steps as f64;
        let eased = if growing {
            1.0 - (1.0 - progress).powi(3)
        } else {
            progress * progress * (3.0 - 2.0 * progress)
        };
        let next_size = start_logical_size + (target_logical_size - start_logical_size) * eased;
        resize_around_fixed_center(window, next_size, center_twice, scale_factor)?;
        if step < steps {
            tokio::time::sleep(Duration::from_millis(frame_ms)).await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn save_position(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    save_current_position(&app, &state.pool).await
}

async fn save_current_position(app: &AppHandle, pool: &SqlitePool) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    save_setting(pool, X_KEY, position.x.to_string()).await?;
    save_setting(pool, Y_KEY, position.y.to_string()).await
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
