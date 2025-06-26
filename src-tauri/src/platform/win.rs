use std::ptr::null_mut;
use std::sync::Once;
use winapi::shared::windef::{HCURSOR, POINT};
use winapi::um::winuser::ShowCursor;
#[cfg(target_os = "windows")]
pub fn hide_cursor() {
    unsafe {
        // 多次调用，确保计数器为负，光标一定隐藏
        for _ in 0..10 {
            if ShowCursor(0) < 0 {
                break;
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn show_cursor() {
    unsafe {
        // 多次调用，确保计数器为正，光标一定显示
        for _ in 0..10 {
            if ShowCursor(1) >= 0 {
                break;
            }
        }
    }
}
