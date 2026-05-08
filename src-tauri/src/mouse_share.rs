use crate::logic::{
    hide_cursor, move_cursor_to, show_cursor, simulate_button_down, simulate_button_up,
    simulate_key_down, simulate_key_up, simulate_wheel, start_event_listener, AnyEvent,
    KeyEventKind, MouseEvent, MouseEventKind,
};
use base64::Engine;
use rdev::display_size;
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

// 核心说明：
// 1. `start_mouse_server` 是控制端（主动连接到接收端）；
// 2. `start_mouse_client` 是接收端（监听端口，执行远端输入）；
// 3. 握手阶段：配对码验证；
// 4. 业务阶段：所有消息使用明文 DATA 帧传输。
lazy_static::lazy_static! {
    // 线程共享运行状态，stop_sharing 会将其置为 false。
    static ref IS_RUNNING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // 仅表示当前是否存在已鉴权连接，供输入采集线程判断是否进入共享态。
    static ref IS_CONNECTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // 接收端鉴权限流状态（按来源 IP 统计）。
    static ref AUTH_RATE_LIMITER: Mutex<AuthRateLimiter> = Mutex::new(AuthRateLimiter::default());
}

// 鼠标移动速度系数，用于平衡不同系统间的鼠标加速差异
const MOUSE_SPEED_FACTOR: f64 = 1.0;

const AUTH_OK: &str = "AUTH_OK";
const AUTH_FAIL: &str = "AUTH_FAIL";
const DATA_PREFIX: &str = "DATA ";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);
const AUTH_WINDOW: Duration = Duration::from_secs(60);
const AUTH_LOCK_DURATION: Duration = Duration::from_secs(120);
const AUTH_MAX_FAILS: usize = 5;

#[derive(Default)]
struct AuthRateLimiter {
    // 在 AUTH_WINDOW 时间窗内的失败时间点。
    failed_attempts: HashMap<IpAddr, Vec<Instant>>,
    // 被封禁到什么时间点。
    blocked_until: HashMap<IpAddr, Instant>,
}

#[derive(serde::Serialize, Clone)]
struct RuntimeLogEvent {
    level: String,
    message: String,
    ts_ms: u64,
}

fn is_socket_timeout(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

// 统一把后端运行信息推给前端日志面板（事件名：kms-log）。
fn emit_runtime_log(app: &AppHandle, level: &str, message: impl Into<String>) {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let event = RuntimeLogEvent {
        level: level.to_string(),
        message: message.into(),
        ts_ms,
    };
    let _ = app.emit("kms-log", event);
}

fn begin_session() -> Result<(), String> {
    let mut running = IS_RUNNING.lock().unwrap();
    if *running {
        return Err("已有共享任务正在运行，请先停止当前任务".to_string());
    }
    *running = true;
    drop(running);

    let mut connected = IS_CONNECTED.lock().unwrap();
    *connected = false;
    Ok(())
}

fn finish_session() {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = false;
    drop(running);

    let mut connected = IS_CONNECTED.lock().unwrap();
    *connected = false;
    show_cursor();
}

// 配对码强度策略：至少 8 位，且必须包含字母+数字。
fn validate_pair_code_strength(pair_code: &str) -> Result<(), String> {
    if pair_code.len() < 8 {
        return Err("配对码至少 8 位".to_string());
    }
    let has_alpha = pair_code.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = pair_code.chars().any(|c| c.is_ascii_digit());
    if !(has_alpha && has_digit) {
        return Err("配对码必须同时包含字母和数字".to_string());
    }
    Ok(())
}

// 清理时间窗外的失败记录，避免哈希表无限增长。
fn auth_prune_old_attempts(ip: IpAddr, limiter: &mut AuthRateLimiter, now: Instant) {
    if let Some(history) = limiter.failed_attempts.get_mut(&ip) {
        history.retain(|ts| now.duration_since(*ts) <= AUTH_WINDOW);
        if history.is_empty() {
            limiter.failed_attempts.remove(&ip);
        }
    }
}

// 查询某个来源 IP 是否仍在封禁期。
fn auth_block_remaining(ip: IpAddr) -> Option<Duration> {
    let now = Instant::now();
    let mut limiter = AUTH_RATE_LIMITER.lock().unwrap();

    if let Some(until) = limiter.blocked_until.get(&ip).copied() {
        if until > now {
            return Some(until.duration_since(now));
        }
        limiter.blocked_until.remove(&ip);
    }

    auth_prune_old_attempts(ip, &mut limiter, now);
    None
}

// 记录一次失败；达到阈值后进入临时封禁。
fn auth_record_failure(ip: IpAddr) {
    let now = Instant::now();
    let mut limiter = AUTH_RATE_LIMITER.lock().unwrap();
    auth_prune_old_attempts(ip, &mut limiter, now);
    let should_block = {
        let history = limiter.failed_attempts.entry(ip).or_default();
        history.push(now);
        history.len() >= AUTH_MAX_FAILS
    };
    if should_block {
        limiter.blocked_until.insert(ip, now + AUTH_LOCK_DURATION);
        limiter.failed_attempts.remove(&ip);
    }
}

// 鉴权成功后，清理该 IP 的历史失败与封禁状态。
fn auth_record_success(ip: IpAddr) {
    let mut limiter = AUTH_RATE_LIMITER.lock().unwrap();
    limiter.failed_attempts.remove(&ip);
    limiter.blocked_until.remove(&ip);
}

// 简化的明文帧封装。
fn encode_frame(plain: &str) -> String {
    format!("{DATA_PREFIX}{}\n", plain)
}

// 简化的明文帧解析。
fn decode_frame(frame: &str) -> Result<String, String> {
    let frame = frame.trim();
    let Some(rest) = frame.strip_prefix(DATA_PREFIX) else {
        return Err("数据帧协议不匹配".to_string());
    };
    Ok(rest.to_string())
}

// 文件传输帧定义。
const FILE_META_PREFIX: &str = "FILE_META ";
const FILE_CHUNK_PREFIX: &str = "FILE_CHUNK ";
const FILE_DONE: &str = "FILE_DONE";
const FILE_DONE_PREFIX: &str = "FILE_DONE ";
const FILE_ABORT: &str = "FILE_ABORT";

fn encode_file_meta(file_name: &str, file_size: u64) -> String {
    format!("{FILE_META_PREFIX}{}|{}\n", file_name, file_size)
}

fn encode_file_chunk(chunk_b64: &str) -> String {
    format!("{FILE_CHUNK_PREFIX}{}\n", chunk_b64)
}

fn encode_file_done(file_name: &str, file_size: u64, checksum: u64) -> String {
    format!(
        "{FILE_DONE_PREFIX}{}|{}|{}\n",
        file_name, file_size, checksum
    )
}

fn checksum_update(mut checksum: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        checksum = checksum.wrapping_mul(16_777_619) ^ u64::from(*byte);
    }
    checksum
}

