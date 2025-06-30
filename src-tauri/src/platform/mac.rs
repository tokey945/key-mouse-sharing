#![allow(improper_ctypes_definitions)]

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

// 监听拖拽事件
#[cfg(target_os = "macos")]
pub fn start_drag_listener(tx: Sender<(f64, f64)>) {
    std::thread::spawn(move || {
        // 监听拖拽事件
        let event_types = vec![
            CGEventType::LeftMouseDragged,
            CGEventType::RightMouseDragged,
        ];

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            event_types,
            move |_, type_, event| {
                if (type_ as u32) == CGEventType::LeftMouseDragged as u32
                    || (type_ as u32) == CGEventType::RightMouseDragged as u32
                {
                    let loc = event.location();
                    if tx.send((loc.x, loc.y)).is_err() {
                        println!("发送拖拽事件失败，channel 已关闭");
                    }
                }
                CallbackResult::Keep
            },
        )
        .expect("Failed to create event tap");

        let mach_port_ref = tap.mach_port().as_concrete_TypeRef();
        let run_loop_source = unsafe {
            CFMachPortCreateRunLoopSource(
                kCFAllocatorDefault as *mut std::ffi::c_void,
                mach_port_ref as *mut std::ffi::c_void,
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

static mut TAP: Option<CFMachPortRef> = None;
static TAP_MUTEX: Mutex<()> = Mutex::new(());

/// 屏蔽本地键盘和鼠标按键输入（不影响鼠标移动）
pub fn block_local_input() {
    let _guard = TAP_MUTEX.lock().unwrap();
    unsafe {
        if let Some(_) = TAP {
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
        TAP = Some(tap);

        println!("已屏蔽本地键盘和鼠标按键输入（macOS）");

        // 💡 启动事件循环（必须）
        CFRunLoopRun();
    }
}

/// 恢复本地输入
pub fn unblock_local_input() {
    let _guard = TAP_MUTEX.lock().unwrap();
    unsafe {
        if let Some(tap) = TAP.take() {
            CGEventTapEnable(tap, false);
            CFRelease(tap as *const c_void);
            println!("已恢复本地键盘和鼠标按键输入（macOS）");
        }
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
