#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "macos")]
pub use mac::{hide_cursor, move_cursor_to, show_cursor};
#[cfg(target_os = "windows")]
pub use win::{hide_cursor, move_cursor_to, show_cursor};