fn decode_file_meta(frame: &str) -> Result<(String, u64), String> {
    let frame = frame.trim();
    let Some(payload) = frame.strip_prefix(FILE_META_PREFIX) else {
        return Err("文件元信息协议不匹配".to_string());
    };
    let mut parts = payload.splitn(2, '|');
    let name = parts
        .next()
        .ok_or_else(|| "文件名缺失".to_string())?
        .to_string();
    let size = parts
        .next()
        .ok_or_else(|| "文件大小缺失".to_string())?
        .parse::<u64>()
        .map_err(|_| "文件大小解析失败".to_string())?;
    Ok((name, size))
}

fn decode_file_done(frame: &str) -> Result<(String, u64, u64), String> {
    let frame = frame.trim();
    let Some(payload) = frame.strip_prefix(FILE_DONE_PREFIX) else {
        return Err("文件完成帧协议不匹配".to_string());
    };
    let mut parts = payload.splitn(3, '|');
    let name = parts
        .next()
        .ok_or_else(|| "文件名缺失".to_string())?
        .to_string();
    let size = parts
        .next()
        .ok_or_else(|| "文件大小缺失".to_string())?
        .parse::<u64>()
        .map_err(|_| "文件大小解析失败".to_string())?;
    let checksum = parts
        .next()
        .ok_or_else(|| "文件校验值缺失".to_string())?
        .parse::<u64>()
        .map_err(|_| "文件校验值解析失败".to_string())?;
    Ok((name, size, checksum))
}

fn decode_file_chunk(frame: &str) -> Result<Vec<u8>, String> {
    let frame = frame.trim();
    let Some(payload) = frame.strip_prefix(FILE_CHUNK_PREFIX) else {
        return Err("文件块协议不匹配".to_string());
    };
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| format!("文件块base64解析失败: {}", e))
}

fn unique_download_path(dir: &std::path::Path, file_name: &str) -> std::path::PathBuf {
    let original = std::path::Path::new(file_name);
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    let ext = original.extension().and_then(|e| e.to_str());
    let mut path = dir.join(file_name);
    let mut index = 1;

    while path.exists() {
        let candidate = match ext {
            Some(ext) if !ext.is_empty() => format!("{stem} ({index}).{ext}"),
            _ => format!("{stem} ({index})"),
        };
        path = dir.join(candidate);
        index += 1;
    }

    path
}

// 接收端握手（简化版：无加密）。
fn authenticate_client_connection(stream: &mut TcpStream, pair_code: &str) -> Result<(), String> {
    stream
        .write_all(format!("CHALLENGE {pair_code}\n").as_bytes())
        .map_err(|e| format!("发送挑战失败: {}", e))?;
    stream.flush().map_err(|e| format!("刷新挑战失败: {}", e))?;

    let reader_stream = stream
        .try_clone()
        .map_err(|e| format!("复制连接失败: {}", e))?;
    let mut reader = BufReader::new(reader_stream);

    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|e| format!("读取握手失败: {}", e))?;
    if read == 0 {
        return Err("握手失败：连接被关闭".to_string());
    }

    let response = line.trim();
    if response == AUTH_OK {
        Ok(())
    } else {
        let _ = stream.write_all(format!("{AUTH_FAIL}\n").as_bytes());
        let _ = stream.flush();
        Err("握手失败：配对码错误".to_string())
    }
}

