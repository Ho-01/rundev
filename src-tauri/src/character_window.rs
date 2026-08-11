use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition,
};

use crate::database::AppState;
use crate::file_drop;

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

const VISIBLE_KEY: &str = "character.window.visible";
const X_KEY: &str = "character.window.x";
const Y_KEY: &str = "character.window.y";
const LAYOUT_KEY: &str = "character.window.layout";
const COMPACT_LAYOUT: &str = "compact-v2";
const FOLLOW_POINTER_KEY: &str = "character.window.follow_pointer";
const ROAMING_KEY: &str = "character.window.roaming";
const SIZE_KEY: &str = "character.window.size";
const MOTION_EVENT: &str = "character-window-motion-changed";
const DRAG_END_EVENT: &str = "character-window-drag-ended";
const ROAM_IDLE_MIN_MS: u64 = 1_200;
const ROAM_IDLE_MAX_MS: u64 = 7_800;
const ROAM_SPEED_MIN: f64 = 140.0;
const ROAM_SPEED_MAX: f64 = 360.0;
const ROAM_AREA_INSET_RATIO: f64 = 0.12;
const DEFAULT_WINDOW_LOGICAL_SIZE: f64 = 48.0;
const MIN_WINDOW_LOGICAL_SIZE: f64 = 36.0;
const MAX_WINDOW_LOGICAL_SIZE: f64 = 128.0;
const FILE_DROP_MAX_LOGICAL_SIZE: f64 = 160.0;
const FILE_DROP_SIZE_SCALE: f64 = 88.0 / DEFAULT_WINDOW_LOGICAL_SIZE;
const FILE_DROP_GROW_MS: u64 = 15;
const FILE_DROP_SHRINK_MS: u64 = 10;
const FILE_DROP_RESIZE_FRAME_MS: u64 = 14;

static FOLLOW_POINTER: AtomicBool = AtomicBool::new(false);
static ROAMING: AtomicBool = AtomicBool::new(false);
static MOVING: AtomicBool = AtomicBool::new(false);
static MOTION_DIRECTION: AtomicI8 = AtomicI8::new(1);
static CONTEXT_MENU_OPEN: AtomicBool = AtomicBool::new(false);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static DRAG_SESSION: AtomicU64 = AtomicU64::new(0);
static RESIZING: AtomicBool = AtomicBool::new(false);
static WINDOW_LOGICAL_SIZE_BITS: AtomicU64 = AtomicU64::new(DEFAULT_WINDOW_LOGICAL_SIZE.to_bits());
static FILE_DROP_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceButtonState(state_id: u32, button: u32) -> bool;
}

#[cfg(windows)]
fn primary_mouse_button_pressed() -> bool {
    unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON.0)) < 0 }
}

