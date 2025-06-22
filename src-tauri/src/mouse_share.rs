use crate::logic::{hide_cursor, show_cursor, MousePos};
use enigo::*;
use rdev::display_size;
use rdev::{listen, EventType};
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
                Ok((mut stream, addr)) => {
                    println!("客户端已连接: {}", addr);
                    let mut enigo = Enigo::new();
                    let reader = BufReader::new(stream.try_clone().unwrap());

                    // 获取当前鼠标初始位置
                    let (mut cur_x, mut cur_y) = get_current_mouse_position();

                    // 确保客户端光标显示
                    show_cursor();

                    let (screen_width, _) = display_size().unwrap_or((1920, 1080));

                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        #[derive(serde::Deserialize)]
                        struct MouseDelta { dx: i32, dy: i32 }
                        match serde_json::from_str::<MouseDelta>(&line) {
                            Ok(delta) => {
                                cur_x += delta.dx;
                                cur_y += delta.dy;
                                enigo.mouse_move_to(cur_x, cur_y);

                                // 新增：到达左边缘时通知服务端释放
                                if cur_x <= 0 {
                                    let _ = stream.write_all(b"RELEASE\n");
                                    println!("到达左边缘，已通知服务端释放控制权");
                                }
                            }
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

// 获取当前鼠标位置的辅助函数
fn get_current_mouse_position() -> (i32, i32) {
    use rdev::listen;
    use std::sync::{Arc, Mutex};
    let pos = Arc::new(Mutex::new((0, 0)));
    let pos_clone = Arc::clone(&pos);

    // 只监听一次鼠标移动事件
    let _ = std::thread::spawn(move || {
        let cb = move |event: rdev::Event| {
            if let rdev::EventType::MouseMove { x, y } = event.event_type {
                let mut p = pos_clone.lock().unwrap();
                *p = (x as i32, y as i32);
            }
        };
        let _ = listen(cb);
    })
    .join();

    let p = pos.lock().unwrap();
    *p
}

#[tauri::command]
pub fn start_mouse_server(ip: String, port: u16) {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = true;
    drop(running);

    let is_running = Arc::clone(&IS_RUNNING);
    thread::spawn(move || {
        let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
        let center_x = (screen_width / 2) as i32;
        let center_y = (screen_height / 2) as i32;

        let position = Arc::new(Mutex::new((center_x, center_y)));
        let pos_clone = Arc::clone(&position);

        let is_sharing = Arc::new(Mutex::new(false));
        let is_sharing_clone = Arc::clone(&is_sharing);

        // 用于线程间传递dx/dy
        let delta = Arc::new(Mutex::new((0, 0)));
        let delta_clone = Arc::clone(&delta);

        // 鼠标监听线程，更新位置和共享状态
        thread::spawn(move || {
            let mut enigo = Enigo::new();

            let callback = move |event: rdev::Event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    let mut pos = pos_clone.lock().unwrap();
                    let mut sharing = is_sharing_clone.lock().unwrap();

                    // 判断是否到达右边缘
                    if x >= (screen_width as f64 - 1.0) {
                        if !*sharing {
                            println!("进入共享状态，隐藏光标并置于中心");
                            hide_cursor();
                            enigo.mouse_move_to(center_x, center_y);
                            *sharing = true;
                            *pos = (center_x, center_y);
                        }
                    } else if x < screen_width as f64 - 1.0 {
                        if *sharing {
                            println!("退出共享状态，恢复光标");
                            show_cursor();
                            *sharing = false;
                        }
                    }

                    // 共享状态下，计算相对移动并重置鼠标
                    if *sharing {
                        let dx = x as i32 - center_x;
                        let dy = y as i32 - center_y;
                        if dx != 0 || dy != 0 {
                            // 存储dx/dy，供主循环发送
                            let mut d = delta_clone.lock().unwrap();
                            *d = (dx, dy);
                            // 重置本机鼠标到中心
                            enigo.mouse_move_to(center_x, center_y);
                            *pos = (center_x, center_y);
                        }
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
                    let mut buf_reader = BufReader::new(stream.try_clone().unwrap());

                    while *is_running.lock().unwrap() {
                        let sharing = *is_sharing.lock().unwrap();
                        // 只在状态切换时调用隐藏/显示光标
                        if sharing && !last_sharing {
                            println!("调用 mac_cursor::hide_cursor()");
                            hide_cursor();
                        } else if !sharing && last_sharing {
                            println!("调用 mac_cursor::show_cursor()");
                            show_cursor();
                        }
                        last_sharing = sharing;

                        // 共享状态下发送dx/dy
                        if sharing {
                            let (dx, dy) = {
                                let mut d = delta.lock().unwrap();
                                let v = *d;
                                *d = (0, 0); // 发送后清零
                                v
                            };
                            if dx != 0 || dy != 0 {
                                let msg = format!("{{\"dx\":{},\"dy\":{}}}\n", dx, dy);
                                if let Err(e) = stream.write_all(msg.as_bytes()) {
                                    println!("发送数据错误: {}", e);
                                    break;
                                }
                            }
                        }

                        // 新增：检查客户端是否发来释放信号
                        let mut buf = String::new();
                        if let Ok(n) = buf_reader.read_line(&mut buf) {
                            if n > 0 && buf.trim() == "RELEASE" {
                                let mut sharing = is_sharing.lock().unwrap();
                                *sharing = false;
                                show_cursor();
                                println!("收到客户端释放信号，恢复本机光标");
                            }
                        }

                        thread::sleep(Duration::from_millis(10));
                    }
                    // 断开连接时恢复光标
                    show_cursor();
                }
                Err(e) => {
                    println!("连接失败: {}, 2秒后重试", e);
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        // 线程退出时恢复光标
        show_cursor();
    });
}
