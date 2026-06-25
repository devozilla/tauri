#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::time::SystemTime;
use base64::{Engine as _, engine::general_purpose};
use tauri::{Manager, Emitter};

use tauri::Runtime;
use tauri_plugin_powersync::PowerSyncExt;
use powersync::{
    BackendConnector,
    SyncOptions,
    PowerSyncCredentials,
    error::PowerSyncError,
};
use async_trait::async_trait;

// ════════════════════════════════════════════
//  Connector — read-only
// ════════════════════════════════════════════
struct MyConnector {
    endpoint: String,
    token: String,
}

#[async_trait]
impl BackendConnector for MyConnector {
    async fn fetch_credentials(&self) -> Result<PowerSyncCredentials, PowerSyncError> {
        Ok(PowerSyncCredentials {
            endpoint: self.endpoint.clone(),
            token: self.token.clone(),
        })
    }

    async fn upload_data(&self) -> Result<(), PowerSyncError> {
        Ok(())
    }
}

// ════════════════════════════════════════════
//  Updater
// ════════════════════════════════════════════
fn get_dist_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .unwrap()
        .join("web_dist")
}

#[tauri::command]
async fn download_update(app: tauri::AppHandle, download_url: String) -> Result<(), String> {
    let dist_path = get_dist_path(&app);

    let _ = app.emit("update-progress", serde_json::json!({
        "status": "downloading",
        "progress": 0
    }));

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("update-progress", serde_json::json!({
        "status": "extracting",
        "progress": 60
    }));

    if dist_path.exists() {
        std::fs::remove_dir_all(&dist_path).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dist_path).map_err(|e| e.to_string())?;

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = dist_path.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    let _ = app.emit("update-progress", serde_json::json!({
        "status": "ready",
        "progress": 100
    }));

    Ok(())
}

#[tauri::command]
async fn restart_app(app: tauri::AppHandle) -> Result<(), String> {
    app.restart();
}

// ════════════════════════════════════════════
//  Tauri Commands
// ════════════════════════════════════════════
#[tauri::command]
async fn connect<R: Runtime>(
    app: tauri::AppHandle<R>,
    handle: usize,
    endpoint: String,
    token: String,
) -> tauri_plugin_powersync::Result<()> {
    let ps = app.powersync();
    let database = ps.database_from_javascript_handle(handle)?;
    let options = SyncOptions::new(MyConnector { endpoint, token });
    database.connect(options).await;
    Ok(())
}

#[tauri::command]
fn print_pdf(app: tauri::AppHandle, pdf_base64: String, printer_name: String, orientation: String) {
    let pdf_bytes = general_purpose::STANDARD
        .decode(&pdf_base64)
        .expect("failed to decode base64 PDF");

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let pdf_path = std::env::temp_dir().join(format!("print_{}.pdf", timestamp));
    std::fs::write(&pdf_path, &pdf_bytes).expect("failed to write PDF");

    let sumatra = app
        .path()
        .resolve("resources/SumatraPDF.exe", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve SumatraPDF path");

    let mut child = Command::new(&sumatra)
        .args([
            "-print-to", &printer_name,
            "-print-settings", &format!("{},fit,noShrink,0x0", orientation),
            "-silent",
            "-zoom", "100",
            pdf_path.to_str().unwrap(),
        ])
        .spawn()
        .expect("failed to launch SumatraPDF");

    child.wait().expect("failed to wait for SumatraPDF");

    std::thread::sleep(std::time::Duration::from_millis(300));
}

#[tauri::command]
fn get_printers_silent() -> String {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name | ConvertTo-Json",
        ])
        .output()
        .expect("failed to run powershell");

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let trimmed = raw.trim();
    if trimmed.starts_with('[') {
        trimmed.to_string()
    } else if trimmed.starts_with('"') {
        format!("[{}]", trimmed)
    } else {
        "[]".to_string()
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // ✅ تأكد إن الـ app data directory موجود
            // ده بيحل مشكلة PowerSync لما بيحاول ينشئ الـ .db
            let app_data_dir = app.path().app_data_dir().unwrap();
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir)
                    .expect("failed to create app data directory");
            }

            // ✅ شغّل الـ local dist لو موجود
            let dist_path = get_dist_path(app.handle());
            let index_path = dist_path.join("index.html");
            let window = app.get_webview_window("main").unwrap();

            if index_path.exists() {
                let url = format!("file://{}", index_path.to_str().unwrap());
                let _ = window.navigate(url.parse().unwrap());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            print_pdf,
            get_printers_silent,
            connect,
            download_update,
            restart_app,
        ])
        .plugin(tauri_plugin_powersync::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}