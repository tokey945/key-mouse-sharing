#![allow(improper_ctypes_definitions)]

use crate::logic::{AnyEvent, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::kCFBooleanTrue;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;
#[cfg(target_os = "macos")]
use core_graphics::display::{CGDisplayHideCursor, CGDisplayShowCursor, CGMainDisplayID};
#[cfg(target_os = "macos")]
use once_cell::sync::Lazy;
#[cfg(target_os = "macos")]
use std::sync::Once;
#[cfg(target_os = "macos")]
static INIT: Once = Once::new();
#[cfg(target_os = "macos")]
use core_foundation::runloop::CFRunLoopSourceRef;
#[cfg(target_os = "macos")]
use core_foundation::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun,
};
#[cfg(target_os = "macos")]
use core_foundation_sys::base::kCFAllocatorDefault;
#[cfg(target_os = "macos")]
use core_graphics::event::CGEventType;
#[cfg(target_os = "macos")]
use core_graphics::event::CallbackResult;
#[cfg(target_os = "macos")]
use core_graphics::event::*;
#[cfg(target_os = "macos")]
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapOptions};
#[cfg(target_os = "macos")]
use std::os::raw::{c_int, c_void};
#[cfg(target_os = "macos")]
use std::ptr;
#[cfg(target_os = "macos")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "macos")]
use std::sync::Mutex;
#[allow(non_camel_case_types)]
enum __CGEvent {}

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

type CGEventTapProxy = *mut c_void;
type CFMachPortRef = *mut c_void;
type CGEventMask = u64;
type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    type_: CGEventType,
    event: *mut c_void,
    refcon: *mut c_void,
) -> *mut c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        eventsOfInterest: CGEventMask,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: CFMachPortRef,
        order: c_int,
    ) -> CFRunLoopSourceRef;

    fn CFRelease(cf: *const c_void);
}

const KCG_HID_EVENT_TAP: u32 = 0;
const KCG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const KCG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

fn cg_event_mask_bit(event_type: CGEventType) -> CGEventMask {
    1u64 << (event_type as u32)
}

static TAP_HANDLE: Mutex<Option<usize>> = Mutex::new(None);
#[cfg(target_os = "macos")]
static EVENT_LISTENER_INIT: Once = Once::new();
#[cfg(target_os = "macos")]
static EVENT_SUBSCRIBERS: Lazy<Mutex<Vec<Sender<AnyEvent>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[cfg(target_os = "macos")]
fn publish_event(event: AnyEvent) {
    let mut subscribers = EVENT_SUBSCRIBERS.lock().unwrap();
    subscribers.retain(|tx| tx.send(event.clone()).is_ok());
}

/// 屏蔽本地键盘和鼠标按键输入（不影响鼠标移动）
pub fn block_local_input() {
    let mut tap_guard = TAP_HANDLE.lock().unwrap();
    unsafe {
        if tap_guard.is_some() {
            return;
        }

        extern "C" fn tap_callback(
            _proxy: CGEventTapProxy,
            type_: CGEventType,
            _event: *mut c_void,
            _user_info: *mut c_void,
        ) -> *mut c_void {
            match type_ {
                CGEventType::KeyDown
                | CGEventType::KeyUp
                | CGEventType::FlagsChanged
                | CGEventType::LeftMouseDown
                | CGEventType::LeftMouseUp
                | CGEventType::RightMouseDown
                | CGEventType::RightMouseUp
                | CGEventType::OtherMouseDown
                | CGEventType::OtherMouseUp => std::ptr::null_mut(),
                _ => _event,
            }
        }

        let event_mask = cg_event_mask_bit(CGEventType::KeyDown)
            | cg_event_mask_bit(CGEventType::KeyUp)
            | cg_event_mask_bit(CGEventType::FlagsChanged)
            | cg_event_mask_bit(CGEventType::LeftMouseDown)
            | cg_event_mask_bit(CGEventType::LeftMouseUp)
            | cg_event_mask_bit(CGEventType::RightMouseDown)
            | cg_event_mask_bit(CGEventType::RightMouseUp)
            | cg_event_mask_bit(CGEventType::OtherMouseDown)
            | cg_event_mask_bit(CGEventType::OtherMouseUp);

        let tap = CGEventTapCreate(
            KCG_HID_EVENT_TAP,
            KCG_HEAD_INSERT_EVENT_TAP,
            KCG_EVENT_TAP_OPTION_DEFAULT,
            event_mask,
            tap_callback,
            ptr::null_mut(),
        );
        if tap.is_null() {
            eprintln!("创建事件钩子失败，可能没有辅助功能权限");
            return;
        }

        let run_loop_source = CFMachPortCreateRunLoopSource(ptr::null_mut(), tap, 0);
        CFRunLoopAddSource(
            CFRunLoopGetCurrent(),
            run_loop_source,
            kCFRunLoopCommonModes,
        );

        CGEventTapEnable(tap, true);
        *tap_guard = Some(tap as usize);
        drop(tap_guard);

        println!("已屏蔽本地键盘和鼠标按键输入（macOS）");

        // 💡 启动事件循环（必须）
        CFRunLoopRun();
    }
}

