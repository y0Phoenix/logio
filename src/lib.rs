pub mod macros;

use std::{
    fs::File,
    io::{BufReader, Read, Write},
    sync::{Arc, Mutex, OnceLock},
    thread::{self, JoinHandle},
};

use chrono::Local;

pub static LOGGER: OnceLock<Logger> = OnceLock::new();
pub static INTERNAL_WRITER: SharedLogBuf = Mutex::new(NewInternalLog::None);
pub static KILLED: Mutex<bool> = Mutex::new(false);

pub type LogBuffer = Box<dyn Read + Send + 'static>;

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

#[derive(Debug, Clone)]
pub struct LogFile(pub &'static str);

impl Default for LogFile {
    fn default() -> Self {
        LogFile("log.txt")
    }
}

pub type LogErr = String;

pub struct Logger {
    file: Option<LogFile>,
    logger_thread: Option<JoinHandle<()>>,
    input_buf_pool: Arc<Mutex<Vec<BufReader<LogBuffer>>>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            file: Some(LogFile::default()),
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

    pub fn log_file(mut self, file: LogFile) -> Self {
        self.file = Some(file);
        self
    }

    pub fn run(mut self) -> Self {
        let file_location = self.file.clone().expect("No log file defined.").0;

        let input_pool = Arc::clone(&self.input_buf_pool);

        self.logger_thread = Some(
            thread::Builder::new()
                .name("logger_thread".to_string())
                .spawn(move || {
                    let mut file = File::options()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(file_location)
                        .expect("Failed to open log file");

                    println!("file opened");
                    loop {
                        let mut pool = input_pool.lock().unwrap();
                        for buf in pool.iter_mut() {
                            let mut line = String::new();
                            match buf.read_to_string(&mut line) {
                                Ok(n) if n > 0 => {
                                    file.write_all(line.as_bytes()).unwrap();
                                    file.flush().unwrap();
                                }
                                _ => {} // No data or EOF
                            }
                        }

                        let mut lock = INTERNAL_WRITER.lock().unwrap();
                        if let NewInternalLog::New(log) = lock.clone() {
                            lock.reset();
                            println!("{:?}", lock.clone());
                            file.write_all(log.as_bytes()).unwrap();
                            file.flush().unwrap();
                        }
                        // for some reason rust will not drop the lock variable from memory after
                        // the loop block so need to do it manually
                        drop(lock);

                        if *KILLED.lock().unwrap() {
                            break;
                        }
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
    let log = convert_log(log_type, &log);
    *INTERNAL_WRITER.lock().unwrap() = NewInternalLog::New(log);
}

pub fn kill() {
    *KILLED.lock().unwrap() = true;
    println!("sent kill");
}

pub fn init(logger: Logger) {
    if LOGGER.set(logger).is_err() {
        panic!("Logger cannot be set more than once")
    }
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
    println!("{msg}");
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
