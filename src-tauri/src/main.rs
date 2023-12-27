// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Manager;
use qzone_exporter::qzone;



fn main() {
    let manager = tauri::Builder::default().setup(|app| {
        #[cfg(debug_assertions)] // only include this code on debug builds
        {
            let window = app.get_window("main").unwrap();
            window.open_devtools();
            window.close_devtools();
        }
        Ok(())
    });
    manager
        .invoke_handler(tauri::generate_handler![
            qzone::qrcode::get_login_qrcode,
            qzone::qrcode::get_login_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
