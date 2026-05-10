// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod dev_tools;
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
        // - send_files: 在已有键鼠连接旁路发送文件
        // - answer_device_trust/get_connection_state: 设备信任与连接状态
        .invoke_handler(tauri::generate_handler![
            mouse_share::start_mouse_server,
            mouse_share::start_mouse_client,
            mouse_share::stop_sharing,
            mouse_share::send_files,
            mouse_share::answer_device_trust,
            mouse_share::get_connection_state,
            dev_tools::get_network_summary,
            dev_tools::check_port_available,
            dev_tools::probe_tcp_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
