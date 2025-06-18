// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod mouse_share;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            mouse_share::start_mouse_server,
            mouse_share::start_mouse_client,
            mouse_share::stop_sharing
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
