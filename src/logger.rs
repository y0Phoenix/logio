use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    file::LogioFile,
    info,
    utils::{convert_log, LogType},
    LogBuffer, Name, NewInternalLog, ThreadChannel, INTERNAL_WRITER, KILLED,
};

pub type LogErr = String;

pub struct LoggerThread {
    _buf: Arc<Mutex<BufReader<Box<dyn Read + Send + 'static>>>>,
    _thread: JoinHandle<()>,
}

pub struct Logger {
    _main_thread: JoinHandle<()>,
    new_buf_tx: Sender<ThreadChannel>,
    file: Arc<Mutex<LogioFile>>,
}

impl Logger {
    pub fn new() -> Self {
        let main_thread_name = Name("main_logger_thread");
        let (new_buf_tx, new_buf_rx) = mpsc::channel::<ThreadChannel>();
        let file: Arc<Mutex<LogioFile>> = Arc::new(Mutex::new(LogioFile::default()));
        let file_clone = Arc::clone(&file);
        let main_thread = thread::Builder::new()
            .name(main_thread_name.0.to_string())
            .spawn(move || {
                let mut thread_pool: HashMap<Name, LoggerThread> = HashMap::new();
                loop {
                    if let Ok(channel_msg) = new_buf_rx.recv_timeout(Duration::from_millis(500)) {
                        match channel_msg {
                            ThreadChannel::Clear => {}
                            ThreadChannel::NewBufs(new_bufs) => {
                                new_bufs.into_iter().for_each(|buf| {
                                    let new_thread_file = Arc::clone(&file_clone);
                                    let new_buf = Arc::new(Mutex::new(buf.buf));
                                    let new_buf_clone = Arc::clone(&new_buf);
                                    let new_thread = thread::Builder::new()
                                        .name(buf.name.0.to_string())
                                        .spawn(move || loop {
                                            let mut str = String::new();
                                            match new_buf_clone.lock().unwrap().read_line(&mut str)
                                            {
                                                Ok(_) => {
                                                    let mut file_lock =
                                                        new_thread_file.lock().unwrap();
                                                    print!("{str}");
                                                    file_lock.write_all(str.as_bytes());
                                                    file_lock.flush();
                                                }
                                                Err(_) => {
                                                    info!("Closing thread");
                                                    break;
                                                }
                                            }
                                        })
                                        .expect(
                                            "Internal OS Error: Failed to spawn new buffer thread",
                                        );
                                    thread_pool.insert(
                                        buf.name,
                                        LoggerThread {
                                            _buf: new_buf,
                                            _thread: new_thread,
                                        },
                                    );
                                });
                            }
                        }
                    }
                    let mut file_lock = file_clone.lock().unwrap();
                    let mut log_lock = INTERNAL_WRITER.lock().unwrap();
                    if let NewInternalLog::New(log) = log_lock.clone() {
                        log_lock.reset();
                        LogioFile::write_all(&mut file_lock, log.as_bytes());
                        LogioFile::flush(&mut file_lock);
                    }
                    if *KILLED.lock().unwrap() {
                        let log = convert_log(LogType::Info, "Closing main logger thread");
                        println!("{log}");
                        LogioFile::write_all(&mut file_lock, log.as_bytes());
                        LogioFile::flush(&mut file_lock);
                        break;
                    }
                    drop(file_lock);
                    drop(log_lock);
                }
            })
            .expect("Internal OS Error: Failed to spawn main logging thread");
        Self {
            new_buf_tx,
            file,
            _main_thread: main_thread,
        }
    }

    pub fn input_buf(self, buffer: LogBuffer) -> Self {
        self.new_buf_tx
            .send(ThreadChannel::NewBufs(vec![buffer]))
            .expect("Internal IO Error: Failed to send new input buffer to thread channel");
        self
    }

    pub fn new_input_bufs(&self, bufs: Vec<LogBuffer>) {
        self.new_buf_tx
            .send(ThreadChannel::NewBufs(bufs))
            .expect("Internal IO Error: Failed to send new input buffer to thread channel");
    }

    pub fn log_file(self, file: LogioFile) -> Self {
        *self.file.lock().unwrap() = file;
        self
    }

    pub fn clear_buf_pool(&self) {
        self.new_buf_tx
            .send(ThreadChannel::Clear)
            .expect("Internal IO Error: Failed to send clear bufs to thread channel");
    }

    pub fn is_finished(&self) -> bool {
        self._main_thread.is_finished()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
