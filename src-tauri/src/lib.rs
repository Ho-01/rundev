mod adapters;
mod commands;
mod database;
mod tray;

use database::AppState;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("rundev=info".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let database_url = format!("sqlite://{}", app_data_dir.join("rundev.db").display());
            let pool = tauri::async_runtime::block_on(database::connect(&database_url))?;
            app.manage(AppState { pool: pool.clone() });
            tauri::async_runtime::spawn(async move {
                loop {
                    if adapters::codex::is_enabled(&pool).await.unwrap_or(false) {
                        if let Err(error) = adapters::codex::sync(&pool).await {
                            tracing::warn!(%error, "Codex usage synchronization failed");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                }
            });
            tray::create(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Focused(false) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_daily_summary,
            commands::get_character_state,
            commands::get_ai_usage_today,
            commands::set_codex_usage_enabled,
            commands::preview_codex_account,
            commands::connect_codex_account
        ])
        .run(tauri::generate_context!())
        .expect("error while running RunDev");
}
