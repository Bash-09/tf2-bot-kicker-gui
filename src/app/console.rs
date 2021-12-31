
pub mod log_watcher;
use log_watcher::*;

pub mod commander;
use commander::*;

pub struct Console {
    pub log: LogWatcher,
    pub com: Commander,
}