use std::{sync::mpsc::Receiver, error::Error};

use regex::Regex;

use crate::{
    command_manager::{self, CommandManager},
    logwatcher::LogWatcher,
    player_checker::{PlayerChecker, PLAYER_LIST, REGEX_LIST},
    regexes::{
        fn_lobby, fn_status, LogMatcher, REGEX_LOBBY, REGEX_STATUS
    },
    server::Server,
    settings::Settings,
    timer::Timer, version::VersionResponse,
};

pub struct State {
    pub refresh_timer: Timer,
    pub alert_timer: Timer,
    pub kick_timer: Timer,

    pub settings: Settings,
    pub log: Option<LogWatcher>,

    pub server: Server,

    pub regx_status: LogMatcher,
    pub regx_lobby: LogMatcher,

    pub player_checker: PlayerChecker,

    pub latest_version: Option<Receiver<Result<VersionResponse, Box<dyn Error + Send>>>>,
    pub force_latest_version: bool,
}

impl State {
    pub fn new() -> State {
        let settings: Settings;

        // Attempt to load settings, create new default settings if it can't load an existing file
        let set = Settings::import("cfg/settings.json");

        if let Ok(set) = set {
            settings = set;
        } else {
            settings = Settings::new();
            log::warn!(
                "{}",
                format!("Error loading settings: {}", set.unwrap_err())
            );
        }

        // Load regexes
        let regx_status = LogMatcher::new(Regex::new(REGEX_STATUS).unwrap(), fn_status);
        let regx_lobby = LogMatcher::new(Regex::new(REGEX_LOBBY).unwrap(), fn_lobby);

        // Create player checker and load any regexes and players saved
        let mut player_checker = PlayerChecker::new();
        match player_checker.read_players(PLAYER_LIST) {
            Ok(()) => {
                log::info!("Loaded playerlist");
            }
            Err(e) => {
                log::error!("Failed to read playlist: {:?}", e);
            }
        }
        match player_checker.read_regex_list(REGEX_LIST) {
            Ok(_) => {}
            Err(e) => {
                log::error!("{}", format!("Error loading {}: {}", REGEX_LIST, e));
            }
        }

        let log = LogWatcher::use_directory(&settings.tf2_directory);

        State {
            refresh_timer: Timer::new(),
            alert_timer: Timer::new(),
            kick_timer: Timer::new(),

            settings,
            log,
            server: Server::new(),

            regx_status,
            regx_lobby,

            player_checker,
            latest_version: None,
            force_latest_version: false,
        }
    }

    /// Begins a refresh on the local server state, any players unaccounted for since the last time this function was called will be removed.
    pub fn refresh(&mut self, cmd: &mut CommandManager) {
        if cmd.connected(&self.settings.rcon_password).is_err() {
            return;
        }
        self.server.prune();

        // Run status and tf_lobby_debug commands
        let status = cmd.run_command(command_manager::CMD_STATUS);
        let lobby = cmd.run_command(command_manager::CMD_TF_LOBBY_DEBUG);

        if status.is_none() || lobby.is_none() {
            return;
        }

        let lobby = lobby.unwrap();

        // Not connected to valid server
        if lobby.contains("Failed to find lobby shared object") {
            self.server.clear();
            return;
        }

        self.server.refresh();

        // Parse players from tf_lobby_debug output
        for l in lobby.lines() {
            match self.regx_lobby.r.captures(l) {
                None => {}
                Some(c) => {
                    (self.regx_lobby.f)(
                        &mut self.server,
                        l,
                        c,
                        &self.settings,
                        &mut self.player_checker,
                        cmd,
                    );
                }
            }
        }
    }
}
