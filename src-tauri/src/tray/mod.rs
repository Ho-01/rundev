use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex, OnceLock,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Manager, PhysicalPosition, PhysicalSize, Rect,
};

use crate::{adapters, character_window, database::AppState};

static SELECTED_RUNNER: AtomicU8 = AtomicU8::new(0);
static COMPACT_MAIN_X: OnceLock<Mutex<Option<i32>>> = OnceLock::new();

const CAT_FRAMES: [&[u8]; 4] = [
    include_bytes!("../../icons/tray/coding/01.png").as_slice(),
    include_bytes!("../../icons/tray/coding/02.png").as_slice(),
    include_bytes!("../../icons/tray/coding/03.png").as_slice(),
    include_bytes!("../../icons/tray/coding/04.png").as_slice(),
];
const FISH_FRAMES: [&[u8]; 4] = [
    include_bytes!("../../icons/tray/coding-fish/01.png").as_slice(),
    include_bytes!("../../icons/tray/coding-fish/02.png").as_slice(),
    include_bytes!("../../icons/tray/coding-fish/03.png").as_slice(),
    include_bytes!("../../icons/tray/coding-fish/04.png").as_slice(),
];
const ORANGE_CAT_FRAMES: [&[u8]; 4] = [
    include_bytes!("../../icons/tray/coding-orange-cat/01.png").as_slice(),
    include_bytes!("../../icons/tray/coding-orange-cat/02.png").as_slice(),
    include_bytes!("../../icons/tray/coding-orange-cat/03.png").as_slice(),
    include_bytes!("../../icons/tray/coding-orange-cat/04.png").as_slice(),
];
const SHRIMP_FRAMES: [&[u8]; 4] = [
    include_bytes!("../../icons/tray/coding-shrimp/01.png").as_slice(),
    include_bytes!("../../icons/tray/coding-shrimp/02.png").as_slice(),
    include_bytes!("../../icons/tray/coding-shrimp/03.png").as_slice(),
    include_bytes!("../../icons/tray/coding-shrimp/04.png").as_slice(),
];
const VTUBER_FRAMES: [&[u8]; 4] = [
    include_bytes!("../../icons/tray/coding-vtuber/01.png").as_slice(),
    include_bytes!("../../icons/tray/coding-vtuber/02.png").as_slice(),
    include_bytes!("../../icons/tray/coding-vtuber/03.png").as_slice(),
    include_bytes!("../../icons/tray/coding-vtuber/04.png").as_slice(),
];

pub fn set_runner(runner: &str) {
    let runner_index = match runner {
        "coding-fish" => 1,
        "coding-orange-cat" => 2,
        "coding-shrimp" | "coding-white-cat" => 3,
        "coding-vtuber" => 4,
        _ => 0,
    };
    SELECTED_RUNNER.store(runner_index, Ordering::Relaxed);
}

fn selected_frames() -> &'static [&'static [u8]; 4] {
    match SELECTED_RUNNER.load(Ordering::Relaxed) {
        1 => &FISH_FRAMES,
        2 => &ORANGE_CAT_FRAMES,
        3 => &SHRIMP_FRAMES,
        4 => &VTUBER_FRAMES,
        _ => &CAT_FRAMES,
    }
}

pub fn create(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "RunDev 열기", true, None::<&str>)?;
    let character =
        MenuItem::with_id(app, "character", "캐릭터 보이기/숨기기", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &character, &quit])?;
    let icon = tauri::image::Image::from_bytes(selected_frames()[0])?;

    TrayIconBuilder::with_id("rundev-tray")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("RunDev")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "character" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = character_window::toggle(&app).await {
                        tracing::warn!(%error, "Character window toggle failed");
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    start_animation(app.handle().clone());
    Ok(())
}

fn start_animation(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut frame_index = 0;
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(170));
        loop {
            ticker.tick().await;
            let frames = selected_frames();
            frame_index = (frame_index + 1) % frames.len();
            if let (Some(tray), Ok(icon)) = (
                app.tray_by_id("rundev-tray"),
                tauri::image::Image::from_bytes(frames[frame_index]),
            ) {
                let _ = tray.set_icon(Some(icon));
            }
        }
    });
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        refresh_usage_on_open(app);
    }
}

