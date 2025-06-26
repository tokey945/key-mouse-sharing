#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::kCFBooleanTrue;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;
#[cfg(target_os = "macos")]
use core_graphics::display::{CGDisplayHideCursor, CGDisplayShowCursor, CGMainDisplayID};
#[cfg(target_os = "macos")]
use std::sync::Once;
#[cfg(target_os = "macos")]
static INIT: Once = Once::new();

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn _CGSDefaultConnection() -> i32;
    fn CGSSetConnectionProperty(
        cid: i32,
        cid2: i32,
        key: *const std::ffi::c_void,
        value: *const std::ffi::c_void,
    ) -> i32;
}

#[cfg(target_os = "macos")]
fn set_sets_cursor_in_background() {
    unsafe {
        let conn = _CGSDefaultConnection();
        let key_cfstring = CFString::new("SetsCursorInBackground");
        let key_ptr = key_cfstring.as_concrete_TypeRef() as *const std::ffi::c_void;
        let value_ptr = kCFBooleanTrue as *const std::ffi::c_void;
        let result = CGSSetConnectionProperty(conn, conn, key_ptr, value_ptr);
        if result != 0 {
            eprintln!("CGSSetConnectionProperty 调用失败，返回值: {}", result);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn hide_cursor() {
    INIT.call_once(|| set_sets_cursor_in_background());
    unsafe {
        CGDisplayHideCursor(CGMainDisplayID());
    }
}

#[cfg(target_os = "macos")]
pub fn show_cursor() {
    INIT.call_once(|| set_sets_cursor_in_background());
    unsafe {
        CGDisplayShowCursor(CGMainDisplayID());
    }
}
