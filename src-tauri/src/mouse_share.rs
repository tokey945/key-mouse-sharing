use crate::logic::{
    hide_cursor, move_cursor_to, show_cursor, simulate_button_down, simulate_button_up,
    simulate_key_down, simulate_key_up, simulate_wheel, start_event_listener, AnyEvent, KeyEvent,
    KeyEventKind, MouseEvent, MouseEventKind,
};
use rdev::display_size;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// 服务端或客户端是否启动运行
lazy_static::lazy_static! {
    static ref IS_RUNNING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref IS_CONNECTED: std::sync::Arc<std::sync::Mutex<bool>> = std::sync::Arc::new(std::sync::Mutex::new(false));
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

                    // 获取当前默认的鼠标初始位置
                    let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
                    let (mut cur_x, mut cur_y) =
                        ((screen_width / 2) as i32, (screen_height / 2) as i32);

                    // todo 确保客户端光标显示
                    println!("[客户端] 调用 show_cursor()");

                    let max_x = screen_width as i32 - 1;
                    let max_y = screen_height as i32 - 1;

                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        if line.trim() == "RELEASE" {
                            println!("[客户端] 收到 Release 信号，退出循环");
                            break;
                        }

                        match serde_json::from_str::<AnyEvent>(&line) {
                            Ok(AnyEvent::MouseEvent(evt)) => match evt.kind {
                                MouseEventKind::MoveDelta { dx, dy } => {
                                    cur_x = (cur_x + dx).clamp(0, max_x);
                                    cur_y = (cur_y + dy).clamp(0, max_y);
                                    println!(
                                        "[客户端] 收到 Move dx={}, dy={}，移动到 ({}, {})",
                                        dx, dy, cur_x, cur_y
                                    );
                                    move_cursor_to(cur_x, cur_y);
                                    // 到达左边缘时通知服务端释放
                                    if cur_x <= 1 {
                                        let _ = stream.write_all(b"RELEASE\n");
                                        println!("[客户端] 到达左边缘，已通知服务端释放控制权");
                                    }
                                }
                                MouseEventKind::ButtonDown { button } => {
                                    println!("[客户端] 收到 ButtonDown: {}", button);
                                    simulate_button_down(&button);
                                }
                                MouseEventKind::ButtonUp { button } => {
                                    println!("[客户端] 收到 ButtonUp: {}", button);
                                    simulate_button_up(&button);
                                }
                                MouseEventKind::Wheel { delta } => {
                                    println!("[客户端] 收到 Wheel: {}", delta);
                                    simulate_wheel(delta);
                                }
                                _ => {}
                            },
                            Ok(AnyEvent::KeyEvent(evt)) => match evt.kind {
                                KeyEventKind::KeyDown { key } => {
                                    println!("[客户端] 收到 KeyDown: {}", key);
                                    let res = std::panic::catch_unwind(|| simulate_key_down(&key));
                                    if let Err(e) = res {
                                        println!("simulate_key_down 崩溃: {:?}", e);
                                    }
                                }
                                KeyEventKind::KeyUp { key } => {
                                    println!("[客户端] 收到 KeyUp: {}", key);
                                    let res = std::panic::catch_unwind(|| simulate_key_up(&key));
                                    if let Err(e) = res {
                                        println!("simulate_key_up 崩溃: {:?}", e);
                                    }
                                }
                            },
                            Err(e) => println!("[客户端] 解析数据错误: {}", e),
                        }

                        if !*is_running.lock().unwrap() {
                            println!("[客户端] 共享已运行，退出循环");
                            break;
                        }
                    }
                    println!("[客户端] 连接断开或循环结束");
                    let _ = stream.write_all(b"RELEASE\n");
                    stream.flush().ok();
                }
                Err(e) => println!("[客户端] 接受连接错误: {}", e),
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
    let is_running_main = Arc::clone(&is_running);
    thread::spawn(move || {
        let (screen_width, screen_height) = display_size().unwrap_or((1920, 1080));
        let center_x = (screen_width / 2) as i32;
        let center_y = (screen_height / 2) as i32;

        let is_sharing = Arc::new(Mutex::new(false));
        let is_sharing_clone = Arc::clone(&is_sharing);

        // 用于线程间传递dx/dy
        let event_queue = Arc::new(Mutex::new(Vec::new()));
        let event_queue_clone = Arc::clone(&event_queue);

        let pending_move = Arc::new(Mutex::new(None));
        let pending_move_clone = Arc::clone(&pending_move);
        // 启动平台拖拽监听
        let (event_tx, event_rx) = channel();
        start_event_listener(event_tx);
        // 该线程用来监听本地键鼠事件，并判断是否进入共享状态，更新位置和共享状态
        thread::spawn(move || {
            let mut last_x = center_x;
            let mut last_y = center_y;
            while let Ok(event) = event_rx.recv() {
                if !*is_running.lock().unwrap() {
                    return;
                }
                // 只有已连接时才允许进入共享
                if !*IS_CONNECTED.lock().unwrap() {
                    return;
                }
                let mut sharing = is_sharing_clone.lock().unwrap();

                // 先判断是否进入共享条件

                if let AnyEvent::MouseEvent(MouseEvent {
                    kind: MouseEventKind::Move { x, .. },
                }) = &event
                {
                    // 只有未共享且到达右边缘时才进入共享
                    if *x >= (screen_width as i32 - 1) && !*sharing {
                        println!("进入共享状态，隐藏光标并置于中心");
                        hide_cursor();
                        move_cursor_to(center_x, center_y);
                        *sharing = true;
                        // 进入共享锁住本地的键盘鼠标事件（除了鼠标移动）
                        // std::thread::spawn(|| {
                        //     block_local_input();
                        // });
                    }
                };

                // 共享状态下，计算相对移动并重置鼠标
                if *sharing {
                    let mut event_queue = event_queue_clone.lock().unwrap();

                    match &event {
                        // 鼠标移动（绝对坐标）
                        AnyEvent::MouseEvent(MouseEvent {
                            kind: MouseEventKind::Move { x, y },
                        }) => {
                            let dx = *x - last_x;
                            let dy = *y - last_y;
                            println!("dx={}, dy={}", dx, dy);
                            if dx != 0 || dy != 0 {
                                last_x = *x;
                                last_y = *y;
                                *pending_move_clone.lock().unwrap() = Some((dx, dy));
                                move_cursor_to(center_x, center_y);
                            }
                        }
                        // 鼠标按下
                        AnyEvent::MouseEvent(MouseEvent {
                            kind: MouseEventKind::ButtonDown { button },
                        }) => {
                            event_queue.push(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::ButtonDown {
                                    button: button.clone(),
                                },
                            }));
                            println!("收到 ButtonDown: {}", button);
                        }
                        // 鼠标抬起
                        AnyEvent::MouseEvent(MouseEvent {
                            kind: MouseEventKind::ButtonUp { button },
                        }) => {
                            event_queue.push(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::ButtonUp {
                                    button: button.clone(),
                                },
                            }));
                            println!("收到 ButtonUp: {}", button);
                        }
                        // 鼠标滚轮
                        AnyEvent::MouseEvent(MouseEvent {
                            kind: MouseEventKind::Wheel { delta },
                        }) => {
                            event_queue.push(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::Wheel { delta: *delta },
                            }));
                            println!("收到 Wheel: {}", delta);
                        }
                        // 键盘按下
                        AnyEvent::KeyEvent(KeyEvent {
                            kind: KeyEventKind::KeyDown { key },
                        }) => {
                            event_queue.push(AnyEvent::KeyEvent(KeyEvent {
                                kind: KeyEventKind::KeyDown { key: key.clone() },
                            }));
                            println!("收到 KeyDown: {}", key);
                        }
                        // 键盘抬起
                        AnyEvent::KeyEvent(KeyEvent {
                            kind: KeyEventKind::KeyUp { key },
                        }) => {
                            event_queue.push(AnyEvent::KeyEvent(KeyEvent {
                                kind: KeyEventKind::KeyUp { key: key.clone() },
                            }));
                            println!("收到 KeyUp: {}", key);
                        }
                        _ => {}
                    }
                }
            }
        });

        // 主循环：负责连接、同步数据、切换光标
        while *is_running_main.lock().unwrap() {
            println!("尝试连接到 {}:{}", ip, port);
            let ip_addr: IpAddr = ip
                .parse()
                .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

            match TcpStream::connect_timeout(&(ip_addr, port).into(), Duration::from_secs(2)) {
                Ok(mut stream) => {
                    {
                        let mut connected = IS_CONNECTED.lock().unwrap();
                        *connected = true;
                    }
                    println!("已连接到服务器");

                    let is_sharing_recv = Arc::clone(&is_sharing);
                    let mut buf_reader = BufReader::new(stream.try_clone().unwrap());

                    // 该线程用来监听客户端释放信号
                    thread::spawn(move || loop {
                        let mut buf = String::new();
                        match buf_reader.read_line(&mut buf) {
                            Ok(n) => {
                                if n > 0 && buf.trim() == "RELEASE" {
                                    let mut sharing = is_sharing_recv.lock().unwrap();
                                    *sharing = false;
                                    show_cursor();
                                    println!("收到客户端释放信号，恢复本机光标");
                                    // unblock_local_input();
                                    println!("已恢复本地键盘和鼠标按键输入");
                                } else if n == 0 {
                                    println!("客户端连接已关闭");
                                    break;
                                }
                            }
                            Err(e) => {
                                println!("接收客户端消息出错: {}", e);
                                break;
                            }
                        }
                    });

                    // 服务端启动后，是否共享过
                    let mut last_sharing = false;

                    while *is_running_main.lock().unwrap() {
                        let sharing = *is_sharing.lock().unwrap();

                        if !sharing && last_sharing {
                            println!("调用 mac_cursor::show_cursor()");
                            show_cursor();
                            // unblock_local_input();
                        }
                        last_sharing = sharing;

                        // 共享状态下发送dx/dy
                        if sharing {
                            let mut event_queue = event_queue.lock().unwrap();

                            // 每 1~3ms 发送一次
                            if let Some((dx, dy)) = pending_move.lock().unwrap().take() {
                                event_queue.push(AnyEvent::MouseEvent(MouseEvent {
                                    kind: MouseEventKind::MoveDelta { dx, dy },
                                }));
                            }

                            // 发送队列中的所有事件
                            while let Some(evt) = event_queue.pop() {
                                println!("准备发送事件: {:?}", evt);
                                let msg = serde_json::to_string(&evt).unwrap() + "\n";
                                println!("发送键鼠鼠标数据: {}", msg);
                                if let Err(e) = stream.write_all(msg.as_bytes()) {
                                    println!("发送键鼠鼠标数据错误: {}", e);
                                    break;
                                }
                                stream.flush().ok();
                            }
                        }

                        thread::sleep(Duration::from_millis(10));
                    }
                    // 断开连接时恢复光标
                    show_cursor();
                    let _ = stream.write_all(b"RELEASE\n");
                    let _ = stream.flush();
                    {
                        let mut connected = IS_CONNECTED.lock().unwrap();
                        *connected = false;
                    }
                }
                Err(e) => {
                    // 连接失败时确保 is_connected 为 false
                    let mut connected = IS_CONNECTED.lock().unwrap();
                    *connected = false;
                    println!("连接失败: {}, 2秒后重试", e);
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        // 线程退出时恢复光标和本地键鼠事件
        show_cursor();
        // unblock_local_input();
    });
}
