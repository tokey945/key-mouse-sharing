#[cfg(target_os = "windows")]
use winapi::um::winuser::{SetSystemCursor, LoadImageW, OCR_NORMAL, SPI_SETCURSORS, SystemParametersInfoW, LR_LOADFROMFILE, IMAGE_CURSOR};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HCURSOR;
#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
pub fn hide_cursor() {
    unsafe {
        // 加载透明光标
        let path = wide_null("src/platform/blank.cur"); // 路径根据实际情况调整
        let hcursor: HCURSOR = LoadImageW(
            null_mut(),
            path.as_ptr(),
            IMAGE_CURSOR,
            0,
            0,
            LR_LOADFROMFILE,
        ) as HCURSOR;
        if !hcursor.is_null() {
            SetSystemCursor(hcursor, OCR_NORMAL);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn show_cursor() {
    unsafe {
        // 恢复系统默认光标
        SystemParametersInfoW(SPI_SETCURSORS, 0, null_mut(), 0);
    }
}