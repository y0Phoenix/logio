pub mod file;
pub mod macros;

use std::{
    io::{BufRead, BufReader, Read},
    sync::{Arc, Mutex, OnceLock},
    thread::{self, JoinHandle},
};

use chrono::Local;

use crate::file::LogioFile;

static LOGGER: OnceLock<Logger> = OnceLock::new();
static INTERNAL_WRITER: SharedLogBuf = Mutex::new(NewInternalLog::None);
static KILLED: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Default)]
pub enum NewInternalLog {
    New(String),
    #[default]
    None,
}

impl NewInternalLog {
    pub fn is_new(&self) -> bool {
        match self {
            NewInternalLog::New(_) => true,
            NewInternalLog::None => false,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::None
    }
}

type SharedLogBuf = Mutex<NewInternalLog>;
pub type LogBuffer = Box<dyn Read + Send + 'static>;
pub type LogErr = String;

pub struct Logger {
    file: Option<Arc<Mutex<LogioFile>>>,
    logger_thread: Option<JoinHandle<()>>,
    input_buf_pool: Arc<Mutex<Vec<BufReader<LogBuffer>>>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            file: Some(Arc::new(Mutex::new(LogioFile::default()))),
            logger_thread: None,
            input_buf_pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn input_buf(self, buffer: LogBuffer) -> Self {
        let mut pool = self.input_buf_pool.lock().unwrap();
        pool.push(BufReader::new(buffer));
        drop(pool);
        self
    }

    pub fn input_bufs(&self, mut bufs: Vec<BufReader<LogBuffer>>) {
        let mut pool = self.input_buf_pool.lock().unwrap();
        pool.append(&mut bufs);
        drop(pool);
    }

    fn clear_buf_pool(&self) {
        *self.input_buf_pool.lock().unwrap() = Vec::new();
    }

    pub fn log_file(mut self, file: LogioFile) -> Self {
        self.file = Some(Arc::new(Mutex::new(file)));
        self
    }

    pub fn run(mut self) -> Self {
        let file = Arc::clone(
            self.file
                .as_mut()
                .expect("No log file initialized yet. You must init a log file with log_file()"),
        );

        let input_pool = Arc::clone(&self.input_buf_pool);

        self.logger_thread = Some(
            thread::Builder::new()
                .name("logger_thread".to_string())
                .spawn(move || {
                    info!("logger thread started");
                    // so we can go through the loop one more time after kill send
                    let mut killed = false;
                    loop {
                        let mut file_lock = file.lock().unwrap();
                        //println!("loop1");
                        let mut pool = input_pool.lock().unwrap();
                        //println!("loop2");
                        for buffer in pool.iter_mut() {
                            let mut text = String::new();
                            match buffer.read_line(&mut text) {
                                Ok(n) if n > 0 => {
                                    //let text = String::from_utf8_lossy(&buf[..n]);
                                    print!("{text}");
                                    LogioFile::write_all(&mut file_lock, text.as_bytes());
                                    LogioFile::flush(&mut file_lock);
                                }
                                _ => {} // No data or EOF
                            }
                            //println!("loop3");
                        }

                        let mut lock = INTERNAL_WRITER.lock().unwrap();
                        if let NewInternalLog::New(log) = lock.clone() {
                            lock.reset();
                            LogioFile::write_all(&mut file_lock, log.as_bytes());
                            LogioFile::flush(&mut file_lock);
                        }
                        // for some reason rust will not drop the lock variable from memory after
                        // the loop block so need to do it manually
                        drop(lock);
                        drop(file_lock);
                        drop(pool);

                        if *KILLED.lock().unwrap() {
                            killed = false;
                            info!("kill recieved");
                        } else if killed {
                            break;
                        }
                        //println!("loop");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                })
                .unwrap(),
        );
        self
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

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

pub fn kill() {
    *KILLED.lock().unwrap() = true;
    info!("sent kill");
}

pub fn init(logger: Logger) {
    if LOGGER.set(logger).is_ok() {
        return;
    }
    panic!("Logger cannot be set more than once")
}

pub fn new_input_bufs(bufs: Vec<BufReader<LogBuffer>>) {
    let logger = LOGGER.get().expect("No logger initialized");
    logger.clear_buf_pool();
    logger.input_bufs(bufs);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger() {
        let log = "test log".to_string();
        err!("{log}");
    }
}
