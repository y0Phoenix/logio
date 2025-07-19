pub mod file;
pub mod logger;
pub mod macros;
pub mod utils;

use std::{
    io::{BufReader, Read},
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

use crate::logger::Logger;

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
pub struct LogBuffer {
    pub name: Name,
    pub buf: BufReader<Box<dyn Read + Send + 'static>>,
}

impl LogBuffer {
    pub fn new<R>(name: Name, buf: R) -> Self
    where
        R: Read + Send + 'static,
    {
        Self {
            name,
            buf: BufReader::new(Box::new(buf)),
        }
    }
}

pub enum ThreadChannel {
    Clear,
    NewBufs(Vec<LogBuffer>),
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct Name(pub &'static str);

pub fn kill() {
    *KILLED.lock().unwrap() = true;
    info!("Sent kill to logger thread");
    if let Some(logger) = LOGGER.get() {
        while !logger.is_finished() {
            thread::sleep(Duration::from_millis(250));
        }
    } else {
        panic!("No logger initialized. Can't kill an uninitialized logger");
    }
}

pub fn init(logger: Logger) {
    if LOGGER.set(logger).is_ok() {
        return;
    }
    panic!("Logger cannot be set more than once")
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
