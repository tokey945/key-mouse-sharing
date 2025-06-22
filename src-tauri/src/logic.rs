use enigo::MouseControllable;
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
