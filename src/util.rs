use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;

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

#[macro_export]
macro_rules! sprintln {
    ($($arg:tt)*) => {
        println!("[{} SERVER] {}", get_utc(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! cprintln {
    ($($arg:tt)*) => {
        println!("[{} CLIENT] {}", get_utc(), format_args!($($arg)*))
    };
}