pub(crate) fn set_system_panel_expanded(
    app: &tauri::AppHandle,
    expanded: bool,
    expansion_side: Option<&str>,
    previous_expansion_side: Option<&str>,
) -> Result<(), String> {
    const COMPACT_WIDTH: f64 = 340.0;
    const EXPANDED_WIDTH: f64 = 512.0;
    const HEIGHT: f64 = 480.0;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let current_size = window.outer_size().map_err(|error| error.to_string())?;
    let current_position = window.outer_position().map_err(|error| error.to_string())?;
    let target_width = ((if expanded {
        EXPANDED_WIDTH
    } else {
        COMPACT_WIDTH
    }) * scale)
        .round() as u32;
    let target_height = (HEIGHT * scale).round() as u32;
    let compact_width = (COMPACT_WIDTH * scale).round() as u32;
    let current_left_inset = if previous_expansion_side == Some("left") {
        current_size.width.saturating_sub(compact_width)
    } else {
        0
    };
    let target_left_inset = if expanded && expansion_side == Some("left") {
        target_width.saturating_sub(compact_width)
    } else {
        0
    };
    let inferred_main_x = current_position
        .x
        .saturating_add(i32::try_from(current_left_inset).unwrap_or(i32::MAX));
    let anchor = COMPACT_MAIN_X.get_or_init(|| Mutex::new(None));
    let main_x = {
        let mut saved = anchor.lock().map_err(|error| error.to_string())?;
        if expanded && saved.is_none() {
            *saved = Some(inferred_main_x);
        }
        saved.unwrap_or(inferred_main_x)
    };
    let mut x = main_x
        .saturating_sub(i32::try_from(target_left_inset).unwrap_or(i32::MAX));
    let mut y = current_position.y;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let work = monitor.work_area();
        let gap = (8.0 * scale).round() as i32;
        let min_x = work.position.x.saturating_add(gap);
        let min_y = work.position.y.saturating_add(gap);
        let max_x = work
            .position
            .x
            .saturating_add(i32::try_from(work.size.width).unwrap_or(i32::MAX))
            .saturating_sub(i32::try_from(target_width).unwrap_or(i32::MAX))
            .saturating_sub(gap);
        let max_y = work
            .position
            .y
            .saturating_add(i32::try_from(work.size.height).unwrap_or(i32::MAX))
            .saturating_sub(i32::try_from(target_height).unwrap_or(i32::MAX))
            .saturating_sub(gap);
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }

    window
        .set_size(PhysicalSize::new(target_width, target_height))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())?;
    if !expanded {
        *anchor.lock().map_err(|error| error.to_string())? = None;
    }
    Ok(())
}

fn toggle_main_window(app: &tauri::AppHandle, tray_rect: Rect) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    position_near_tray(app, &window, tray_rect);
    let _ = window.show();
    let _ = window.set_focus();
    refresh_usage_on_open(app);
}

fn refresh_usage_on_open(app: &tauri::AppHandle) {
    let pool = app.state::<AppState>().pool.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        adapters::cursor::refresh_codex_and_cursor_on_open(pool).await;
        let _ = app.emit("usage-refreshed", ());
    });
}

fn position_near_tray(app: &tauri::AppHandle, window: &tauri::WebviewWindow, tray_rect: Rect) {
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let Ok(monitors) = app.available_monitors() else {
        return;
    };

    let tray_position = tray_rect.position.to_physical::<f64>(1.0);
    let tray_size = tray_rect.size.to_physical::<f64>(1.0);
    let tray_center_x = tray_position.x + tray_size.width / 2.0;
    let tray_center_y = tray_position.y + tray_size.height / 2.0;
    let Some(monitor) = monitors.iter().find(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        tray_center_x >= f64::from(position.x)
            && tray_center_x <= f64::from(position.x) + f64::from(size.width)
            && tray_center_y >= f64::from(position.y)
            && tray_center_y <= f64::from(position.y) + f64::from(size.height)
    }) else {
        return;
    };

    let work = monitor.work_area();
    let work_left = f64::from(work.position.x);
    let work_top = f64::from(work.position.y);
    let work_right = work_left + f64::from(work.size.width);
    let work_bottom = work_top + f64::from(work.size.height);
    let width = f64::from(window_size.width);
    let height = f64::from(window_size.height);
    let gap = 8.0;

    let mut x = tray_center_x - width / 2.0;
    let mut y;

    if tray_center_y >= work_bottom {
        // Windows taskbar at the bottom.
        y = tray_position.y - height - gap;
    } else if tray_center_y <= work_top {
        // macOS menu bar or a top-aligned Windows taskbar.
        y = tray_position.y + tray_size.height + gap;
    } else if tray_center_x >= work_right {
        // Right-aligned Windows taskbar.
        x = tray_position.x - width - gap;
        y = tray_center_y - height / 2.0;
    } else {
        // Left-aligned Windows taskbar.
        x = tray_position.x + tray_size.width + gap;
        y = tray_center_y - height / 2.0;
    }

    x = x.clamp(work_left + gap, work_right - width - gap);
    y = y.clamp(work_top + gap, work_bottom - height - gap);
    let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}
