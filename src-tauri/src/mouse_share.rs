use enigo::*;
use rdev::display_size;
use rdev::{listen, EventType};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct MousePos {
    x: i32,
    y: i32,
}

lazy_static::lazy_static! {
    static ref IS_RUNNING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[tauri::command]
pub fn stop_sharing() {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = false;
    println!("停止共享");
}

#[tauri::command]
pub fn start_mouse_client(port: u16) {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = true;
    drop(running);

    let is_running = Arc::clone(&IS_RUNNING);
    thread::spawn(move || {
        let listener = TcpListener::bind(("0.0.0.0", port)).unwrap();
        println!("客户端监听端口: {}", port);
        while *is_running.lock().unwrap() {
            match listener.accept() {
                Ok((stream, addr)) => {
                    println!("客户端已连接: {}", addr);
                    let mut enigo = Enigo::new();
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        match serde_json::from_str::<MousePos>(&line) {
                            Ok(pos) => enigo.mouse_move_to(pos.x, pos.y),
                            Err(e) => println!("解析数据错误: {}", e),
                        }
                        if !*is_running.lock().unwrap() {
                            break;
                        }
                    }
                }
                Err(e) => println!("接受连接错误: {}", e),
            }
        }
    });
}

#[tauri::command]
pub fn start_mouse_server(ip: String, port: u16) {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = true;
    drop(running);

    let is_running = Arc::clone(&IS_RUNNING);
    thread::spawn(move || {
        let position = Arc::new(Mutex::new((0, 0)));
        let pos_clone = Arc::clone(&position);

        let is_sharing = Arc::new(Mutex::new(false));
        let is_sharing_clone = Arc::clone(&is_sharing);

        // 鼠标监听线程，更新位置和共享状态
        thread::spawn(move || {
            let (screen_width, _) = display_size().unwrap_or((1920, 1080));
            let callback = move |event: rdev::Event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    let mut pos = pos_clone.lock().unwrap();
                    *pos = (x as i32, y as i32);

                    let mut sharing = is_sharing_clone.lock().unwrap();
                    let buffer = 10.0; // 缓冲区，防止边界抖动
                    if x >= screen_width as f64 {
                        *sharing = true;
                    } else if x < screen_width as f64 - buffer {
                        *sharing = false;
                    }
                }
            };
            listen(callback).unwrap();
        });

        // 主循环：负责连接、同步数据、切换光标
        while *is_running.lock().unwrap() {
            println!("尝试连接到 {}:{}", ip, port);
            let ip_addr: IpAddr = ip
                .parse()
                .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
            match TcpStream::connect_timeout(&(ip_addr, port).into(), Duration::from_secs(2)) {
                Ok(mut stream) => {
                    println!("已连接到服务器");
                    let mut last_sharing = false;
                    while *is_running.lock().unwrap() {
                        let sharing = *is_sharing.lock().unwrap();
                        // 只在状态切换时调用隐藏/显示光标
                        if sharing && !last_sharing {
                            #[cfg(target_os = "macos")]
                            mac_cursor::hide_cursor();
                            #[cfg(target_os = "windows")]
                            win_cursor::hide_cursor();
                        } else if !sharing && last_sharing {
                            #[cfg(target_os = "macos")]
                            mac_cursor::show_cursor();
                            #[cfg(target_os = "windows")]
                            win_cursor::show_cursor();
                        }
                        last_sharing = sharing;

                        if sharing {
                            let (x, y) = *position.lock().unwrap();
                            let msg = serde_json::to_string(&MousePos { x, y }).unwrap() + "\n";
                            if let Err(e) = stream.write_all(msg.as_bytes()) {
                                println!("发送数据错误: {}", e);
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                    // 断开连接时恢复光标
                    #[cfg(target_os = "macos")]
                    mac_cursor::show_cursor();
                    #[cfg(target_os = "windows")]
                    win_cursor::show_cursor();
                }
                Err(e) => {
                    println!("连接失败: {}, 2秒后重试", e);
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        // 线程退出时恢复光标
        #[cfg(target_os = "macos")]
        mac_cursor::show_cursor();
        #[cfg(target_os = "windows")]
        win_cursor::show_cursor();
    });
}

// macOS 光标控制
#[cfg(target_os = "macos")]
mod mac_cursor {
    use core_graphics::display::{CGDisplayHideCursor, CGDisplayShowCursor, CGMainDisplayID};

    pub fn hide_cursor() {
        unsafe {
            CGDisplayHideCursor(CGMainDisplayID());
        }
    }
    pub fn show_cursor() {
        unsafe {
            CGDisplayShowCursor(CGMainDisplayID());
        }
    }
}

// Windows 光标控制
#[cfg(target_os = "windows")]
mod win_cursor {
    use winapi::um::winuser::ShowCursor;

    pub fn hide_cursor() {
        unsafe {
            ShowCursor(0);
        }
    }
    pub fn show_cursor() {
        unsafe {
            ShowCursor(1);
        }
    }
}
