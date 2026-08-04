mod activity;
mod adapters;
mod ai_xp;
mod commands;
mod database;
mod diagnostics;
mod host_metrics;
mod keyboard;
mod tray;
mod whip;
mod xp_boost;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            diagnostics::init(&app_data_dir)?;
            let database_url = format!("sqlite://{}", app_data_dir.join("rundev.db").display());
            let pool = tauri::async_runtime::block_on(database::connect(&database_url))?;
            let selected_runner: Option<String> = tauri::async_runtime::block_on(
                sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'runner.selected'")
                    .fetch_optional(&pool),
            )?;
            tray::set_runner(selected_runner.as_deref().unwrap_or("coding-cat"));
            app.manage(AppState { pool: pool.clone() });
            activity::start(pool.clone(), app.handle().clone());
            keyboard::start(pool.clone(), app.handle().clone());
            host_metrics::start(app.handle().clone());
            tauri::async_runtime::spawn(adapters::claude::serve(pool.clone()));
            let codex_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    if adapters::codex::is_enabled(&codex_pool)
                        .await
                        .unwrap_or(false)
                    {
                        if let Err(error) = adapters::codex::sync(&codex_pool).await {
                            tracing::warn!(%error, "Codex usage synchronization failed");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                }
            });
            let cursor_pool = pool.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                loop {
                    if adapters::cursor::automatic_sync_allowed(&cursor_pool)
                        .await
                        .unwrap_or(false)
                    {
                        if let Err(error) = adapters::cursor::sync(&cursor_pool).await {
                            tracing::warn!(%error, "Cursor usage synchronization failed");
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
            commands::get_focus_activity_today,
            commands::get_activity_history,
            commands::get_character_state,
            commands::get_ai_usage_today,
            commands::set_codex_usage_enabled,
            commands::preview_codex_account,
            commands::connect_codex_account,
            commands::grant_cursor_usage_consent,
            commands::preview_cursor_account,
            commands::connect_cursor_account,
            commands::disconnect_cursor_account,
            commands::get_cursor_usage,
            commands::refresh_cursor_usage,
            commands::preview_claude_connection,
            commands::connect_claude,
            commands::disconnect_claude,
            commands::get_claude_usage_today,
            commands::get_ai_activity_status,
            commands::get_keyboard_activity_today,
            commands::open_keyboard_permission_settings,
            commands::reset_keyboard_permission,
            commands::open_diagnostics_folder,
            commands::get_runner_selection,
            commands::set_runner_selection,
            commands::get_system_stats,
            commands::set_system_panel_expanded,
            commands::get_whip_stats,
            commands::record_whip,
            commands::preview_xp_coupon,
            commands::redeem_xp_coupon,
            commands::get_xp_boost_status,
            commands::sync_ai_weekly_xp
        ])
        .run(tauri::generate_context!())
        .expect("error while running RunDev");
}
