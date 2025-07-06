use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "macos")]
pub use mac::{hide_cursor, show_cursor};
#[cfg(target_os = "windows")]
pub use win::{hide_cursor, show_cursor};

#[cfg(target_os = "macos")]
pub use crate::platform::mac::str_to_enigo_key;
#[cfg(target_os = "windows")]
pub use crate::platform::win::str_to_enigo_key;

#[cfg(target_os = "macos")]
pub use crate::platform::mac::{block_local_input, unblock_local_input};
#[cfg(target_os = "windows")]
pub use crate::platform::win::{block_local_input, unblock_local_input};

#[cfg(target_os = "macos")]
pub use crate::platform::mac::start_event_listener;
#[cfg(target_os = "windows")]
pub use crate::platform::win::start_event_listener;
