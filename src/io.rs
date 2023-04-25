use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::TryRecvError;
use regex::Regex;

use regexes::LobbyLine;
use regexes::StatusLine;
use regexes::REGEX_LOBBY;
use regexes::REGEX_STATUS;

use crate::settings;
use crate::settings::Settings;

use self::command_manager::CommandManager;
use self::logwatcher::LogWatcher;

pub mod command_manager;
pub mod logwatcher;
pub mod regexes;

/// Holds stuff to communicate with the IO thread, send [IORequest]s via the IOManager to do things like run commands in the game, etc
pub struct IOManager {
    sender: Sender<IORequest>,
    receiver: Receiver<IOResponse>,
}

struct IOThread {
    sender: Sender<IOResponse>,
    receiver: Receiver<IORequest>,

    command_manager: CommandManager,
    log_watcher: Option<LogWatcher>,

    tf2_directory: String,

    regex_status: Regex,
    regex_lobby: Regex,
}

/// Request an action to be done on the IO thread, such as update state, run a command in-game, etc
pub enum IORequest {
    UpdateDirectory(String),
    UpdateRconPassword(String),
    RunCommand(String),
}

/// A message from the IO thread that something has happened, like a status output which needs to be handled.
pub enum IOResponse {
    NoLogFile(std::io::Error),
    LogFileOpened,
    NoRCON(rcon::Error),
    RCONConnected,
    Status(StatusLine),
    Lobby(LobbyLine),
}

impl IOManager {
    /// Start a new thread for IO and return a Manager containing message channels for it.
    pub fn start(settings: &Settings) -> IOManager {
        let (msend, trecv) = crossbeam_channel::unbounded();
        let (tsend, mrecv) = crossbeam_channel::unbounded();
        log::debug!("Spawning IO thread");

        let dir = settings.tf2_directory.clone();
        let pwd = settings.rcon_password.clone();

        // Thread to do stuff on
        std::thread::spawn(move || {
            log::debug!("IO Thread running");
            let mut io = IOThread::new(tsend, trecv, dir, pwd);

            io.reopen_log();
            io.reconnect_rcon();

            loop {
                io.handle_messages();
                io.handle_log();
            }
        });

        IOManager {
            sender: msend,
            receiver: mrecv,
        }
    }

    /// Send a message to the IO thread
    pub fn send(&mut self, msg: IORequest) {
        self.sender.send(msg).expect("Sending message to IO thread");
    }

    /// Receive a message from the IO thread, returns none if there are no messages waiting.
    pub fn recv(&mut self) -> Option<IOResponse> {
        match self.receiver.try_recv() {
            Ok(resp) => Some(resp),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(_) => panic!("Lost connection to IO thread"),
        }
    }
}

impl IOThread {
    fn new(
        send: Sender<IOResponse>,
        recv: Receiver<IORequest>,
        directory: String,
        password: String,
    ) -> IOThread {
        let command_manager = CommandManager::new(password);

        IOThread {
            sender: send,
            receiver: recv,
            command_manager,
            log_watcher: None,
            tf2_directory: directory,

            regex_status: Regex::new(REGEX_STATUS).unwrap(),
            regex_lobby: Regex::new(REGEX_LOBBY).unwrap(),
        }
    }

    /// Deal with all of the queued messages
    fn handle_messages(&mut self) {
        loop {
            match self.next_message() {
                None => {
                    break;
                }
                Some(IORequest::UpdateDirectory(dir)) => {
                    self.tf2_directory = dir;
                    self.reopen_log();
                }
                Some(IORequest::UpdateRconPassword(pwd)) => {
                    if let Err(e) = self.command_manager.set_password(pwd) {
                        log::error!("Could not initiate RCon connection: {:?}", e);
                        self.send_message(IOResponse::NoRCON(e));
                    }
                }
                Some(IORequest::RunCommand(cmd)) => self.handle_command(&cmd),
            }
        }
    }

    /// Parse all of the new log entries that have been written
    fn handle_log(&mut self) {
        if self.log_watcher.as_ref().is_none() {
            return;
        }

        while let Some(line) = self.log_watcher.as_mut().unwrap().next_line() {
            // Match status
            if let Some(caps) = self.regex_status.captures(&line) {
                let status_line = StatusLine::parse(caps);
                self.send_message(IOResponse::Status(status_line));
                continue;
            }
        }
    }

    /// Attempt to reopen the log file with the currently set directory.
    /// If the log file fails to be opened, an [IOResponse::NoLogFile] is sent back to the main thread and [Self::log_watcher] is set to [None]
    fn reopen_log(&mut self) {
        match LogWatcher::use_directory(&self.tf2_directory) {
            Ok(lw) => self.log_watcher = Some(lw),
            Err(e) => {
                log::error!("Failed to open log file: {:?}", e);
                self.send_message(IOResponse::NoLogFile(e));
            }
        }
    }

    /// Attempt to reconnect to TF2 rcon is it's currently disconnected
    fn reconnect_rcon(&mut self) {
        if self.command_manager.is_connected() {
            self.send_message(IOResponse::RCONConnected);
            return;
        }

        if let Err(e) = self.command_manager.try_connect() {
            self.send_message(IOResponse::NoRCON(e));
            return;
        }

        self.send_message(IOResponse::RCONConnected);
    }

    /// Run a command and handle the response from it
    fn handle_command(&mut self, command: &str) {
        if !self.command_manager.is_connected() {
            self.reconnect_rcon();
        }

        match self.command_manager.run_command(command) {
            Err(e) => {
                log::error!("Failed to run command: {:?}", e);
                self.send_message(IOResponse::NoRCON(e));
            }
            Ok(resp) => {
                for l in resp.lines() {
                    // Match lobby command
                    if let Some(caps) = self.regex_lobby.captures(l) {
                        let lobby_line = LobbyLine::parse(&caps);
                        self.send_message(IOResponse::Lobby(lobby_line));
                        continue;
                    }
                }
            }
        }
    }

    /// Get the next queued message or None.
    fn next_message(&mut self) -> Option<IORequest> {
        match self.receiver.try_recv() {
            Ok(request) => Some(request),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("Lost connection to main thread, shutting down.")
            }
        }
    }

    /// Send a message back to the main thread
    fn send_message(&mut self, msg: IOResponse) {
        if let Err(e) = self.sender.send(msg) {
            panic!("Failed to talk to main thread: {:?}", e);
        }
    }
}
