use enigo::MouseButton;
use enigo::MouseControllable;
use enigo::{Enigo, KeyboardControllable};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

thread_local! {
    static ENIGO: RefCell<Enigo> = RefCell::new(Enigo::new());
}

fn with_enigo<F, R>(f: F) -> R
where
    F: FnOnce(&mut Enigo) -> R,
{
    ENIGO.with(|cell| f(&mut cell.borrow_mut()))
}

// 传输层统一事件模型：控制端采集后序列化发送，接收端反序列化执行。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AnyEvent {
    MouseEvent(MouseEvent),
    KeyEvent(KeyEvent),
}

// 鼠标事件定义（Move 使用绝对坐标，MoveDelta 使用相对位移）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MouseEventKind {
    MoveDelta { dx: i32, dy: i32 },
    Move { x: i32, y: i32 },
    ButtonDown { button: String },
    ButtonUp { button: String },
    Wheel { delta: i32 },
}

// 键盘事件定义（按下/抬起分离，避免按键状态错乱）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KeyEventKind {
    KeyDown { key: String },
    KeyUp { key: String },
}

// 光标可见性、键鼠模拟和输入监听都封装在此，屏蔽平台差异。
pub fn hide_cursor() {
    crate::platform::hide_cursor();
}
pub fn show_cursor() {
    crate::platform::show_cursor();
}
pub fn move_cursor_to(x: i32, y: i32) {
    with_enigo(|enigo| {
        enigo.mouse_move_to(x, y);
    });
}

pub fn simulate_button_down(button: &str) {
    with_enigo(|enigo| match button {
        "Left" | "Button1" => enigo.mouse_down(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_down(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_down(MouseButton::Middle),
        _ => {}
    });
}

pub fn simulate_button_up(button: &str) {
    with_enigo(|enigo| match button {
        "Left" | "Button1" => enigo.mouse_up(MouseButton::Left),
        "Right" | "Button2" => enigo.mouse_up(MouseButton::Right),
        "Middle" | "Button3" => enigo.mouse_up(MouseButton::Middle),
        _ => {}
    });
}

pub fn simulate_wheel(delta: i32) {
    with_enigo(|enigo| {
        enigo.mouse_scroll_y(delta);
    });
}

pub fn simulate_key_down(key: &str) {
    if let Some(k) = str_to_enigo_key(key) {
        with_enigo(|enigo| {
            enigo.key_down(k);
        });
    }
}

pub fn simulate_key_up(key: &str) {
    if let Some(k) = str_to_enigo_key(key) {
        with_enigo(|enigo| {
            enigo.key_up(k);
        });
    }
}

pub fn start_event_listener(tx: std::sync::mpsc::Sender<AnyEvent>) {
    // 实际监听由平台层实现（mac/win），这里仅做统一入口。
    crate::platform::start_event_listener(tx);
}

fn str_to_enigo_key(key: &str) -> Option<enigo::Key> {
    crate::platform::str_to_enigo_key(key)
}
