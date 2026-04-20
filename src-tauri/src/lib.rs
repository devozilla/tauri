use tauri::Manager;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::time::Instant;

// Custom silent printer discovery for Windows
#[tauri::command]
fn get_printers_silent() -> String {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = Command::new("powershell");
        command.args(&["-Command", "Get-Printer | Select-Object Name | ConvertTo-Json"]);
        command.creation_flags(CREATE_NO_WINDOW);
        
        match command.output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(e) => format!("Error: {}", e),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        "[]".to_string()
    }
}

// High-performance asynchronous PDF printing using Microsoft Edge engine (built-in to Windows)
#[tauri::command]
fn print_pdf_silent(path: String, printer: String) -> String {
    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let mut command = Command::new("powershell");
            
            // This script uses the built-in Chromium engine in Edge to print PDFs directly
            // It is much faster and more reliable than the standard shell 'Print' verb
            let script = if printer.is_empty() {
                format!(
                    "Start-Process 'msedge.exe' -ArgumentList '--headless', '--print-to-printer', '\"{0}\"' -WindowStyle Hidden",
                    path
                )
            } else {
                format!(
                    "Start-Process 'msedge.exe' -ArgumentList '--headless', '--print-to-printer', '--printer-name=\"{1}\"', '\"{0}\"' -WindowStyle Hidden",
                    path, printer
                )
            };
            
            command.args(&["-Command", &script]);
            command.creation_flags(CREATE_NO_WINDOW);
            let _ = command.output();
        });
        "Sent to spooler".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    { "Not implemented for this OS".to_string() }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let start_time = Instant::now();
    println!("[Backend] Starting Tauri Application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_printer_v2::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![greet, get_printers_silent, print_pdf_silent])
        .setup(move |_app| {
            println!("[Backend] App setup took: {:?}ms", start_time.elapsed().as_millis());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
