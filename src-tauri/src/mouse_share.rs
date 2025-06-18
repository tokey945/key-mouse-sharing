use enigo::*;
use rdev::{listen, EventType};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, IpAddr, Ipv4Addr};
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
                Ok((mut stream, addr)) => {
                    println!("客户端已连接: {}", addr);
                    let mut buffer = [0; 128];
                    let mut enigo = Enigo::new();
                    while *is_running.lock().unwrap() {
                        match stream.read(&mut buffer) {
                            Ok(size) if size > 0 => {
                                match serde_json::from_slice::<MousePos>(&buffer[..size]) {
                                    Ok(pos) => enigo.mouse_move_to(pos.x, pos.y),
                                    Err(e) => println!("解析数据错误: {}", e),
                                }
                            }
                            _ => break,
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

        thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    let mut pos = pos_clone.lock().unwrap();
                    *pos = (x as i32, y as i32);
                }
            };
            if let Err(e) = listen(callback) {
                println!("监听鼠标事件错误: {:?}", e);
            }
        });

        while *is_running.lock().unwrap() {
            println!("尝试连接到 {}:{}", ip, port);
            let ip_addr: IpAddr = ip.parse().unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
            match TcpStream::connect_timeout(&(ip_addr, port).into(), Duration::from_secs(2)) {
                Ok(mut stream) => {
                    println!("已连接到服务器");
                    while *is_running.lock().unwrap() {
                        let (x, y) = *position.lock().unwrap();
                        let msg = serde_json::to_string(&MousePos { x, y }).unwrap();
                        if let Err(e) = stream.write_all(msg.as_bytes()) {
                            println!("发送数据错误: {}", e);
                            break;
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                }
                Err(e) => {
                    println!("连接失败: {}, 2秒后重试", e);
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    });
}
