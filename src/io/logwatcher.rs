use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::time::SystemTime;

pub struct LogWatcher {
    // created: SystemTime,
    filename: String,
    pos: u64,
    reader: BufReader<File>,
    last_activity: SystemTime,
}

impl LogWatcher {
    // Try to open this TF2 directory
    pub fn use_directory(dir: &str) -> Result<LogWatcher, io::Error> {
        LogWatcher::register(&format!("{}/tf/console.log", dir))
    }

    /// Internally called by [use_directory]
    pub fn register(filename: &str) -> Result<LogWatcher, io::Error> {
        let f = match File::open(filename) {
            Ok(x) => {
                log::debug!("Successfully opened file {}", filename);
                x
            }
            Err(err) => {
                log::error!("Failed to open file {}: {}", filename, err);
                return Err(err);
            }
        };

        let metadata = match f.metadata() {
            Ok(x) => x,
            Err(err) => {
                log::error!("Failed to get file metadata: {}", err);
                return Err(err);
            }
        };

        let mut reader = BufReader::new(f);
        let pos = metadata.len();
        if let Err(e) = reader.seek(SeekFrom::Start(pos)) {
            log::error!("Failed to seek in file: {}", e);
        }
        Ok(LogWatcher {
            filename: filename.to_string(),
            pos,
            reader,
            last_activity: SystemTime::now(),
        })
    }

    pub fn next_line(&mut self) -> Option<String> {
        let mut line = String::new();
        let resp = self.reader.read_line(&mut line);

        match resp {
            Ok(len) => {
                // Get next line
                if len > 0 {
                    self.pos += len as u64;
                    self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                    self.last_activity = SystemTime::now();
                    return Some(line.replace('\n', ""));
                }

                // Check if file has been shortened
                if self.reader.get_ref().metadata().unwrap().len() < self.pos {
                    log::warn!("Console.log file was reset");
                    self.pos = self.reader.get_ref().metadata().unwrap().len();
                    self.last_activity = SystemTime::now();
                }

                // Reopen the log file if nothing has happened for long enough in case the file has been replaced.
                let time = SystemTime::now().duration_since(self.last_activity);
                if time.unwrap().as_secs() > 10 {
                    let f = match File::open(&self.filename) {
                        Ok(x) => x,
                        Err(_) => return None,
                    };

                    let metadata = match f.metadata() {
                        Ok(x) => x,
                        Err(_) => return None,
                    };

                    let mut reader = BufReader::new(f);
                    let pos = metadata.len();
                    reader.seek(SeekFrom::Start(pos)).unwrap();

                    self.pos = pos;
                    self.reader = reader;
                    self.last_activity = SystemTime::now();
                    return None;
                }

                self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                return None;
            }
            Err(err) => {
                log::error!("Logwatcher error: {}", err);
            }
        }

        None
    }
}
