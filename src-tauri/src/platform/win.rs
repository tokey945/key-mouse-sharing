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
#[cfg(target_os = "windows")]
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

        // 小键盘数字区
        "NumPad0" | "Numpad0" => Some(Key::Layout('0')),
        "NumPad1" | "Numpad1" => Some(Key::Layout('1')),
        "NumPad2" | "Numpad2" => Some(Key::Layout('2')),
        "NumPad3" | "Numpad3" => Some(Key::Layout('3')),
        "NumPad4" | "Numpad4" => Some(Key::Layout('4')),
        "NumPad5" | "Numpad5" => Some(Key::Layout('5')),
        "NumPad6" | "Numpad6" => Some(Key::Layout('6')),
        "NumPad7" | "Numpad7" => Some(Key::Layout('7')),
        "NumPad8" | "Numpad8" => Some(Key::Layout('8')),
        "NumPad9" | "Numpad9" => Some(Key::Layout('9')),
        "NumPadAdd" | "NumpadAdd" | "NumAdd" => Some(Key::Layout('+')),
        "NumPadSubtract" | "NumpadSubtract" | "NumSubtract" => Some(Key::Layout('-')),
        "NumPadMultiply" | "NumpadMultiply" | "NumMultiply" => Some(Key::Layout('*')),
        "NumPadDivide" | "NumpadDivide" | "NumDivide" => Some(Key::Layout('/')),
        "NumPadDecimal" | "NumpadDecimal" | "NumDecimal" => Some(Key::Layout('.')),
        "NumPadEnter" | "NumpadEnter" => Some(Key::Return),

        // Insert/Delete/Home/End/PageUp/PageDown
        "Insert" | "KeyInsert" => Some(Key::Insert),
        "Delete" | "KeyDelete" => Some(Key::Delete),
        "Home" | "KeyHome" => Some(Key::Home),
        "End" | "KeyEnd" => Some(Key::End),
        "PageUp" | "PageUpKey" | "KeyPageUp" => Some(Key::PageUp),
        "PageDown" | "PageDownKey" | "KeyPageDown" => Some(Key::PageDown),

        // PrintScreen/ScrollLock/Pause
        "PrintScreen" | "Print" | "KeyPrintScreen" => Some(Key::Layout('\u{2399}')), // 没有专用枚举，用符号
        "ScrollLock" | "KeyScrollLock" => Some(Key::Layout('\u{21E7}')),
        "Pause" | "Break" | "KeyPause" => Some(Key::Layout('\u{238B}')),

        // Menu/Apps
        "Menu" | "Apps" | "ContextMenu" => Some(Key::Layout('\u{2630}')),

        // Fn键（部分键盘有）
        "Fn" | "Function" => Some(Key::Layout('\u{1F5A5}')),

        _ => None,
    }
}
