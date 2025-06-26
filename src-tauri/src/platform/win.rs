use std::ptr::null_mut;
use std::sync::Once;
use winapi::shared::windef::{HCURSOR, POINT};
use winapi::um::winuser::{
    GetCursorPos, LoadCursorW, SendInput, SetCursor, SetCursorPos, SetSystemCursor, ShowCursor,
    SystemParametersInfoW, IDC_ARROW, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_MOVE_NOCOALESCE, MOUSEINPUT, SPI_SETCURSORS,
};

static INIT: Once = Once::new();
static mut CURSOR_MANAGER: Option<CursorManager> = None;

struct CursorManager {
    original_cursor: Option<HCURSOR>,
}

impl CursorManager {
    fn new() -> Self {
        Self {
            original_cursor: None,
        }
    }

    fn ensure_visible(&mut self) {
        unsafe {
            // 创建鼠标移动事件
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };

            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            };

            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);

            // 设置系统光标
            let cursor = LoadCursorW(null_mut(), IDC_ARROW as *const u16);
            if !cursor.is_null() {
                SetCursor(cursor);
                if self.original_cursor.is_none() {
                    self.original_cursor = Some(cursor);
                }
            }
        }
    }
}

fn get_cursor_manager() -> &'static mut CursorManager {
    unsafe {
        if CURSOR_MANAGER.is_none() {
            INIT.call_once(|| {
                CURSOR_MANAGER = Some(CursorManager::new());
            });
        }
        CURSOR_MANAGER.as_mut().unwrap()
    }
}

#[cfg(target_os = "windows")]
pub fn hide_cursor() {
    unsafe {
        // 隐藏光标
        while ShowCursor(0) >= 0 {}
    }
}

#[cfg(target_os = "windows")]
pub fn show_cursor() {
    unsafe {
        get_cursor_manager().ensure_visible();

        // 恢复系统光标
        SystemParametersInfoW(SPI_SETCURSORS, 0, null_mut(), 0);

        // 确保光标显示
        while ShowCursor(1) < 0 {}
    }
}

#[cfg(target_os = "windows")]
pub fn move_cursor_to(dx: i32, dy: i32) {
    unsafe {
        // 获取当前鼠标位置
        let mut current_pos = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut current_pos) != 0 {
            // 计算新位置
            let new_x = current_pos.x + dx;
            let new_y = current_pos.y + dy;

            // 发送相对移动事件
            let mut input = INPUT {
                type_: INPUT_MOUSE,
                u: std::mem::zeroed(),
            };

            *input.u.mi_mut() = MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_MOVE_NOCOALESCE,
                time: 0,
                dwExtraInfo: 0,
            };

            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_mouse_position() -> (i32, i32) {
    unsafe {
        let mut pos = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pos) != 0 {
            (pos.x, pos.y)
        } else {
            (0, 0)
        }
    }
}
