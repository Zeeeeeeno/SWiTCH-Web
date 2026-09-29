#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    // Startet dein Backend im Hintergrund
    tokio::spawn(async {
        switch_backend::run_backend().await;
    });

    // Startet das Tauri-Fenster
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        // 2. Window State: Speichert automatisch Größe & Position beim Beenden
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // 3. Updater Plugin
        .plugin(tauri_plugin_updater::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("Failed to start App");
}