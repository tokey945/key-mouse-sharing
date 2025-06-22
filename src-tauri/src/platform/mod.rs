
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "macos")]
pub use mac::{hide_cursor, show_cursor};
#[cfg(target_os = "windows")]
pub use win::hide_cursor;
