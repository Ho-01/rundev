use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition};

fn placement(
    origin: (i32, i32),
    sprite: (u32, u32),
    size: (u32, u32),
    gap: i32,
    work: Option<(i32, i32, u32, u32)>,
) -> PhysicalPosition<i32> {
    let mut x = origin.0 + (sprite.0 as i32 - size.0 as i32) / 2;
    let mut y = origin.1 + sprite.1 as i32 / 2 - size.1 as i32 + gap;
    if let Some((left, top, width, height)) = work {
        let right = left + width as i32 - size.0 as i32;
        let bottom = top + height as i32 - size.1 as i32;
        x = x.clamp(left, right.max(left));
        y = y.clamp(top, bottom.max(top));
    }
    PhysicalPosition::new(x, y)
}

/// Keep the salary companion separate from the sprite's drag/resize hit area.
pub fn position(app: &AppHandle) {
    let (Some(character), Some(overlay)) = (
        app.get_webview_window("character"),
        app.get_webview_window("character-salary"),
    ) else {
        return;
    };
    let (Ok(origin), Ok(sprite)) = (character.outer_position(), character.outer_size()) else {
        return;
    };
    let character_scale = character.scale_factor().unwrap_or(1.0);
    let character_size = f64::from(sprite.height) / character_scale;
    let bubble_scale = (character_size / 48.0).clamp(0.75, 4.0);
    let logical_size = LogicalSize::new(240.0 * bubble_scale, 200.0 * bubble_scale);
    let overlay_scale = overlay.scale_factor().unwrap_or(character_scale);
    if let Ok(current) = overlay.inner_size() {
        if (f64::from(current.width) / overlay_scale - logical_size.width).abs() > 0.5
            || (f64::from(current.height) / overlay_scale - logical_size.height).abs() > 0.5
        {
            let _ = overlay.set_size(logical_size);
        }
    }
    let Ok(size) = overlay.outer_size() else {
        return;
    };
    let gap = (12.0 * character_scale).round() as i32;
    let work = character.current_monitor().ok().flatten().map(|monitor| {
        let work = monitor.work_area();
        (
            work.position.x,
            work.position.y,
            work.size.width,
            work.size.height,
        )
    });
    let next = placement(
        (origin.x, origin.y),
        (sprite.width, sprite.height),
        (size.width, size.height),
        gap,
        work,
    );
    if overlay.outer_position().ok() != Some(next) {
        let _ = overlay.set_position(next);
    }
    // The companion overlaps the sprite so each gain starts at the sprite's center.
    // Report the real center after monitor clamping and DPI conversion.
    if let (Ok(actual), Ok(scale)) = (overlay.outer_position(), overlay.scale_factor()) {
        #[derive(Clone, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Anchor {
            x: f64,
            y: f64,
            character_size: f64,
        }
        let _ = app.emit_to(
            "character-salary",
            "character-salary-anchor",
            Anchor {
                character_size,
                x: (f64::from(origin.x) + f64::from(sprite.width) / 2.0 - f64::from(actual.x))
                    / scale,
                y: (f64::from(origin.y) + f64::from(sprite.height) / 2.0 - f64::from(actual.y))
                    / scale,
            },
        );
    }
}

pub fn start(app: AppHandle) -> Result<(), tauri::Error> {
    tauri::async_runtime::spawn(async move {
        let mut configured = false;
        let mut checked = Instant::now() - Duration::from_secs(2);
        loop {
            let character_visible = app
                .get_webview_window("character")
                .is_some_and(|character| character.is_visible().unwrap_or(false));
            if !character_visible {
                crate::window_lifecycle::destroy(&app, "character-salary");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            if checked.elapsed() >= Duration::from_secs(1) {
                configured = crate::salary::load(&app.state::<crate::database::AppState>().pool)
                    .await
                    .ok()
                    .flatten()
                    .is_some();
                checked = Instant::now();
            }
            let visible = configured
                && app
                    .get_webview_window("character")
                    .and_then(|character| character.is_visible().ok())
                    .unwrap_or(false);
            if visible {
                let Ok(overlay) = crate::window_lifecycle::ensure(&app, "character-salary") else {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                };
                let _ = overlay.set_ignore_cursor_events(true);
                position(&app);
                if !overlay.is_visible().unwrap_or(false) {
                    let _ = overlay.show();
                    let _ = app.emit_to("character-salary", "rundev-window-visible", true);
                }
            } else {
                crate::window_lifecycle::destroy(&app, "character-salary");
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlaps_character_center_without_changing_its_hit_area() {
        assert_eq!(
            placement(
                (500, 400),
                (48, 48),
                (240, 200),
                12,
                Some((0, 0, 1920, 1080))
            ),
            PhysicalPosition::new(404, 236)
        );
    }

    #[test]
    fn clamps_to_monitor_edges_while_preserving_the_real_center() {
        assert_eq!(
            placement((0, 0), (48, 48), (240, 200), 12, Some((0, 0, 1920, 1080))),
            PhysicalPosition::new(0, 0)
        );
        assert_eq!(
            placement(
                (1900, 500),
                (48, 48),
                (240, 200),
                12,
                Some((0, 0, 1920, 1080))
            ),
            PhysicalPosition::new(1680, 336)
        );
    }

    #[test]
    fn handles_negative_monitor_coordinates_and_scaled_sizes() {
        assert_eq!(
            placement(
                (-1920, -200),
                (96, 96),
                (480, 400),
                24,
                Some((-1920, -300, 1920, 1080))
            ),
            PhysicalPosition::new(-1920, -300)
        );
    }
}