// 控制端握手（简化版：无加密）。
fn authenticate_server_connection(stream: &mut TcpStream, pair_code: &str) -> Result<(), String> {
    let reader_stream = stream
        .try_clone()
        .map_err(|e| format!("复制连接失败: {}", e))?;
    let mut reader = BufReader::new(reader_stream);

    let mut challenge = String::new();
    let read = reader
        .read_line(&mut challenge)
        .map_err(|e| format!("读取挑战失败: {}", e))?;
    if read == 0 {
        return Err("握手失败：连接被关闭".to_string());
    }

    let Some(server_pair_code) = challenge.trim().strip_prefix("CHALLENGE ") else {
        return Err("握手失败：挑战协议不匹配".to_string());
    };

    if server_pair_code != pair_code {
        stream
            .write_all(format!("{AUTH_FAIL}\n").as_bytes())
            .map_err(|e| format!("发送握手请求失败: {}", e))?;
        stream
            .flush()
            .map_err(|e| format!("刷新握手请求失败: {}", e))?;
        return Err("握手失败：配对码不匹配".to_string());
    }

    stream
        .write_all(format!("{AUTH_OK}\n").as_bytes())
        .map_err(|e| format!("发送握手请求失败: {}", e))?;
    stream
        .flush()
        .map_err(|e| format!("刷新握手请求失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn stop_sharing(app: AppHandle) {
    // 通过共享开关通知所有工作线程尽快退出，并立即恢复本机可见状态。
    finish_session();
    emit_runtime_log(&app, "info", "共享已停止");
    println!("停止共享");
}

#[tauri::command]
pub fn start_mouse_client(app: AppHandle, port: u16, pair_code: String) -> Result<(), String> {
    // 先在命令入口做参数合法性校验，避免启动后才失败。
    let pair_code = pair_code.trim().to_string();
    validate_pair_code_strength(&pair_code)?;
    begin_session()?;

    emit_runtime_log(&app, "info", format!("客户端启动，监听端口 {}", port));

    let is_running = Arc::clone(&IS_RUNNING);
    let app_handle = app.clone();
    thread::spawn(move || {
        // 客户端角色：监听端口，等待控制端接入。
        // 同时监听 IPv4 和 IPv6
        let listener = match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => listener,
            Err(e) => {
                // 如果 IPv4 失败，尝试 IPv6
                match TcpListener::bind(("::", port)) {
                    Ok(listener) => listener,
                    Err(e2) => {
                        emit_runtime_log(
                            &app_handle,
                            "error",
                            format!("客户端监听失败(IPv4: {}, IPv6: {})", e, e2),
                        );
                        println!("[客户端] 监听端口失败: IPv4={}, IPv6={}", e, e2);
                        finish_session();
                        return;
                    }
                }
            }
        };

        if let Err(e) = listener.set_nonblocking(true) {
            println!("[客户端] 设置监听非阻塞失败: {}", e);
        }

        let local_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        println!("客户端监听端口: {} (地址: {})", port, local_addr);
        emit_runtime_log(
            &app_handle,
            "info",
            format!("客户端已启动，监听 {}，等待连接...", local_addr),
        );

        while *is_running.lock().unwrap() {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    let peer_ip = addr.ip();

                    if let Err(e) = stream.set_nonblocking(false) {
                        println!("[客户端] 设置阻塞模式失败: {}", e);
                    }

                    // 限流挡板：封禁窗口内直接拒绝连接，不进入握手流程。
                    if let Some(remaining) = auth_block_remaining(peer_ip) {
                        emit_runtime_log(
                            &app_handle,
                            "warn",
                            format!(
                                "拒绝 {} 的连接：鉴权失败过多，剩余锁定 {} 秒",
                                peer_ip,
                                remaining.as_secs()
                            ),
                        );
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }

                    println!("客户端已连接: {}", addr);
                    emit_runtime_log(&app_handle, "info", format!("收到连接请求: {}", addr));

                    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
                    let _ = stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT));

                    // 握手失败计入限流；成功则清空失败历史。
                    match authenticate_client_connection(&mut stream, &pair_code) {
                        Ok(_) => {}
                        Err(e) => {
                            auth_record_failure(peer_ip);
                            emit_runtime_log(
                                &app_handle,
                                "warn",
                                format!("客户端鉴权失败（{}）: {}", peer_ip, e),
                            );
                            println!("[客户端] {}", e);
                            let _ = stream.shutdown(Shutdown::Both);
                            continue;
                        }
                    };
                    auth_record_success(peer_ip);
                    emit_runtime_log(&app_handle, "success", format!("客户端鉴权通过: {}", addr));

                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

                    show_cursor();

                    let reader_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            println!("[客户端] 复制连接失败: {}", e);
                            continue;
                        }
                    };
                    let mut reader = BufReader::new(reader_stream);

                    // 获取当前默认的鼠标初始位置
                    let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
                    let (mut cur_x, mut cur_y) =
                        ((screen_width / 2) as i32, (screen_height / 2) as i32);
                    let max_x = screen_width as i32 - 1;
                    let max_y = screen_height as i32 - 1;

                    loop {
                        if !*is_running.lock().unwrap() {
                            println!("[客户端] 共享已停止，退出连接循环");
                            break;
                        }

                        let mut line = String::new();
                        match reader.read_line(&mut line) {
                            Ok(0) => {
                                println!("[客户端] 连接已关闭");
                                break;
                            }
                            Ok(_) => {
                                let payload = match decode_frame(&line) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        println!("[客户端] 解析数据失败: {}", e);
                                        break;
                                    }
                                };

                                if payload.trim() == "RELEASE" {
                                    println!("[客户端] 收到 Release 信号，退出循环");
                                    break;
                                }

                                match serde_json::from_str::<AnyEvent>(&payload) {
                                    Ok(AnyEvent::MouseEvent(evt)) => match evt.kind {
                                        MouseEventKind::MoveDelta { dx, dy } => {
                                            // 应用速度系数以平衡鼠标加速差异
                                            let scaled_dx = (dx as f64 * MOUSE_SPEED_FACTOR) as i32;
                                            let scaled_dy = (dy as f64 * MOUSE_SPEED_FACTOR) as i32;

                                            cur_x = (cur_x + scaled_dx).clamp(0, max_x);
                                            cur_y = (cur_y + scaled_dy).clamp(0, max_y);
                                            move_cursor_to(cur_x, cur_y);

                                            // 到达左边缘时通知服务端释放
                                            if cur_x <= 1 {
                                                let frame = encode_frame("RELEASE");
                                                let _ = stream.write_all(frame.as_bytes());
                                                let _ = stream.flush();
                                                println!(
                                                    "[客户端] 到达左边缘，已通知服务端释放控制权"
                                                );
                                            }
                                        }
                                        MouseEventKind::ButtonDown { button } => {
                                            simulate_button_down(&button);
                                        }
                                        MouseEventKind::ButtonUp { button } => {
                                            simulate_button_up(&button);
                                        }
                                        MouseEventKind::Wheel { delta } => {
                                            simulate_wheel(delta);
                                        }
                                        _ => {}
                                    },
                                    Ok(AnyEvent::KeyEvent(evt)) => match evt.kind {
                                        KeyEventKind::KeyDown { key } => {
                                            let res = std::panic::catch_unwind(|| {
                                                simulate_key_down(&key)
                                            });
                                            if let Err(e) = res {
                                                println!("simulate_key_down 崩溃: {:?}", e);
                                            }
                                        }
                                        KeyEventKind::KeyUp { key } => {
                                            let res =
                                                std::panic::catch_unwind(|| simulate_key_up(&key));
                                            if let Err(e) = res {
                                                println!("simulate_key_up 崩溃: {:?}", e);
                                            }
                                        }
                                    },
                                    Err(e) => println!("[客户端] 解析数据错误: {}", e),
                                }
                            }
                            Err(e) if is_socket_timeout(&e) => continue,
                            Err(e) => {
                                println!("[客户端] 读取连接数据失败: {}", e);
                                break;
                            }
                        }
                    }

                    println!("[客户端] 连接断开或循环结束");
                    // 结束前尽量通知控制端恢复本地光标状态。
                    let frame = encode_frame("RELEASE");
                    let _ = stream.write_all(frame.as_bytes());
                    let _ = stream.flush();
                    let _ = stream.shutdown(Shutdown::Both);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    println!("[客户端] 接受连接错误: {}", e);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }

        finish_session();
        emit_runtime_log(&app_handle, "info", "客户端已停止");
    });
    Ok(())
}

