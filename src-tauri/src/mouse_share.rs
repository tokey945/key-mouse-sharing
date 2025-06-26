use crate::logic::{hide_cursor, move_cursor_to, show_cursor, MouseDelta};
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
                    let reader = BufReader::new(stream.try_clone().unwrap());

                    // 获取当前鼠标初始位置
                    let (mut cur_x, mut cur_y) = get_current_mouse_position();

                    // 确保客户端光标显示
                    println!("[客户端] 调用 show_cursor()");
                    let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));

                    let max_x = screen_width as i32 - 1;
                    let max_y = screen_height as i32 - 1;

                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        match serde_json::from_str::<MouseDelta>(&line) {
                            Ok(delta) => {
                                cur_x = (cur_x + delta.dx).clamp(0, max_x);
                                cur_y = (cur_y + delta.dy).clamp(0, max_y);
                                println!(
                                    "[客户端] 收到 dx={}, dy={}，移动到 ({}, {})",
                                    delta.dx, delta.dy, cur_x, cur_y
                                );
                                show_cursor();
                                move_cursor_to(cur_x, cur_y);

                                // 新增：到达左边缘时通知服务端释放
                                if cur_x <= 1 {
                                    let _ = stream.write_all(b"RELEASE\n");
                                    println!("[客户端] 到达左边缘，已通知服务端释放控制权");
                                }
                            }
                            Err(e) => println!("[客户端] 解析数据错误: {}", e),
                        }
                        if !*is_running.lock().unwrap() {
                            println!("[客户端] 共享已停止，退出循环");
                            break;
                        }
                    }
                    println!("[客户端] 连接断开或循环结束");
                }
                Err(e) => println!("[客户端] 接受连接错误: {}", e),
            }
        }
    });
}

// 获取当前鼠标位置的辅助函数
fn get_current_mouse_position() -> (i32, i32) {
    let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
    let x = 2;
    let y = (screen_height / 2) as i32;
    (x, y)
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

        let is_sharing = Arc::new(Mutex::new(false));
        let is_sharing_clone = Arc::clone(&is_sharing);

        // 用于线程间传递dx/dy
        let delta = Arc::new(Mutex::new((0, 0)));
        let delta_clone = Arc::clone(&delta);

        // 鼠标监听线程，更新位置和共享状态
        thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    let mut sharing = is_sharing_clone.lock().unwrap();

                    // 只有未共享且到达右边缘时才进入共享
                    if x >= (screen_width as f64 - 1.0) && !*sharing {
                        println!("进入共享状态，隐藏光标并置于中心");
                        hide_cursor();
                        move_cursor_to(center_x, center_y);
                        *sharing = true;
                    }

                    // 共享状态下，计算相对移动并重置鼠标
                    if *sharing {
                        let dx = x as i32 - center_x;
                        let dy = y as i32 - center_y;
                        if dx != 0 || dy != 0 {
                            let mut d = delta_clone.lock().unwrap();
                            *d = (dx, dy);
                            move_cursor_to(center_x, center_y);
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

                    let is_sharing_recv = Arc::clone(&is_sharing);
                    let mut buf_reader = BufReader::new(stream.try_clone().unwrap());
                    thread::spawn(move || loop {
                        let mut buf = String::new();
                        match buf_reader.read_line(&mut buf) {
                            Ok(n) => {
                                if n > 0 && buf.trim() == "RELEASE" {
                                    let mut sharing = is_sharing_recv.lock().unwrap();
                                    *sharing = false;
                                    show_cursor();
                                    println!("收到客户端释放信号，恢复本机光标");
                                }
                            }
                            Err(e) => {
                                println!("接收客户端消息出错: {}", e);
                                break;
                            }
                        }
                    });

                    let mut last_sharing = false;

                    while *is_running.lock().unwrap() {
                        let sharing = *is_sharing.lock().unwrap();

                        // 只在进入共享时隐藏光标，收到RELEASE时恢复
                        if !sharing && last_sharing {
                            println!("调用 mac_cursor::show_cursor()");
                            show_cursor();
                        }
                        last_sharing = sharing;

                        println!("当前共享状态: {}", sharing);
                        // 共享状态下发送dx/dy
                        if sharing {
                            // 持续锁定光标在中心
                            move_cursor_to(center_x, center_y);

                            let (dx, dy) = {
                                let mut d = delta.lock().unwrap();
                                let v = *d;
                                *d = (0, 0); // 发送后清零
                                v
                            };
                            println!("x:{}, y:{}", &dx, &dy);

                            if dx != 0 || dy != 0 {
                                println!("[服务端] 发送 dx={}, dy={}", dx, dy); // 新增日志
                                let msg = format!("{{\"dx\":{},\"dy\":{}}}\n", dx, dy);
                                if let Err(e) = stream.write_all(msg.as_bytes()) {
                                    println!("发送数据错误: {}", e);
                                    break;
                                }
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
