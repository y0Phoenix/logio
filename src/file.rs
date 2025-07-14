use std::fs;
use std::fs::read_dir;
use std::fs::remove_file;
use std::io::Read;
use std::io::Write;
use std::{fs::File, path::PathBuf};

use chrono::{Local, NaiveDateTime};

use crate::err;
use crate::info;
use crate::warn;

pub const MB: usize = 1024 * 1024;
// const GB: usize = 1024 * 1000000;
// 1GB max file size
pub const MAX_FILE_SIZE: usize = 1000000000;

#[derive(Debug, Default)]
pub struct Directory(pub &'static str);
#[derive(Debug, Default)]
pub struct FileName(pub &'static str);

#[derive(Debug, Default)]
pub struct DirFile {
    pub date_of_creation: NaiveDateTime,
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Default, Clone)]
pub enum ArchiveType {
    /// No archive functionality
    #[default]
    None,
    /// Full Archive functionality with the specified archive directory default 15 maximum archive
    /// files
    Archive(&'static str),
}

#[derive(Debug, Default)]
pub struct LogioFile {
    curr_file: Option<File>,
    _dir: Directory,
    _archive: ArchiveType,
}

impl LogioFile {
    pub fn new(curr_file: FileName, dir: Directory, archive: ArchiveType) -> Self {
        let curr_file = if let ArchiveType::Archive(archive_dir) = archive.clone() {
            LogioFile::archive_log(&curr_file, &dir, archive_dir)
        } else {
            File::options()
                .read(true)
                .write(true)
                .truncate(true)
                .open(curr_file.0)
                .expect("Failed to open new log file")
        };
        Self {
            curr_file: Some(curr_file),
            _dir: dir,
            _archive: archive,
        }
    }
    pub fn archive_log(log_file: &FileName, dir: &Directory, arch_dir: &'static str) -> File {
        let file_path = format!("{}/{}", dir.0, log_file.0);
        let arch_dir = format!("{}/{}", dir.0, arch_dir);

        let date_format = "%m-%d-%y %H:%M";

        info!("Checking logs directory");
        if fs::metadata(dir.0).is_err() {
            fs::create_dir(dir.0).expect("FS Error: Failed to create logs directory");
        }

        info!("Checking Archive Directory");
        if fs::metadata(&arch_dir).is_err() {
            fs::create_dir(&arch_dir).expect("FS Error: Failed To Create archives Directory");
        }

        let archives =
            read_dir(arch_dir).expect("Internal Error: Error Opening Log Archives Folder");

        let mut files = Vec::<DirFile>::new();

        for file in archives.into_iter() {
            match file {
                Ok(file) => match file.file_name().into_string() {
                    Ok(name) => {
                        if name.contains(".DS_Store") {
                            continue;
                        } else if !name.contains("log") {
                            warn!("Incompatable File Found In Archives: {}", &name);
                            continue;
                        }
                        let date = match NaiveDateTime::parse_from_str(
                            name.replace("log ", "").as_str(),
                            date_format,
                        ) {
                            Ok(date) => date,
                            Err(_) => Local::now().naive_local(),
                        };
                        files.push(DirFile {
                            date_of_creation: date,
                            path: file.path(),
                            name,
                        });
                    }
                    Err(e) => err!("Error Converting {:?} To String For Internal Use", e),
                },
                Err(_) => warn!("There Was A Problem Acessing A Log Archive File",),
            }
        }

        if files.len() > 15 {
            let oldest_file_index = LogioFile::find_oldest_file_date(&files).unwrap();
            let oldest_file = files.get(oldest_file_index).unwrap();

            match remove_file(oldest_file.path.clone()) {
                Ok(_) => info!(
                    "Log Archive Has Reached A Maximum of 15 Files. The Oldest File Was Removed `{}`.\nMove Files To Your Own Directory Manually To Prevent The Automatic Removal Of The Oldest File In The Future",
                    oldest_file.name
                ),
                Err(_) => warn!(
                    "Log Archive Has Reached The Maximum of 15. And An Error Occurred While Trying To Delete The Oldest File. Delete Some To Remove This Warning",
                ),
            }

            return File::create(file_path).expect("FS Error: Failed To Create New log.txt File");
        }

        let sys_time = Local::now().format(date_format);
        let formatted_path = format!("logs/archives/log {sys_time}");

        // new archive file creation
        if let Ok(mut old_log_file) = File::options().read(true).open(&file_path) {
            let mut new_log_file = File::create(formatted_path)
                .expect("FS Error: Failed To Create New Archive Log File");
            let file_size = match old_log_file.metadata() {
                Ok(metadata) => metadata.len() as usize,
                Err(e) => {
                    err!(
                        "Failed To Aquire MetaData for log.txt. Defaulting File Size To Config 'max_file_size' of {}. {}",
                        MAX_FILE_SIZE,
                        e
                    );
                    MAX_FILE_SIZE
                }
            };

            info!(
                "Preparing To Copy Old Log File To New Log File With Size {}",
                file_size
            );

            let mut buf_len = std::cmp::min(MB, file_size);
            loop {
                let mut old_buf = vec![0; buf_len];
                match old_log_file.read(&mut old_buf) {
                    Ok(n) => {
                        new_log_file.write_all(&old_buf[0..n]).expect(
                            "IO Error: Failed To Write Old Log Data To New Archive Log Buffer",
                        );
                        if n == 0 {
                            info!("All {} Bytes Read From Old Log File", file_size);
                            break;
                        }
                    }
                    Err(e) => {
                        err!("Error Reading From Old Log File Into New Buffer {:?}", e);
                        break;
                    }
                }

                // If we read less than the buffer size, we reached the end of the file
                if buf_len > file_size {
                    break;
                }

                // Decrease the buffer size for the next iteration, but keep it at least 1 byte
                buf_len = std::cmp::max(1, buf_len / 2);
            }
        }
        File::create(file_path).expect("FS Error: Failed To Create New log.txt File")
    }
    fn find_oldest_file_date(files: &[DirFile]) -> Option<usize> {
        if files.is_empty() {
            return None;
        }

        let mut oldest_date = files[0].date_of_creation;
        let mut file_to_remove = 0;
        for (i, file) in files.iter().enumerate() {
            let file_date = file.date_of_creation;
            if file_date < oldest_date {
                oldest_date = file_date;
                file_to_remove = i;
            }
        }
        Some(file_to_remove)
    }
    pub fn write_all(&mut self, buf: &[u8]) {
        match &mut self.curr_file {
            Some(file) => {
                file.write_all(buf).expect("failed to write to log file");
            }
            _ => {
                panic!("file should be initilized")
            }
        }
    }
    pub fn flush(&mut self) {
        match &mut self.curr_file {
            Some(file) => {
                file.flush().expect("failed to write to log file");
            }
            _ => {
                panic!("file should be initilized")
            }
        }
    }
}
