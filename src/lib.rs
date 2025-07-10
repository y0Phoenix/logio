use std::{io::{Read, Write}, sync::{self, mpsc::{Receiver, Sender}, Arc, Mutex, OnceLock}, thread::JoinHandle};

// mod macros;
// pub use macros::*;

pub static LOGGER: OnceLock<Logger> = OnceLock::new();

pub type LogBuffer = Box<dyn Write + Send + 'static>;

pub struct Logger {
    logger_thread: Option<JoinHandle<()>>,
    input_buf_pool: Arc<Mutex<Vec<LogBuffer>>>,
    output_buf_pool: Arc<Mutex<Vec<LogBuffer>>>,
    killed: Arc<Mutex<bool>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            logger_thread: None,
            input_buf_pool: Arc::new(Mutex::new(Vec::new())),
            output_buf_pool: Arc::new(Mutex::new(Vec::new())),
            killed: Arc::new(Mutex::new(false))
        }
    }

}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
