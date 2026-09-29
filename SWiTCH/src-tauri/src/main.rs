#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager; // <-- Das hat gefehlt!

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        switch_backend::run_backend().await;
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten der App");
}