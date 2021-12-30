use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::path::Path;

pub struct LogWatcher {
    // created: SystemTime,
    pos: u64,
    reader: BufReader<File>,
    finish: bool,
}

impl LogWatcher {
    pub fn register<P: AsRef<Path>>(filename: P) -> Result<LogWatcher, io::Error> {
        let f = match File::open(&filename) {
            Ok(x) => x,
            Err(err) => return Err(err),
        };

        let metadata = match f.metadata() {
            Ok(x) => x,
            Err(err) => return Err(err),
        };

        let mut reader = BufReader::new(f);
        let pos = metadata.len();
        reader.seek(SeekFrom::Start(pos)).unwrap();
        Ok(LogWatcher {
            pos,
            reader,
            finish: false,
        })
    }

    // Gotta move this out
    pub fn watch<F: ?Sized>(&mut self, callback: &mut F)
    where
        F: FnMut(String),
    {
        loop {
            let mut line = String::new();
            let resp = self.reader.read_line(&mut line);
            match resp {
                Ok(len) => {
                    if len > 0 {
                        self.pos += len as u64;
                        self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                        callback(line.replace("\n", ""));

                        line.clear();
                    } else {
                        if self.finish {
                            break;
                        }
                        //TODO Reset to end of file if it shortens
                        callback(String::new());
                        self.reader.seek(SeekFrom::Start(self.pos)).unwrap();
                    }
                }
                Err(err) => {
                    println!("Logwatcher Error: {}", err);
                }
            }
        }
    }
}
