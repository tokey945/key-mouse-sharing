#[cfg(target_os = "windows")]
use winapi::um::winuser::ShowCursor;

#[cfg(target_os = "windows")]
pub fn hide_cursor() {
    unsafe { ShowCursor(0); }
}

#[cfg(target_os = "windows")]
pub fn show_cursor() {
    unsafe { ShowCursor(1); }
}