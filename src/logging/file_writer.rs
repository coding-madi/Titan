use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, MutexGuard},
};

use tracing_subscriber::fmt::MakeWriter;

pub struct FileWriter {
    file: Mutex<File>,
}

impl FileWriter {
    pub fn new(path: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Failed to open log file");

        FileWriter {
            file: Mutex::new(file),
        }
    }
}

impl<'a> MakeWriter<'a> for FileWriter {
    type Writer = FileGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        FileGuard {
            lock: self.file.lock().unwrap(),
        }
    }
}

pub struct FileGuard<'a> {
    lock: MutexGuard<'a, File>,
}

impl<'a> Write for FileGuard<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.lock.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.lock.flush()
    }
}
