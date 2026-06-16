// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::time::SystemTime;
use base64::{Engine as _, engine::general_purpose};
use tauri::Manager;

// ✅ الـ imports الصح من المصادر الصح
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
        Ok(()) // read-only
    }
}

// ════════════════════════════════════════════
//  Tauri Command
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
    // 1. Decode and write PDF to temp
    let pdf_bytes = general_purpose::STANDARD
        .decode(&pdf_base64)
        .expect("failed to decode base64 PDF");

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let pdf_path = std::env::temp_dir().join(format!("print_{}.pdf", timestamp));
    std::fs::write(&pdf_path, &pdf_bytes).expect("failed to write PDF");

    // 2. Resolve SumatraPDF from bundled resources
    let sumatra = app
        .path()
        .resolve("resources/SumatraPDF.exe", tauri::path::BaseDirectory::Resource)
        .expect("failed to resolve SumatraPDF path");

    // 3. Print silently to default printer
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
    
    // PowerShell returns a plain string (not array) if only 1 printer
    // Normalize to always return a JSON array
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
        .invoke_handler(tauri::generate_handler![print_pdf, get_printers_silent, connect])
        .plugin(tauri_plugin_powersync::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}



// fn main() {
//     top_lib::run()
// }
