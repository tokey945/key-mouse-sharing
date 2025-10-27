use enigo::MouseButton;
use enigo::MouseControllable;
use enigo::{Enigo, KeyboardControllable};
use serde::{Deserialize, Serialize};
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

// 事件类型
#[derive(Serialize, Deserialize, Debug)]
pub enum AnyEvent {
    MouseEvent(MouseEvent),
    KeyEvent(KeyEvent),
}

// 鼠标事件
#[derive(Serialize, Deserialize, Debug)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum MouseEventKind {
    MoveDelta { dx: i32, dy: i32 },
    Move { x: i32, y: i32 },
    ButtonDown { button: String },
    ButtonUp { button: String },
    Wheel { delta: i32 },
}

// 键盘事件
#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum KeyEventKind {
    KeyDown { key: String },
    KeyUp { key: String },
}

// 统一接口
pub fn hide_cursor() {
    crate::platform::hide_cursor();
}
pub fn show_cursor() {
    crate::platform::show_cursor();
}
pub fn move_cursor_to(x: i32, y: i32) {
    let mut enigo = enigo::Enigo::new();
    enigo.mouse_move_to(x, y);
}

pub fn simulate_button_down(button: &str) {
    let mut enigo = enigo::Enigo::new();
    match button {
        "Left" | "Button1" => enigo.mouse_down(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_down(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_down(MouseButton::Middle),
        _ => {}
    }
}

pub fn simulate_button_up(button: &str) {
    let mut enigo = enigo::Enigo::new();
    match button {
        "Left" | "Button1" => enigo.mouse_up(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_up(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_up(MouseButton::Middle),
        _ => {}
    }
}

pub fn simulate_wheel(delta: i32) {
    let mut enigo = enigo::Enigo::new();
    enigo.mouse_scroll_y(delta);
}

pub fn simulate_key_down(key: &str) {
    if let Some(k) = str_to_enigo_key(key) {
        let mut enigo = Enigo::new();
        enigo.key_down(k);
    }
}

pub fn simulate_key_up(key: &str) {
    if let Some(k) = str_to_enigo_key(key) {
        let mut enigo = Enigo::new();
        enigo.key_up(k);
    }
}
pub fn block_local_input() {
    crate::platform::block_local_input();
}
pub fn unblock_local_input() {
    crate::platform::unblock_local_input();
}
pub fn start_event_listener(tx: std::sync::mpsc::Sender<AnyEvent>) {
    crate::platform::start_event_listener(tx);
}

fn str_to_enigo_key(key: &str) -> Option<enigo::Key> {
    crate::platform::str_to_enigo_key(key)
}
