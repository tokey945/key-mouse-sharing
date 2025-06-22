use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MousePos {
    pub x: i32,
    pub y: i32,
}

// 统一接口
pub fn hide_cursor() {
    crate::platform::hide_cursor();
}
pub fn show_cursor() {
    crate::platform::show_cursor();
}
