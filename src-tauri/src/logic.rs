use enigo::MouseControllable;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MousePos {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseDelta {
    pub dx: i32,
    pub dy: i32,
}

// 统一接口
pub fn hide_cursor() {
    crate::platform::hide_cursor();
}
pub fn show_cursor() {
    crate::platform::show_cursor();
}
pub fn start_drag_listener(tx: Sender<(f64, f64)>) {
    crate::platform::start_drag_listener(tx);
}
pub fn move_cursor_to(dx: i32, dy: i32) {
    let mut enigo = enigo::Enigo::new();
    enigo.mouse_move_to(dx, dy);
}

pub fn simulate_button_down(button: &str) {
    // 这里用 enigo 举例
    use enigo::{Enigo, MouseButton, MouseControllable};
    let mut enigo = Enigo::new();
    match button {
        "Left" | "Button1" => enigo.mouse_down(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_down(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_down(MouseButton::Middle),
        _ => {}
    }
}

pub fn simulate_button_up(button: &str) {
    use enigo::{Enigo, MouseButton, MouseControllable};
    let mut enigo = Enigo::new();
    match button {
        "Left" | "Button1" => enigo.mouse_up(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_up(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_up(MouseButton::Middle),
        _ => {}
    }
}

pub fn simulate_wheel(delta: i32) {
    use enigo::{Enigo, MouseControllable};
    let mut enigo = Enigo::new();
    enigo.mouse_scroll_y(delta);
}

pub fn simulate_key_down(key: &str) {
    use enigo::{Enigo, Key, KeyboardControllable};
    let mut enigo = Enigo::new();
    if let Some(k) = str_to_enigo_key(key) {
        enigo.key_down(k);
    }
}

pub fn simulate_key_up(key: &str) {
    use enigo::{Enigo, Key, KeyboardControllable};
    let mut enigo = Enigo::new();
    if let Some(k) = str_to_enigo_key(key) {
        enigo.key_up(k);
    }
}
pub fn block_local_input() {
    crate::platform::block_local_input();
}
pub fn unblock_local_input() {
    crate::platform::unblock_local_input();
}

fn str_to_enigo_key(key: &str) -> Option<enigo::Key> {
    crate::platform::str_to_enigo_key(key)
}
