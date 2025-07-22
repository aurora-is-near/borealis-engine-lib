use chrono::{DateTime, Utc};
use std::{
    fs::File,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

// This module implements a log to file macros based on the following snippet:
//
// static LOG_FILE: LazyLock<Mutex<File>> = LazyLock::new(|| {
//     Mutex::new(File::create("log.txt").unwrap())
// });
//
// let mut file = LOG_FILE.lock().unwrap();
// writeln!(file, "Log message: {}", data).unwrap();
// file.flush().unwrap();

const LOG_FNAME_TEMPLATE: &str =
    "/Users/alexander/root/github_aurora/borealis-rs/engine_storage_logs/{}_engine.log";

pub static LOG_FILE: LazyLock<Mutex<File>> = LazyLock::new(|| {
    let now: DateTime<Utc> = Utc::now();
    let now_str = now.format("%Y-%m-%d_-_%H-%M-%S").to_string();
    let filename = LOG_FNAME_TEMPLATE.replace("{}", &now_str);
    Mutex::new(File::create(filename).expect("Failed to create log file"))
});

pub static DIFF_MISMATCH_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn diff_mismatch_increment() -> usize {
    DIFF_MISMATCH_COUNT.fetch_add(1, Ordering::Relaxed)
}

#[macro_export]
macro_rules! warn_and_log_to_file {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
        {
            let mut log_file_guard = $crate::log_file::LOG_FILE.lock().expect("Failed to lock log file");
            std::io::Write::write_fmt(&mut *log_file_guard, format_args!($($arg)*)).expect("Failed to write to log file");
            std::io::Write::write_all(&mut *log_file_guard, b"\n").expect("Failed to write newline to log file");
        }
    };
}

#[macro_export]
macro_rules! log_to_file {
    ($($arg:tt)*) => {
        {
            let mut log_file_guard = $crate::log_file::LOG_FILE.lock().expect("Failed to lock log file");
            std::io::Write::write_fmt(&mut *log_file_guard, format_args!($($arg)*)).expect("Failed to write to log file");
            std::io::Write::write_all(&mut *log_file_guard, b"\n").expect("Failed to write newline to log file");
        }
    };
}

pub use log_to_file;
pub use warn_and_log_to_file;