/// 恢复本地输入
pub fn unblock_local_input() {
    let mut tap_guard = TAP_HANDLE.lock().unwrap();
    if let Some(raw_tap) = tap_guard.take() {
        let tap = raw_tap as CFMachPortRef;
        unsafe {
            CGEventTapEnable(tap, false);
            CFRelease(tap as *const c_void);
        }
        println!("已恢复本地键盘和鼠标按键输入（macOS）");
    }
}

#[cfg(target_os = "macos")]
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
        "Meta" | "MetaLeft" | "MetaRight" | "Command" | "Cmd" | "Win" | "KeyMeta" => {
            Some(Key::Meta)
        }

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

pub fn start_event_listener(tx: Sender<AnyEvent>) {
    EVENT_SUBSCRIBERS.lock().unwrap().push(tx);

    EVENT_LISTENER_INIT.call_once(|| {
        std::thread::spawn(move || {
            let event_types = vec![
                CGEventType::MouseMoved,
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
                CGEventType::RightMouseDown,
                CGEventType::RightMouseUp,
                CGEventType::OtherMouseDown,
                CGEventType::OtherMouseUp,
                CGEventType::LeftMouseDragged,
                CGEventType::RightMouseDragged,
                CGEventType::ScrollWheel,
                CGEventType::KeyDown,
                CGEventType::KeyUp,
            ];

            let tap = CGEventTap::new(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                event_types,
                move |_, type_, event| {
                    match type_ {
                        CGEventType::MouseMoved => {
                            let loc = event.location();
                            publish_event(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::Move {
                                    x: loc.x as i32,
                                    y: loc.y as i32,
                                },
                            }));
                        }
                        CGEventType::LeftMouseDown
                        | CGEventType::RightMouseDown
                        | CGEventType::OtherMouseDown => {
                            let btn_num = event
                                .get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                            let btn = mouse_to_string(btn_num);
                            publish_event(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::ButtonDown {
                                    button: btn.to_string(),
                                },
                            }));
                        }
                        CGEventType::LeftMouseUp
                        | CGEventType::RightMouseUp
                        | CGEventType::OtherMouseUp => {
                            let btn_num = event
                                .get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                            let btn = mouse_to_string(btn_num);
                            publish_event(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::ButtonUp {
                                    button: btn.to_string(),
                                },
                            }));
                        }
                        CGEventType::ScrollWheel => {
                            let delta = event.get_integer_value_field(
                                EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
                            ) as i32;
                            publish_event(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::Wheel { delta },
                            }));
                        }
                        CGEventType::KeyDown => {
                            let keycode =
                                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                            let key_str = keycode_to_string(keycode);
                            let key = if key_str == "Unknown" {
                                format!("Unknown({})", keycode)
                            } else {
                                key_str.to_string()
                            };

                            publish_event(AnyEvent::KeyEvent(KeyEvent {
                                kind: KeyEventKind::KeyDown { key },
                            }));
                        }
                        CGEventType::KeyUp => {
                            let keycode =
                                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                            let key_str = keycode_to_string(keycode);
                            let key = if key_str == "Unknown" {
                                format!("Unknown({})", keycode)
                            } else {
                                key_str.to_string()
                            };

                            publish_event(AnyEvent::KeyEvent(KeyEvent {
                                kind: KeyEventKind::KeyUp { key },
                            }));
                        }
                        CGEventType::LeftMouseDragged | CGEventType::RightMouseDragged => {
                            let loc = event.location();
                            publish_event(AnyEvent::MouseEvent(MouseEvent {
                                kind: MouseEventKind::Move {
                                    x: loc.x as i32,
                                    y: loc.y as i32,
                                },
                            }));
                        }
                        _ => {}
                    }
                    CallbackResult::Keep
                },
            )
            .expect("Failed to create event tap");

            let mach_port_ref = tap.mach_port().as_concrete_TypeRef();
            let run_loop_source = unsafe {
                core_foundation_sys::mach_port::CFMachPortCreateRunLoopSource(
                    kCFAllocatorDefault,
                    mach_port_ref,
                    0,
                )
            };

            unsafe {
                CFRunLoopAddSource(
                    CFRunLoopGetCurrent(),
                    run_loop_source,
                    kCFRunLoopCommonModes,
                );
                CFRunLoopRun();
            }
        });
    });
}

