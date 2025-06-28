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
pub fn str_to_enigo_key(key: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match key {
        // 字母
        "KeyA" | "A" | "a" => Some(Key::Layout('a')),
        "KeyB" | "B" | "b" => Some(Key::Layout('b')),
        "KeyC" | "C" | "c" => Some(Key::Layout('c')),
        "KeyD" | "D" | "d" => Some(Key::Layout('d')),
        "KeyE" | "E" | "e" => Some(Key::Layout('e')),
        "KeyF" | "F" | "f" => Some(Key::Layout('f')),
        "KeyG" | "G" | "g" => Some(Key::Layout('g')),
        "KeyH" | "H" | "h" => Some(Key::Layout('h')),
        "KeyI" | "I" | "i" => Some(Key::Layout('i')),
        "KeyJ" | "J" | "j" => Some(Key::Layout('j')),
        "KeyK" | "K" | "k" => Some(Key::Layout('k')),
        "KeyL" | "L" | "l" => Some(Key::Layout('l')),
        "KeyM" | "M" | "m" => Some(Key::Layout('m')),
        "KeyN" | "N" | "n" => Some(Key::Layout('n')),
        "KeyO" | "O" | "o" => Some(Key::Layout('o')),
        "KeyP" | "P" | "p" => Some(Key::Layout('p')),
        "KeyQ" | "Q" | "q" => Some(Key::Layout('q')),
        "KeyR" | "R" | "r" => Some(Key::Layout('r')),
        "KeyS" | "S" | "s" => Some(Key::Layout('s')),
        "KeyT" | "T" | "t" => Some(Key::Layout('t')),
        "KeyU" | "U" | "u" => Some(Key::Layout('u')),
        "KeyV" | "V" | "v" => Some(Key::Layout('v')),
        "KeyW" | "W" | "w" => Some(Key::Layout('w')),
        "KeyX" | "X" | "x" => Some(Key::Layout('x')),
        "KeyY" | "Y" | "y" => Some(Key::Layout('y')),
        "KeyZ" | "Z" | "z" => Some(Key::Layout('z')),

        // 数字
        "Key0" | "0" => Some(Key::Layout('0')),
        "Key1" | "1" => Some(Key::Layout('1')),
        "Key2" | "2" => Some(Key::Layout('2')),
        "Key3" | "3" => Some(Key::Layout('3')),
        "Key4" | "4" => Some(Key::Layout('4')),
        "Key5" | "5" => Some(Key::Layout('5')),
        "Key6" | "6" => Some(Key::Layout('6')),
        "Key7" | "7" => Some(Key::Layout('7')),
        "Key8" | "8" => Some(Key::Layout('8')),
        "Key9" | "9" => Some(Key::Layout('9')),

        // 功能键
        "Enter" | "Return" | "KeyReturn" => Some(Key::Return),
        "Space" | "Spacebar" | "KeySpace" => Some(Key::Space),
        "Tab" | "KeyTab" => Some(Key::Tab),
        "Escape" | "Esc" | "KeyEscape" => Some(Key::Escape),
        "Backspace" | "KeyBackspace" => Some(Key::Backspace),
        "CapsLock" | "KeyCapsLock" => Some(Key::CapsLock),

        // 修饰键
        // Shift 键（左/右/通用）
        "Shift" | "ShiftLeft" | "ShiftRight" => Some(Key::Shift),
        // Control 键（左/右/通用/Ctrl别名）
        "Control" | "ControlLeft" | "ControlRight" | "KeyControl" | "Ctrl" => Some(Key::Control),
        // Alt/Option 键（左/右/通用/Option别名）
        "Alt" | "AltLeft" | "AltRight" | "KeyAlt" | "Option" => Some(Key::Alt),
        // Win键、Mac Command键、Meta键统一映射为 Meta
        "Meta" | "MetaLeft" | "MetaRight" | "Command" | "Cmd" | "Win" | "KeyMeta" => Some(Key::Meta),

        // 方向键
        "ArrowUp" | "Up" | "KeyUp" => Some(Key::UpArrow),
        "ArrowDown" | "Down" | "KeyDown" => Some(Key::DownArrow),
        "ArrowLeft" | "Left" | "KeyLeft" => Some(Key::LeftArrow),
        "ArrowRight" | "Right" | "KeyRight" => Some(Key::RightArrow),

        // F区
        "F1" | "KeyF1" => Some(Key::F1),
        "F2" | "KeyF2" => Some(Key::F2),
        "F3" | "KeyF3" => Some(Key::F3),
        "F4" | "KeyF4" => Some(Key::F4),
        "F5" | "KeyF5" => Some(Key::F5),
        "F6" | "KeyF6" => Some(Key::F6),
        "F7" | "KeyF7" => Some(Key::F7),
        "F8" | "KeyF8" => Some(Key::F8),
        "F9" | "KeyF9" => Some(Key::F9),
        "F10" | "KeyF10" => Some(Key::F10),
        "F11" | "KeyF11" => Some(Key::F11),
        "F12" | "KeyF12" => Some(Key::F12),

        // 其它常用符号
        "Minus" | "-" => Some(Key::Layout('-')),
        "Equal" | "=" => Some(Key::Layout('=')),
        "LeftBracket" | "[" => Some(Key::Layout('[')),
        "RightBracket" | "]" => Some(Key::Layout(']')),
        "Backslash" | "\\" => Some(Key::Layout('\\')),
        "Semicolon" | ";" => Some(Key::Layout(';')),
        "Quote" | "'" => Some(Key::Layout('\'')),
        "Comma" | "," => Some(Key::Layout(',')),
        "Period" | "." => Some(Key::Layout('.')),
        "Slash" | "/" => Some(Key::Layout('/')),

        _ => None,
    }
}