#[cfg(target_os = "macos")]
fn primary_mouse_button_pressed() -> bool {
    unsafe { CGEventSourceButtonState(0, 0) }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn primary_mouse_button_pressed() -> bool {
    true
}

fn clamp_window_logical_size(size: f64) -> f64 {
    if size.is_finite() {
        size.clamp(MIN_WINDOW_LOGICAL_SIZE, MAX_WINDOW_LOGICAL_SIZE)
    } else {
        DEFAULT_WINDOW_LOGICAL_SIZE
    }
}

fn window_logical_size() -> f64 {
    clamp_window_logical_size(f64::from_bits(
        WINDOW_LOGICAL_SIZE_BITS.load(Ordering::Relaxed),
    ))
}

fn set_window_logical_size(size: f64) -> f64 {
    let size = clamp_window_logical_size(size);
    WINDOW_LOGICAL_SIZE_BITS.store(size.to_bits(), Ordering::Relaxed);
    size
}

fn file_drop_logical_size_for(base_size: f64) -> f64 {
    (clamp_window_logical_size(base_size) * FILE_DROP_SIZE_SCALE)
        .min(FILE_DROP_MAX_LOGICAL_SIZE)
        .max(clamp_window_logical_size(base_size))
}

fn file_drop_logical_size() -> f64 {
    file_drop_logical_size_for(window_logical_size())
}

fn resize_character_window_centered(
    window: &tauri::WebviewWindow,
    logical_size: f64,
) -> Result<(), String> {
    let previous_position = window.outer_position().map_err(|error| error.to_string())?;
    let previous_size = window.outer_size().map_err(|error| error.to_string())?;
    window
        .set_size(LogicalSize::new(logical_size, logical_size))
        .map_err(|error| error.to_string())?;
    let resized = window.outer_size().map_err(|error| error.to_string())?;
    let width_delta = i32::try_from(resized.width)
        .unwrap_or(i32::MAX)
        .saturating_sub(i32::try_from(previous_size.width).unwrap_or(i32::MAX));
    let height_delta = i32::try_from(resized.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(i32::try_from(previous_size.height).unwrap_or(i32::MAX));
    window
        .set_position(PhysicalPosition::new(
            previous_position.x.saturating_sub(width_delta / 2),
            previous_position.y.saturating_sub(height_delta / 2),
        ))
        .map_err(|error| error.to_string())
}

fn resize_character_window_from_bottom_left(
    window: &tauri::WebviewWindow,
    logical_size: f64,
) -> Result<(), String> {
    let previous_position = window.outer_position().map_err(|error| error.to_string())?;
    let previous_size = window.outer_size().map_err(|error| error.to_string())?;
    window
        .set_size(LogicalSize::new(logical_size, logical_size))
        .map_err(|error| error.to_string())?;
    let resized = window.outer_size().map_err(|error| error.to_string())?;
    let previous_width = i32::try_from(previous_size.width).unwrap_or(i32::MAX);
    let resized_width = i32::try_from(resized.width).unwrap_or(i32::MAX);
    window
        .set_position(PhysicalPosition::new(
            previous_position
                .x
                .saturating_add(previous_width.saturating_sub(resized_width)),
            previous_position.y,
        ))
        .map_err(|error| error.to_string())
}

pub fn is_pointer_following() -> bool {
    FOLLOW_POINTER.load(Ordering::Relaxed)
}

pub fn is_roaming() -> bool {
    ROAMING.load(Ordering::Relaxed)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWindowState {
    pub visible: bool,
    pub follow_pointer: bool,
    pub roaming: bool,
    pub moving: bool,
    pub direction: i8,
    pub size: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterMotionState {
    pub moving: bool,
    pub direction: i8,
}

pub async fn restore(app: &AppHandle, pool: &SqlitePool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("character") else {
        return Err("character window is unavailable".to_string());
    };
    FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
    DRAGGING.store(false, Ordering::Relaxed);
    RESIZING.store(false, Ordering::Relaxed);
    let size = setting(pool, SIZE_KEY)
        .await?
        .and_then(|value| value.parse::<f64>().ok())
        .map(set_window_logical_size)
        .unwrap_or_else(|| set_window_logical_size(DEFAULT_WINDOW_LOGICAL_SIZE));
    window
        .set_size(LogicalSize::new(size, size))
        .map_err(|error| error.to_string())?;
    FOLLOW_POINTER.store(
        setting(pool, FOLLOW_POINTER_KEY).await?.as_deref() == Some("true"),
        Ordering::Relaxed,
    );
    ROAMING.store(
        setting(pool, ROAMING_KEY).await?.as_deref() == Some("true")
            && !FOLLOW_POINTER.load(Ordering::Relaxed),
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
        roaming: ROAMING.load(Ordering::Relaxed),
        moving: MOVING.load(Ordering::Relaxed),
        direction: MOTION_DIRECTION.load(Ordering::Relaxed),
        size: window_logical_size(),
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
    let size = window_logical_size();
    window
        .set_size(LogicalSize::new(size, size))
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
    if !visible {
        DRAGGING.store(false, Ordering::Relaxed);
        RESIZING.store(false, Ordering::Relaxed);
        FILE_DROP_ACTIVE.store(false, Ordering::Relaxed);
        emit_motion(&app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
    }
    let next = CharacterWindowState {
        visible,
        follow_pointer: FOLLOW_POINTER.load(Ordering::Relaxed),
        roaming: ROAMING.load(Ordering::Relaxed),
        moving: MOVING.load(Ordering::Relaxed),
        direction: MOTION_DIRECTION.load(Ordering::Relaxed),
        size: window_logical_size(),
    };
    let _ = app.emit("character-window-state-changed", &next);
    Ok(next)
}

pub fn start_pointer_follower(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_position = None;
        let mut roam_segment: Option<RoamSegment> = None;
        let mut next_roam_at = Instant::now();
        let mut was_dragging = false;
        let mut random_state = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0)
            ^ 0x9e37_79b9;
        loop {
            let Some(window) = app.get_webview_window("character") else {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            };
            if !window.is_visible().unwrap_or(false) {
                if roam_segment.take().is_some() {
                    emit_motion(&app, false, 1);
                }
                last_position = None;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                continue;
            }

            if DRAGGING.load(Ordering::Relaxed)
                || RESIZING.load(Ordering::Relaxed)
                || FILE_DROP_ACTIVE.load(Ordering::Relaxed)
            {
                if roam_segment.take().is_some() {
                    emit_motion(&app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
                }
                last_position = None;
                was_dragging = true;
                tokio::time::sleep(Duration::from_millis(33)).await;
                continue;
            }
            if was_dragging {
                was_dragging = false;
                next_roam_at = Instant::now() + next_roam_delay(&mut random_state);
            }

            if FOLLOW_POINTER.load(Ordering::Relaxed) {
                if roam_segment.take().is_some() {
                    emit_motion(&app, false, 1);
                }
                if CONTEXT_MENU_OPEN.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                if let Some(position) = pointer_follow_position(&window) {
                    if last_position != Some(position) {
                        let _ = window.set_position(position);
                        last_position = Some(position);
                    }
                }
                tokio::time::sleep(Duration::from_millis(33)).await;
                continue;
            }

            if ROAMING.load(Ordering::Relaxed) {
                if CONTEXT_MENU_OPEN.load(Ordering::Relaxed) {
                    if let Some(segment) = roam_segment.as_mut() {
                        segment.started += Duration::from_millis(100);
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                let now = Instant::now();
                if let Some(segment) = roam_segment.as_ref() {
                    let progress = (now.duration_since(segment.started).as_secs_f64()
                        / segment.duration.as_secs_f64())
                    .clamp(0.0, 1.0);
                    let eased = progress * progress * (3.0 - 2.0 * progress);
                    let x = segment.from.x as f64 + (segment.to.x - segment.from.x) as f64 * eased;
                    let y = segment.from.y as f64 + (segment.to.y - segment.from.y) as f64 * eased
                        - (std::f64::consts::PI * eased).sin() * segment.arc_height;
                    let _ = window
                        .set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
                    if progress >= 1.0 {
                        let direction = segment.direction;
                        roam_segment = None;
                        emit_motion(&app, false, direction);
                        next_roam_at = now + next_roam_delay(&mut random_state);
                    }
                    tokio::time::sleep(Duration::from_millis(33)).await;
                    continue;
                }

                if now >= next_roam_at {
                    if let Some(next) = next_roam_segment(&window, &mut random_state) {
                        let direction = if next.to.x >= next.from.x { 1 } else { -1 };
                        emit_motion(&app, true, direction);
                        roam_segment = Some(RoamSegment { direction, ..next });
                        continue;
                    }
                }
                tokio::time::sleep(Duration::from_millis(120)).await;
                continue;
            }

            if roam_segment.take().is_some() {
                emit_motion(&app, false, 1);
            }
            last_position = None;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
}

struct RoamSegment {
    from: PhysicalPosition<i32>,
    to: PhysicalPosition<i32>,
    started: Instant,
    duration: Duration,
    arc_height: f64,
    direction: i8,
}

fn emit_motion(app: &AppHandle, moving: bool, direction: i8) {
    MOVING.store(moving, Ordering::Relaxed);
    MOTION_DIRECTION.store(direction, Ordering::Relaxed);
    let _ = app.emit(MOTION_EVENT, CharacterMotionState { moving, direction });
}

fn next_random(state: &mut u64, max: u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    if max == 0 {
        0
    } else {
        (*state >> 32) % max
    }
}

fn next_roam_delay(random_state: &mut u64) -> Duration {
    let span = ROAM_IDLE_MAX_MS - ROAM_IDLE_MIN_MS + 1;
    Duration::from_millis(ROAM_IDLE_MIN_MS + next_random(random_state, span))
}

fn next_roam_speed(random_state: &mut u64) -> f64 {
    let span = (ROAM_SPEED_MAX - ROAM_SPEED_MIN) as u64 + 1;
    ROAM_SPEED_MIN + next_random(random_state, span) as f64
}

fn next_roam_segment(window: &tauri::WebviewWindow, random_state: &mut u64) -> Option<RoamSegment> {
    let monitor = window.current_monitor().ok().flatten()?;
    let work = monitor.work_area();
    let size = window.outer_size().ok()?;
    let margin = 12_i32;
    let width = i32::try_from(size.width).ok()?;
    let height = i32::try_from(size.height).ok()?;
    let work_width = i32::try_from(work.size.width).ok()?;
    let work_height = i32::try_from(work.size.height).ok()?;
    let inset_x = (f64::from(work_width) * ROAM_AREA_INSET_RATIO).round() as i32;
    let inset_y = (f64::from(work_height) * ROAM_AREA_INSET_RATIO).round() as i32;
    let min_x = work
        .position
        .x
        .saturating_add(margin)
        .saturating_add(inset_x);
    let min_y = work
        .position
        .y
        .saturating_add(margin)
        .saturating_add(inset_y);
    let max_x = work
        .position
        .x
        .saturating_add(work_width)
        .saturating_sub(width)
        .saturating_sub(margin)
        .saturating_sub(inset_x);
    let max_y = work
        .position
        .y
        .saturating_add(work_height)
        .saturating_sub(height)
        .saturating_sub(margin)
        .saturating_sub(inset_y);
    if max_x < min_x || max_y < min_y {
        return None;
    }

    let current = window.outer_position().ok()?;
    let from = current;
    let bounded_from =
        PhysicalPosition::new(current.x.clamp(min_x, max_x), current.y.clamp(min_y, max_y));
    let to = PhysicalPosition::new(
        min_x + next_random(random_state, (max_x - min_x + 1) as u64) as i32,
        min_y + next_random(random_state, (max_y - min_y + 1) as u64) as i32,
    );
    let distance = (((to.x - from.x) as f64).powi(2) + ((to.y - from.y) as f64).powi(2)).sqrt();
    let speed = next_roam_speed(random_state);
    let duration = Duration::from_secs_f64((distance / speed).clamp(0.9, 8.0));
    let requested_arc = (distance / 12.0).clamp(18.0, 60.0);
    let top_clearance = f64::from(bounded_from.y.min(to.y) - min_y);
    let bottom_clearance = f64::from(max_y - bounded_from.y.max(to.y));
    let arc_height = if top_clearance >= requested_arc {
        requested_arc
    } else if bottom_clearance >= requested_arc {
        -requested_arc
    } else if top_clearance >= bottom_clearance {
        top_clearance.max(0.0)
    } else {
        -bottom_clearance.max(0.0)
    };
    Some(RoamSegment {
        from,
        to,
        started: Instant::now(),
        duration,
        arc_height,
        direction: 1,
    })
}

#[tauri::command]
pub fn begin_character_drag(scale: f64, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    let scale = if scale.is_finite() {
        scale.clamp(1.0, 2.0)
    } else {
        1.0
    };
    resize_character_window_centered(&window, window_logical_size() * scale)?;
    let session = DRAG_SESSION.fetch_add(1, Ordering::Relaxed) + 1;
    DRAGGING.store(true, Ordering::Relaxed);
    emit_motion(&app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
    tauri::async_runtime::spawn(async move {
        loop {
            if !DRAGGING.load(Ordering::Relaxed) || DRAG_SESSION.load(Ordering::Relaxed) != session
            {
                return;
            }
            if !primary_mouse_button_pressed() {
                DRAGGING.store(false, Ordering::Relaxed);
                let _ = app.emit(DRAG_END_EVENT, ());
                return;
            }
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn end_character_drag(app: AppHandle) -> Result<(), String> {
    DRAGGING.store(false, Ordering::Relaxed);
    DRAG_SESSION.fetch_add(1, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window("character") {
        resize_character_window_centered(&window, window_logical_size())?;
    }
    Ok(())
}

#[tauri::command]
pub fn resize_character_window(size: f64, app: AppHandle) -> Result<f64, String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    let size = set_window_logical_size(size);
    RESIZING.store(true, Ordering::Relaxed);
    emit_motion(&app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
    resize_character_window_from_bottom_left(&window, size)?;
    Ok(size)
}

#[tauri::command]
pub async fn finish_character_resize(
    size: f64,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<f64, String> {
    let size = set_window_logical_size(size);
    RESIZING.store(false, Ordering::Relaxed);
    save_setting(&state.pool, SIZE_KEY, size.to_string()).await?;
    let visible = setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true");
    let _ = app.emit(
        "character-window-state-changed",
        CharacterWindowState {
            visible,
            follow_pointer: FOLLOW_POINTER.load(Ordering::Relaxed),
            roaming: ROAMING.load(Ordering::Relaxed),
            moving: MOVING.load(Ordering::Relaxed),
            direction: MOTION_DIRECTION.load(Ordering::Relaxed),
            size,
        },
    );
    Ok(size)
}

#[tauri::command]
pub async fn begin_character_file_drop(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("character")
        .ok_or_else(|| "character window is unavailable".to_string())?;
    FILE_DROP_ACTIVE.store(true, Ordering::Relaxed);
    emit_motion(&app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
    if let Err(error) =
        animate_window_size(&window, file_drop_logical_size(), FILE_DROP_GROW_MS).await
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
    let result = animate_window_size(&window, window_logical_size(), FILE_DROP_SHRINK_MS).await;
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
    let roaming = CheckMenuItem::with_id(
        &app,
        "character-roam-monitor",
        "모니터 자유롭게 돌아다니기",
        !FOLLOW_POINTER.load(Ordering::Relaxed),
        ROAMING.load(Ordering::Relaxed),
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
    let menu =
        Menu::with_items(&app, &[&follow, &roaming, &hide]).map_err(|error| error.to_string())?;
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
    if follow {
        DRAGGING.store(false, Ordering::Relaxed);
        RESIZING.store(false, Ordering::Relaxed);
        ROAMING.store(false, Ordering::Relaxed);
        crate::tray::set_roaming(false);
        save_setting(&state.pool, ROAMING_KEY, "false".to_string()).await?;
        emit_motion(app, false, MOTION_DIRECTION.load(Ordering::Relaxed));
    }
    if !follow {
        save_current_position(app, &state.pool).await?;
    }
    let visible = setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true");
    let _ = app.emit(
        "character-window-state-changed",
        CharacterWindowState {
            visible,
            follow_pointer: follow,
            roaming: ROAMING.load(Ordering::Relaxed),
            moving: MOVING.load(Ordering::Relaxed),
            direction: MOTION_DIRECTION.load(Ordering::Relaxed),
            size: window_logical_size(),
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn toggle_roaming(app: AppHandle) -> Result<(), String> {
    if FOLLOW_POINTER.load(Ordering::Relaxed) {
        return Err("포인터 따라다니기 모드에서는 모니터 자유 이동을 사용할 수 없습니다".into());
    }
    let roaming = !ROAMING.load(Ordering::Relaxed);
    DRAGGING.store(false, Ordering::Relaxed);
    RESIZING.store(false, Ordering::Relaxed);
    ROAMING.store(roaming, Ordering::Relaxed);
    crate::tray::set_roaming(roaming);
    let state = app.state::<AppState>();
    save_setting(&state.pool, ROAMING_KEY, roaming.to_string()).await?;
    if !roaming {
        save_current_position(&app, &state.pool).await?;
        emit_motion(&app, false, 1);
    }
    let visible = setting(&state.pool, VISIBLE_KEY).await?.as_deref() == Some("true");
    let _ = app.emit(
        "character-window-state-changed",
        CharacterWindowState {
            visible,
            follow_pointer: false,
            roaming,
            moving: MOVING.load(Ordering::Relaxed),
            direction: MOTION_DIRECTION.load(Ordering::Relaxed),
            size: window_logical_size(),
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

#[cfg(test)]
mod tests {
    use super::{clamp_window_logical_size, file_drop_logical_size_for};

    #[test]
    fn clamps_user_selected_character_size() {
        assert_eq!(clamp_window_logical_size(12.0), 36.0);
        assert_eq!(clamp_window_logical_size(72.0), 72.0);
        assert_eq!(clamp_window_logical_size(250.0), 128.0);
    }

    #[test]
    fn file_drop_growth_respects_the_resized_character_limit() {
        assert_eq!(file_drop_logical_size_for(48.0), 88.0);
        assert_eq!(file_drop_logical_size_for(128.0), 160.0);
    }
}
