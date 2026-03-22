// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod logic;
mod mouse_share;
pub mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // 前端只通过这些 command 与后端交互：
        // - start_mouse_server: 控制端（主动连接）
        // - start_mouse_client: 接收端（被动监听）
        // - stop_sharing: 统一停止信号
        // - start_file_server: 文件发送端
        // - start_file_client: 文件接收端
        .invoke_handler(tauri::generate_handler![
            mouse_share::start_mouse_server,
            mouse_share::start_mouse_client,
            mouse_share::stop_sharing,
            mouse_share::start_file_server,
            mouse_share::start_file_client,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