pub fn keycode_to_string(keycode: i64) -> &'static str {
    match keycode {
        // 字母
        0 => "KeyA",
        11 => "KeyB",
        8 => "KeyC",
        2 => "KeyD",
        14 => "KeyE",
        3 => "KeyF",
        5 => "KeyG",
        4 => "KeyH",
        34 => "KeyI",
        38 => "KeyJ",
        40 => "KeyK",
        37 => "KeyL",
        46 => "KeyM",
        45 => "KeyN",
        31 => "KeyO",
        35 => "KeyP",
        12 => "KeyQ",
        15 => "KeyR",
        1 => "KeyS",
        17 => "KeyT",
        32 => "KeyU",
        9 => "KeyV",
        13 => "KeyW",
        7 => "KeyX",
        16 => "KeyY",
        6 => "KeyZ",

        // 数字
        29 => "Key0",
        18 => "Key1",
        19 => "Key2",
        20 => "Key3",
        21 => "Key4",
        23 => "Key5",
        22 => "Key6",
        26 => "Key7",
        28 => "Key8",
        25 => "Key9",

        // 功能键
        36 => "Enter", // Return
        49 => "Space",
        48 => "Tab",
        53 => "Escape",
        51 => "Backspace",
        57 => "CapsLock",

        // 修饰键
        56 => "ShiftLeft",
        60 => "ShiftRight",
        59 => "ControlLeft",
        62 => "ControlRight",
        58 => "AltLeft",
        61 => "AltRight",
        55 => "MetaLeft",
        54 => "MetaRight",

        // 方向键
        123 => "ArrowLeft",
        124 => "ArrowRight",
        125 => "ArrowDown",
        126 => "ArrowUp",

        // F区
        122 => "F1",
        120 => "F2",
        99 => "F3",
        118 => "F4",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        100 => "F8",
        101 => "F9",
        109 => "F10",
        103 => "F11",
        111 => "F12",

        // 其它常用符号
        27 => "Minus",        // -
        24 => "Equal",        // =
        33 => "LeftBracket",  // [
        30 => "RightBracket", // ]
        42 => "Backslash",    // \
        41 => "Semicolon",    // ;
        39 => "Quote",        // '
        43 => "Comma",        // ,
        47 => "Period",       // .
        44 => "Slash",        // /

        // 小键盘（可选补充）
        71 => "NumLock", // Clear
        81 => "NumpadEqual",
        67 => "NumpadMultiply",
        69 => "NumpadAdd",
        78 => "NumpadSubtract",
        75 => "NumpadDivide",
        65 => "NumpadDecimal",
        76 => "NumpadEnter",
        82 => "Numpad0",
        83 => "Numpad1",
        84 => "Numpad2",
        85 => "Numpad3",
        86 => "Numpad4",
        87 => "Numpad5",
        88 => "Numpad6",
        89 => "Numpad7",
        91 => "Numpad8",
        92 => "Numpad9",

        // 其它
        114 => "Insert",
        115 => "Home",
        119 => "End",
        116 => "PageUp",
        121 => "PageDown",
        117 => "Delete",

        // 你可以根据需要继续补充
        _ => "Unknown",
    }
}

/// macOS MOUSE_EVENT_BUTTON_NUMBER 映射
pub fn mouse_to_string(btn: i64) -> &'static str {
    match btn {
        0 => "Left",    // 左键
        1 => "Right",   // 右键
        2 => "Middle",  // 中键
        3 => "Button4", // 侧键1
        4 => "Button5", // 侧键2
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enigo::Key;

    #[test]
    fn mac_keycode_zero_maps_to_key_a() {
        assert_eq!(keycode_to_string(0), "KeyA");
        assert!(matches!(str_to_enigo_key("KeyA"), Some(Key::Layout('a'))));
    }

    #[test]
    fn mac_common_modifiers_and_arrows_are_supported() {
        assert!(matches!(str_to_enigo_key("ShiftLeft"), Some(Key::Shift)));
        assert!(matches!(
            str_to_enigo_key("ControlRight"),
            Some(Key::Control)
        ));
        assert!(matches!(str_to_enigo_key("MetaLeft"), Some(Key::Meta)));
        assert!(matches!(
            str_to_enigo_key("ArrowLeft"),
            Some(Key::LeftArrow)
        ));
    }

    #[test]
    fn mac_unknown_keycode_is_explicit() {
        assert_eq!(keycode_to_string(-1), "Unknown");
        assert!(str_to_enigo_key("Unknown(-1)").is_none());
    }
}
