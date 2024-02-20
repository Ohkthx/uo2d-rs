use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;

#[macro_export]
macro_rules! sprintln {
    ($($arg:tt)*) => {
        println!("[{} SERVER] {}", $crate::util::get_utc(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! cprintln {
    ($($arg:tt)*) => {
        println!("[{} CLIENT] {}", $crate::util::get_utc(), format_args!($($arg)*))
    };
}

/// UTC ISO 8601 formatted string.
pub fn get_utc() -> String {
    let now = Utc::now();
    now.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Current system time as u64.
pub fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Taste the rainbow.
#[allow(dead_code)]
pub fn exec_rainbow((r, g, b): (u8, u8, u8), step: u8) -> (u8, u8, u8) {
    if r == 255 && g < 255 && b == 0 {
        return (255, g.saturating_add(step), 0);
    } else if g == 255 && r > 0 && b == 0 {
        return (r.saturating_sub(step), 255, 0);
    } else if g == 255 && b < 255 {
        return (0, 255, b.saturating_add(step));
    } else if b == 255 && g > 0 {
        return (0, g.saturating_sub(step), 255);
    } else if b == 255 && r < 255 {
        return (r.saturating_add(step), 0, 255);
    } else if r == 255 && b > 0 {
        return (255, 0, b.saturating_sub(step));
    } else if r == 255 && g == 0 && b == 0 {
        return (255, step.min(255), 0);
    }
    (255, 0, 0)
}
