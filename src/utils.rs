use std::thread;

use chrono::Local;

use crate::{LogBuffer, NewInternalLog, INTERNAL_WRITER, LOGGER};

pub fn with_logger(log: String, log_type: LogType) {
    let mut log = convert_log(log_type, &log);
    println!("{log}");
    log.push('\n');
    let mut lock = INTERNAL_WRITER.lock().unwrap();
    let log = if let NewInternalLog::New(internal_log) = lock.clone() {
        format!("{internal_log}{log}")
    } else {
        log
    };
    *lock = NewInternalLog::New(log);
    drop(lock);
}

pub fn new_input_bufs(bufs: Vec<LogBuffer>) {
    let logger = LOGGER.get().expect("No logger initialized");
    logger.clear_buf_pool();
    logger.new_input_bufs(bufs);
}

pub enum LogType {
    Warn,
    Info,
    Err,
}

impl LogType {
    pub fn prefix(&self) -> String {
        let curr_thread = thread::current();
        let thread_name = curr_thread.name().unwrap_or("unamed");
        match self {
            LogType::Warn => format!("[thread:{thread_name}:WARN]:"),
            LogType::Info => format!("[thread:{thread_name}:INFO]:"),
            LogType::Err => format!("[thread:{thread_name}:ERR]:"),
        }
    }
}

pub fn log_time() -> String {
    let curr_time = Local::now();
    format!("[{}]:", curr_time.format("%m/%d/%y %H:%M:%S"))
}

pub fn convert_log(log_type: LogType, msg: &str) -> String {
    let prefix = log_type.prefix();
    let msg: String = msg
        .lines()
        .map(|line| format!("{}{} {}", log_time(), prefix, line))
        .collect();
    msg
}