#[tauri::command]
pub fn start_file_client(
    app: AppHandle,
    port: u16,
    pair_code: String,
    download_dir: String,
) -> Result<(), String> {
    let pair_code = pair_code.trim().to_string();
    validate_pair_code_strength(&pair_code)?;

    let download_path = std::path::PathBuf::from(download_dir.trim());
    if !download_path.exists() || !download_path.is_dir() {
        return Err("下载目录不存在或不是目录".to_string());
    }
    begin_session()?;

    emit_runtime_log(&app, "info", format!("文件客户端启动，监听端口 {}", port));

    let is_running = Arc::clone(&IS_RUNNING);
    let app_handle = app.clone();
    thread::spawn(move || {
        let listener = match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => listener,
            Err(e) => match TcpListener::bind(("::", port)) {
                Ok(listener) => listener,
                Err(e2) => {
                    emit_runtime_log(
                        &app_handle,
                        "error",
                        format!("文件客户端监听失败(IPv4: {}, IPv6: {})", e, e2),
                    );
                    finish_session();
                    return;
                }
            },
        };

        if let Err(e) = listener.set_nonblocking(true) {
            println!("[文件客户端] 设置监听非阻塞失败: {}", e);
        }

        let local_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        emit_runtime_log(
            &app_handle,
            "info",
            format!("文件客户端已启动，监听 {}，等待连接...", local_addr),
        );

        while *is_running.lock().unwrap() {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    let peer_ip = addr.ip();

                    if let Err(e) = stream.set_nonblocking(false) {
                        println!("[文件客户端] 设置阻塞模式失败: {}", e);
                    }

                    // 限流同鼠标共享
                    if let Some(remaining) = auth_block_remaining(peer_ip) {
                        emit_runtime_log(
                            &app_handle,
                            "warn",
                            format!(
                                "拒绝 {} 的连接：鉴权失败过多，剩余锁定 {} 秒",
                                peer_ip,
                                remaining.as_secs()
                            ),
                        );
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }

                    emit_runtime_log(&app_handle, "info", format!("文件客户端收到连接: {}", addr));

                    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
                    let _ = stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT));

                    if let Err(e) = authenticate_client_connection(&mut stream, &pair_code) {
                        auth_record_failure(peer_ip);
                        emit_runtime_log(
                            &app_handle,
                            "warn",
                            format!("文件客户端鉴权失败({}): {}", peer_ip, e),
                        );
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }

                    auth_record_success(peer_ip);
                    emit_runtime_log(
                        &app_handle,
                        "success",
                        format!("文件客户端鉴权通过: {}", addr),
                    );

                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

                    let reader_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            emit_runtime_log(
                                &app_handle,
                                "error",
                                format!("文件客户端复制连接失败: {}", e),
                            );
                            let _ = stream.shutdown(Shutdown::Both);
                            continue;
                        }
                    };
                    let mut reader = BufReader::new(reader_stream);

                    let mut current_file: Option<std::fs::File> = None;
                    let mut current_path: Option<std::path::PathBuf> = None;
                    let mut expected_size: Option<u64> = None;
                    let mut received: u64 = 0;
                    let mut checksum: u64 = 0;
                    let mut current_name = String::new();

                    loop {
                        if !*is_running.lock().unwrap() {
                            break;
                        }

                        let mut line = String::new();
                        match reader.read_line(&mut line) {
                            Ok(0) => break,
                            Ok(_) => {
                                let trimmed = line.trim();

                                if trimmed == FILE_ABORT {
                                    emit_runtime_log(&app_handle, "warn", "文件传输被发送端中断");
                                    if let Some(mut file) = current_file.take() {
                                        let _ = file.flush();
                                    }
                                    if let Some(path) = current_path.take() {
                                        let _ = std::fs::remove_file(path);
                                    }
                                    current_name.clear();
                                    expected_size = None;
                                    received = 0;
                                    checksum = 0;
                                    continue;
                                }

                                if trimmed == FILE_DONE || trimmed.starts_with(FILE_DONE_PREFIX) {
                                    if let Some(mut file) = current_file.take() {
                                        let _ = file.flush();
                                    }
                                    let done_result = if trimmed == FILE_DONE {
                                        expected_size
                                            .map(|size| (current_name.clone(), size, checksum))
                                            .ok_or_else(|| {
                                                "缺少文件大小，无法校验完成帧".to_string()
                                            })
                                    } else {
                                        decode_file_done(trimmed)
                                    };

                                    match done_result {
                                        Ok((_name, done_size, done_checksum))
                                            if Some(done_size) == expected_size
                                                && received == done_size
                                                && checksum == done_checksum =>
                                        {
                                            emit_runtime_log(
                                                &app_handle,
                                                "success",
                                                format!(
                                                    "文件完成: {} ({} bytes)",
                                                    current_name, received
                                                ),
                                            );
                                        }
                                        Ok((_name, done_size, done_checksum)) => {
                                            emit_runtime_log(
                                                &app_handle,
                                                "error",
                                                format!(
                                                    "文件校验失败: {}，收到 {}/{} bytes，checksum {}/{}",
                                                    current_name,
                                                    received,
                                                    done_size,
                                                    checksum,
                                                    done_checksum
                                                ),
                                            );
                                            if let Some(path) = current_path.take() {
                                                let _ = std::fs::remove_file(path);
                                            }
                                        }
                                        Err(e) => {
                                            emit_runtime_log(
                                                &app_handle,
                                                "error",
                                                format!("文件完成帧无效: {}", e),
                                            );
                                            if let Some(path) = current_path.take() {
                                                let _ = std::fs::remove_file(path);
                                            }
                                        }
                                    }
                                    current_name.clear();
                                    current_path = None;
                                    expected_size = None;
                                    received = 0;
                                    checksum = 0;
                                    continue;
                                }

                                if trimmed.starts_with(FILE_META_PREFIX) {
                                    match decode_file_meta(trimmed) {
                                        Ok((name, size)) => {
                                            let file_name = std::path::Path::new(&name)
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .ok_or_else(|| "文件名不合法".to_string());
                                            match file_name {
                                                Ok(file_name) => {
                                                    if let Some(mut file) = current_file.take() {
                                                        let _ = file.flush();
                                                    }
                                                    if let Some(path) = current_path.take() {
                                                        let _ = std::fs::remove_file(path);
                                                    }

                                                    let path = unique_download_path(
                                                        &download_path,
                                                        file_name,
                                                    );
                                                    match std::fs::File::create(&path) {
                                                        Ok(file) => {
                                                            current_file = Some(file);
                                                            current_path = Some(path.clone());
                                                            expected_size = Some(size);
                                                            received = 0;
                                                            checksum = 0;
                                                            current_name = path
                                                                .file_name()
                                                                .and_then(|n| n.to_str())
                                                                .unwrap_or(file_name)
                                                                .to_string();
                                                            emit_runtime_log(
                                                                &app_handle,
                                                                "info",
                                                                format!(
                                                                    "开始接收文件: {} ({} bytes)",
                                                                    current_name, size
                                                                ),
                                                            );
                                                        }
                                                        Err(e) => {
                                                            emit_runtime_log(
                                                                &app_handle,
                                                                "error",
                                                                format!(
                                                                    "无法创建文件 {}: {}",
                                                                    file_name, e
                                                                ),
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    emit_runtime_log(&app_handle, "error", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            emit_runtime_log(
                                                &app_handle,
                                                "error",
                                                format!("解析文件元信息失败: {}", e),
                                            );
                                        }
                                    }
                                    continue;
                                }

                                if trimmed.starts_with(FILE_CHUNK_PREFIX) {
                                    match decode_file_chunk(trimmed) {
                                        Ok(bytes) => {
                                            if let Some(file) = current_file.as_mut() {
                                                if let Err(e) = file.write_all(&bytes) {
                                                    emit_runtime_log(
                                                        &app_handle,
                                                        "error",
                                                        format!("写文件失败: {}", e),
                                                    );
                                                    break;
                                                }
                                                received += bytes.len() as u64;
                                                checksum = checksum_update(checksum, &bytes);
                                                if let Some(total) = expected_size {
                                                    let pct =
                                                        (received as f64 / total as f64) * 100.0;
                                                    emit_runtime_log(
                                                        &app_handle,
                                                        "info",
                                                        format!(
                                                            "{} 传输中: {:.1}% ({}/{})",
                                                            current_name, pct, received, total
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            emit_runtime_log(
                                                &app_handle,
                                                "error",
                                                format!("解析文件块失败: {}", e),
                                            );
                                            break;
                                        }
                                    }
                                    continue;
                                }

                                // 未知协议：兼容保留
                            }
                            Err(e) if is_socket_timeout(&e) => continue,
                            Err(e) => {
                                emit_runtime_log(
                                    &app_handle,
                                    "error",
                                    format!("文件客户端读取失败: {}", e),
                                );
                                break;
                            }
                        }
                    }

                    if let Some(mut file) = current_file {
                        let _ = file.flush();
                    }
                    let _ = stream.shutdown(Shutdown::Both);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    emit_runtime_log(
                        &app_handle,
                        "warn",
                        format!("文件客户端接收连接错误: {}", e),
                    );
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }

        emit_runtime_log(&app_handle, "info", "文件客户端已停止");
        finish_session();
    });

    Ok(())
}

#[tauri::command]
pub fn start_file_server(
    app: AppHandle,
    ip: String,
    port: u16,
    pair_code: String,
    file_paths: Vec<String>,
) -> Result<(), String> {
    let pair_code = pair_code.trim().to_string();
    validate_pair_code_strength(&pair_code)?;
    let ip = ip.trim().to_string();
    ip.parse::<IpAddr>()
        .map_err(|e| format!("IP 地址解析失败: {}", e))?;

    if file_paths.is_empty() {
        return Err("请选择至少一个文件".to_string());
    }
    begin_session()?;

    emit_runtime_log(
        &app,
        "info",
        format!("文件服务端启动，目标 {}:{}", ip, port),
    );

    let is_running = Arc::clone(&IS_RUNNING);
    let app_handle = app.clone();
    thread::spawn(move || {
        while *is_running.lock().unwrap() {
            let ip_addr: IpAddr = match ip.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    emit_runtime_log(
                        &app_handle,
                        "warn",
                        format!("文件服务端 IP 解析失败: {}", e),
                    );
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            match TcpStream::connect_timeout(&(ip_addr, port).into(), Duration::from_secs(5)) {
                Ok(mut stream) => {
                    let _ = stream.set_nodelay(true);
                    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
                    let _ = stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT));

                    match authenticate_server_connection(&mut stream, &pair_code) {
                        Ok(_) => {
                            emit_runtime_log(&app_handle, "success", "文件服务端鉴权通过");
                        }
                        Err(e) => {
                            emit_runtime_log(
                                &app_handle,
                                "warn",
                                format!("文件服务端鉴权失败: {}", e),
                            );
                            let _ = stream.shutdown(Shutdown::Both);
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    }

                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

                    for path in file_paths.iter() {
                        if !*is_running.lock().unwrap() {
                            break;
                        }

                        let path = std::path::Path::new(path);
                        if !path.exists() || !path.is_file() {
                            emit_runtime_log(
                                &app_handle,
                                "warn",
                                format!("跳过无效文件: {}", path.display()),
                            );
                            continue;
                        }

                        let file_name = match path.file_name().and_then(|n| n.to_str()) {
                            Some(n) => n,
                            None => {
                                emit_runtime_log(
                                    &app_handle,
                                    "warn",
                                    format!("文件名无法解码: {}", path.display()),
                                );
                                continue;
                            }
                        };

                        let size = match path.metadata() {
                            Ok(meta) => meta.len(),
                            Err(e) => {
                                emit_runtime_log(
                                    &app_handle,
                                    "warn",
                                    format!("无法读取文件元数据: {}", e),
                                );
                                continue;
                            }
                        };

                        let _ = stream.write_all(encode_file_meta(file_name, size).as_bytes());
                        let _ = stream.flush();
                        emit_runtime_log(
                            &app_handle,
                            "info",
                            format!("开始发送文件: {} ({} bytes)", file_name, size),
                        );

                        match std::fs::File::open(path) {
                            Ok(mut file) => {
                                let mut buffer = [0_u8; 64 * 1024];
                                let mut file_checksum = 0_u64;
                                let mut completed = true;
                                loop {
                                    if !*is_running.lock().unwrap() {
                                        let _ = stream
                                            .write_all(format!("{}\n", FILE_ABORT).as_bytes());
                                        completed = false;
                                        break;
                                    }

                                    let read_bytes = match file.read(&mut buffer) {
                                        Ok(0) => break,
                                        Ok(n) => n,
                                        Err(e) => {
                                            emit_runtime_log(
                                                &app_handle,
                                                "error",
                                                format!("读取文件失败: {}", e),
                                            );
                                            let _ = stream
                                                .write_all(format!("{}\n", FILE_ABORT).as_bytes());
                                            completed = false;
                                            break;
                                        }
                                    };

                                    file_checksum =
                                        checksum_update(file_checksum, &buffer[..read_bytes]);
                                    let b64 = base64::engine::general_purpose::STANDARD
                                        .encode(&buffer[..read_bytes]);
                                    if let Err(e) =
                                        stream.write_all(encode_file_chunk(&b64).as_bytes())
                                    {
                                        emit_runtime_log(
                                            &app_handle,
                                            "error",
                                            format!("写入网络失败: {}", e),
                                        );
                                        completed = false;
                                        break;
                                    }
                                }

                                if completed {
                                    let _ = stream.write_all(
                                        encode_file_done(file_name, size, file_checksum).as_bytes(),
                                    );
                                    let _ = stream.flush();
                                    emit_runtime_log(
                                        &app_handle,
                                        "success",
                                        format!("文件发送完成: {}", file_name),
                                    );
                                }
                            }
                            Err(e) => {
                                emit_runtime_log(
                                    &app_handle,
                                    "error",
                                    format!("无法打开文件 {}: {}", path.display(), e),
                                );
                                let _ = stream.write_all(format!("{}\n", FILE_ABORT).as_bytes());
                            }
                        }
                    }

                    let _ = stream.shutdown(Shutdown::Both);
                    break;
                }
                Err(e) => {
                    emit_runtime_log(&app_handle, "warn", format!("文件服务端连接失败: {}", e));
                    if !*is_running.lock().unwrap() {
                        break;
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }

        emit_runtime_log(&app_handle, "info", "文件服务端退出");
        finish_session();
    });

    Ok(())
}

#[tauri::command]
pub fn start_mouse_server(
    app: AppHandle,
    ip: String,
    port: u16,
    pair_code: String,
) -> Result<(), String> {
    // 控制端角色：主动连接接收端并转发本机键鼠事件。
    let pair_code = pair_code.trim().to_string();
    validate_pair_code_strength(&pair_code)?;
    let ip = ip.trim().to_string();
    ip.parse::<IpAddr>()
        .map_err(|e| format!("IP 地址解析失败: {}", e))?;
    begin_session()?;

    emit_runtime_log(&app, "info", format!("控制端启动，目标 {}:{}", ip, port));

    let is_running = Arc::clone(&IS_RUNNING);
    let is_running_main = Arc::clone(&is_running);
    let app_handle = app.clone();
    thread::spawn(move || {
        let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
        let center_x = (screen_width / 2) as i32;
        let center_y = (screen_height / 2) as i32;

        let is_sharing = Arc::new(Mutex::new(false));
        let is_sharing_for_listener = Arc::clone(&is_sharing);
        let is_sharing_for_timer = Arc::clone(&is_sharing);

        // 事件队列，按 FIFO 顺序发送
        let event_queue = Arc::new(Mutex::new(VecDeque::new()));
        let event_queue_for_listener = Arc::clone(&event_queue);
        let event_queue_for_timer = Arc::clone(&event_queue);
        let event_queue_for_sender = Arc::clone(&event_queue);

        // 当前鼠标位置（用于定时器线程读取）
        let current_pos = Arc::new(Mutex::new((center_x, center_y)));
        let current_pos_for_listener = Arc::clone(&current_pos);
        let current_pos_for_timer = Arc::clone(&current_pos);

        // 定时器重置位置基准
        let reset_base = Arc::new(Mutex::new((center_x, center_y)));
        let reset_base_for_listener = Arc::clone(&reset_base);
        let reset_base_for_timer = Arc::clone(&reset_base);

        let is_running_for_listener = Arc::clone(&is_running);
        let is_running_for_timer = Arc::clone(&is_running);

        let (event_tx, event_rx) = channel();
        start_event_listener(event_tx);

        // 线程1：只负责监听鼠标移动事件，更新当前光标位置
        thread::spawn(move || {
            while let Ok(event) = event_rx.recv() {
                if !*is_running_for_listener.lock().unwrap() {
                    return;
                }

                // 未连接时忽略事件
                if !*IS_CONNECTED.lock().unwrap() {
                    continue;
                }

                // 判断是否进入共享
                if let AnyEvent::MouseEvent(MouseEvent {
                    kind: MouseEventKind::Move { x, y },
                }) = &event
                {
                    let mut sharing = is_sharing_for_listener.lock().unwrap();
                    if *x >= (screen_width as i32 - 1) && !*sharing {
                        println!("进入共享状态，隐藏光标并置于中心");
                        hide_cursor();
                        move_cursor_to(center_x, center_y);
                        *sharing = true;
                        // 重置基准
                        *reset_base_for_listener.lock().unwrap() = (center_x, center_y);
                        *current_pos_for_listener.lock().unwrap() = (center_x, center_y);
                        continue;
                    }
                    drop(sharing);

                    // 只更新当前位置
                    *current_pos_for_listener.lock().unwrap() = (*x, *y);
                } else {
                    // 按钮、键盘、滚轮事件直接发送到队列
                    event_queue_for_listener.lock().unwrap().push_back(event);
                }
            }
        });

        // 线程2：定时器，每隔固定时间重置鼠标到中心并发送位移
        thread::spawn(move || {
            loop {
                if !*is_running_for_timer.lock().unwrap() {
                    return;
                }

                let sharing = is_sharing_for_timer.lock().unwrap();
                if !*sharing {
                    drop(sharing);
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                drop(sharing);

                // 获取当前位置
                let (curr_x, curr_y) = *current_pos_for_timer.lock().unwrap();

                // 获取上次重置的基准位置
                let (base_x, base_y) = *reset_base_for_timer.lock().unwrap();

                // 计算相对位移
                let dx = curr_x - base_x;
                let dy = curr_y - base_y;

                if dx != 0 || dy != 0 {
                    // 发送位移到队列
                    event_queue_for_timer
                        .lock()
                        .unwrap()
                        .push_back(AnyEvent::MouseEvent(MouseEvent {
                            kind: MouseEventKind::MoveDelta { dx, dy },
                        }));
                }

                // 重置鼠标到中心
                move_cursor_to(center_x, center_y);

                // 更新基准位置
                *reset_base_for_timer.lock().unwrap() = (center_x, center_y);
                *current_pos_for_timer.lock().unwrap() = (center_x, center_y);

                // 固定间隔 1ms
                thread::sleep(Duration::from_millis(1));
            }
        });

        // 连接主循环：负责重连、握手、加密发送事件、接收 RELEASE。
        while *is_running_main.lock().unwrap() {
            let target_addr = format!("{}:{}", ip, port);
            println!("尝试连接到 {}", target_addr);

            let ip_addr: IpAddr = match ip.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    emit_runtime_log(
                        &app_handle,
                        "warn",
                        format!("IP 地址解析失败: {}, 请检查格式", ip),
                    );
                    println!("[控制端] IP 地址解析失败: {}, 错误: {}", ip, e);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };

            match TcpStream::connect_timeout(&(ip_addr, port).into(), Duration::from_secs(5)) {
                Ok(mut stream) => {
                    let _ = stream.set_nodelay(true);
                    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
                    let _ = stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT));

                    // 每次新连接都先完成握手。
                    match authenticate_server_connection(&mut stream, &pair_code) {
                        Ok(_) => {}
                        Err(e) => {
                            emit_runtime_log(&app_handle, "warn", format!("控制端鉴权失败: {}", e));
                            println!("[控制端] {}", e);
                            let _ = stream.shutdown(Shutdown::Both);
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                    };

                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

                    {
                        let mut connected = IS_CONNECTED.lock().unwrap();
                        *connected = true;
                    }
                    emit_runtime_log(&app_handle, "success", "控制端连接成功，鉴权通过");
                    println!("已连接到服务器，鉴权通过");

                    let is_sharing_recv = Arc::clone(&is_sharing);
                    let is_running_recv = Arc::clone(&is_running_main);
                    let reader_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(e) => {
                            println!("复制连接失败: {}", e);
                            let mut connected = IS_CONNECTED.lock().unwrap();
                            *connected = false;
                            continue;
                        }
                    };

                    // 读取线程：监听远端 RELEASE 以结束共享态。
                    thread::spawn(move || {
                        let _ = reader_stream.set_read_timeout(Some(Duration::from_millis(200)));
                        let mut buf_reader = BufReader::new(reader_stream);

                        loop {
                            if !*is_running_recv.lock().unwrap() {
                                break;
                            }

                            let mut buf = String::new();
                            match buf_reader.read_line(&mut buf) {
                                Ok(0) => {
                                    println!("客户端连接已关闭");
                                    break;
                                }
                                Ok(_) => {
                                    let payload = match decode_frame(&buf) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            println!("接收客户端消息失败: {}", e);
                                            break;
                                        }
                                    };

                                    if payload.trim() == "RELEASE" {
                                        let mut sharing = is_sharing_recv.lock().unwrap();
                                        *sharing = false;
                                        show_cursor();
                                        println!("收到客户端释放信号，恢复本机光标");
                                    }
                                }
                                Err(e) if is_socket_timeout(&e) => continue,
                                Err(e) => {
                                    println!("接收客户端消息出错: {}", e);
                                    break;
                                }
                            }
                        }
                    });

                    // 用于检测共享态从 true -> false 的边沿，触发光标恢复。
                    let mut last_sharing = false;
                    while *is_running_main.lock().unwrap() {
                        let sharing = *is_sharing.lock().unwrap();

                        if !sharing && last_sharing {
                            show_cursor();
                        }
                        last_sharing = sharing;

                        // 共享状态下发送队列中的所有事件
                        if sharing {
                            let mut event_queue = event_queue_for_sender.lock().unwrap();

                            // 发送队列中的所有事件（鼠标移动、按键、滚轮等）
                            let mut batch_data = Vec::new();
                            while let Some(evt) = event_queue.pop_front() {
                                let plain = serde_json::to_string(&evt).unwrap();
                                let frame = encode_frame(&plain);
                                batch_data.extend(frame.into_bytes());
                            }

                            if !batch_data.is_empty() {
                                if let Err(e) = stream.write_all(&batch_data) {
                                    println!("发送键鼠鼠标数据错误: {}", e);
                                    break;
                                }
                            }
                        }

                        // 减少 sleep 时间以降低延迟
                        thread::sleep(Duration::from_millis(1));
                    }

                    // 断开连接时恢复光标
                    show_cursor();
                    let frame = encode_frame("RELEASE");
                    let _ = stream.write_all(frame.as_bytes());
                    let _ = stream.flush();
                    let _ = stream.shutdown(Shutdown::Both);
                    {
                        let mut connected = IS_CONNECTED.lock().unwrap();
                        *connected = false;
                    }
                }
                Err(e) => {
                    // 连接失败时确保 is_connected 为 false
                    let mut connected = IS_CONNECTED.lock().unwrap();
                    *connected = false;
                    let err_msg = format!("连接失败: {}", e);
                    emit_runtime_log(&app_handle, "warn", &err_msg);
                    println!(
                        "{}，请检查：1. 目标 IP 是否正确 2. 防火墙是否阻止 3. 端口是否开放",
                        err_msg
                    );

                    if !*is_running_main.lock().unwrap() {
                        break;
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }

        // 线程退出时恢复光标
        finish_session();
        emit_runtime_log(&app_handle, "info", "控制端已停止");
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_code_requires_length_letters_and_digits() {
        assert!(validate_pair_code_strength("Share123").is_ok());
        assert!(validate_pair_code_strength("12345678").is_err());
        assert!(validate_pair_code_strength("abcdefgh").is_err());
        assert!(validate_pair_code_strength("a1").is_err());
    }

    #[test]
    fn data_frame_round_trip_preserves_payload() {
        let frame = encode_frame(r#"{"kind":"ping"}"#);
        assert_eq!(decode_frame(&frame).unwrap(), r#"{"kind":"ping"}"#);
        assert!(decode_frame("BROKEN payload").is_err());
    }

    #[test]
    fn file_frames_round_trip_and_checksum() {
        let bytes = b"hello shared file";
        let checksum = checksum_update(0, bytes);

        let meta = encode_file_meta("demo.txt", bytes.len() as u64);
        assert_eq!(
            decode_file_meta(&meta).unwrap(),
            ("demo.txt".to_string(), bytes.len() as u64)
        );

        let done = encode_file_done("demo.txt", bytes.len() as u64, checksum);
        assert_eq!(
            decode_file_done(&done).unwrap(),
            ("demo.txt".to_string(), bytes.len() as u64, checksum)
        );
    }

    #[test]
    fn unique_download_path_does_not_overwrite_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "kms-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("demo.txt");
        std::fs::write(&existing, b"existing").unwrap();

        let next = unique_download_path(&dir, "demo.txt");
        assert_eq!(next.file_name().unwrap().to_str().unwrap(), "demo (1).txt");

        let _ = std::fs::remove_file(existing);
        let _ = std::fs::remove_dir(dir);
    }
}
