use tauri::Emitter;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e: tauri_plugin_updater::Error| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => {
            // Notify frontend that an update is available
            let version = update.version.clone();
            let body = update.body.clone();
            app.emit("update-available", serde_json::json!({
                "version": version,
                "body": body,
            }))
            .map_err(|e| e.to_string())?;

            // Download and install the update
            update
                .download_and_install(|_chunk, _total| {}, || {})
                .await
                .map_err(|e| e.to_string())?;

            // Restart the app after installation
            app.restart();
        }
        Ok(None) => {
            let _ = app.emit("update-not-available", ());
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_printer_v2::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_updater::Builder::new()
                .header(
                    "Authorization",
                    format!("token {}", "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"),
                )
                .expect("failed to add authorization header")
                .build(),
        )
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "API Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app: &tauri::AppHandle, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "settings" => {
                        let _ = tauri::WebviewWindowBuilder::new(
                            app,
                            "api_settings",
                            tauri::WebviewUrl::App("/api-settings".into()),
                        )
                        .title("API Configuration")
                        .inner_size(400.0, 400.0)
                        .resizable(true)
                        .always_on_top(true)
                        .build();
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, check_for_updates])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
