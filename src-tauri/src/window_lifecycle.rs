//! Webviews are disposable views; durable state lives in Rust/SQLite.
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WebviewWindowBuilder};

static MAIN_GENERATION: AtomicU64 = AtomicU64::new(0);
const MAIN_IDLE_SECONDS: u64 = 30;

pub fn ensure(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == label)
        .expect("configured RunDev window");
    WebviewWindowBuilder::from_config(app, config)?.build()
}

pub fn main_opened() {
    MAIN_GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub fn hide_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.hide();
    let _ = app.emit_to("main", "rundev-window-visible", false);
    let _ = crate::host_metrics::set_sampling_mode("background");
    let generation = MAIN_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(MAIN_IDLE_SECONDS)).await;
        // Serialize the final visibility check and destruction with tray clicks.
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if MAIN_GENERATION.load(Ordering::Relaxed) != generation {
                return;
            }
            if let Some(window) = handle.get_webview_window("main") {
                if !window.is_visible().unwrap_or(true) {
                    let _ = window.destroy();
                }
            }
        });
    });
}

pub fn destroy(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.destroy();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_views_are_created_on_demand_instead_of_at_startup() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let windows = config["app"]["windows"].as_array().unwrap();
        assert_eq!(windows.len(), 3);
        assert!(windows.iter().all(|window| window["create"] == false));
    }
}
