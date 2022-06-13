use tokio::net::TcpStream;

use rcon::Connection;
use regex::Regex;
use tokio::runtime::Runtime;

use crate::{
    bot_checker::BotChecker,
    logwatcher::LogWatcher,
    regexes::{
        f_lobby, f_status, f_user_disconnect, r_lobby, r_status, r_user_disconnect, LogMatcher,
    },
    server::Server,
    settings::Settings,
    timer::Timer,
};

pub struct State {
    pub refresh_timer: Timer,
    pub alert_timer: Timer,
    pub kick_timer: Timer,

    pub message: String,

    pub settings: Settings,
    pub rcon: rcon::Result<Connection<TcpStream>>,
    pub log: Option<LogWatcher>,

    pub server: Server,

    pub regx_status: LogMatcher,
    pub regx_lobby: LogMatcher,
    pub regx_disconnect: LogMatcher,

    pub bot_checker: BotChecker,
}

impl State {
    pub fn new(runtime: &Runtime) -> State {
        let settings: Settings;

        let mut message = String::from("Loaded");
        log::info!("Loaded");

        // Attempt to load settings, create new default settings if it can't load an existing file
        let set = Settings::import("cfg/settings.json");
        if set.is_err() {
            settings = Settings::new();
            message = format!("Error loading settings: {}", set.unwrap_err());
            log::warn!("{}", message);
        } else {
            settings = set.unwrap();
        }

        // Load regexes
        let regx_status = LogMatcher::new(Regex::new(r_status).unwrap(), f_status);
        let regx_lobby = LogMatcher::new(Regex::new(r_lobby).unwrap(), f_lobby);
        let regx_disconnect =
            LogMatcher::new(Regex::new(r_user_disconnect).unwrap(), f_user_disconnect);

        // Create bot checker and load any bot detection rules saved
        let mut bot_checker = BotChecker::new();
        for uuid_list in &settings.steamid_lists {
            match bot_checker.add_steamid_list(uuid_list) {
                Ok(_) => {}
                Err(e) => {
                    message = format!("Error loading {}: {}", uuid_list, e);
                    log::error!("{}", message);
                }
            }
        }
        for regex_list in &settings.regex_lists {
            match bot_checker.add_regex_list(regex_list) {
                Ok(_) => {}
                Err(e) => {
                    message = format!("Error loading {}: {}", regex_list, e);
                    log::error!("{}", message);
                }
            }
        }

        let mut rcon = None;
        runtime.block_on(async {
            rcon = Some(Connection::connect("127.0.0.1:27015", &settings.rcon_password).await);
        });

        let log = LogWatcher::use_directory(&settings.tf2_directory);

        State {
            refresh_timer: Timer::new(),
            alert_timer: Timer::new(),
            kick_timer: Timer::new(),

            message,
            settings,
            rcon: rcon.unwrap(),
            log,
            server: Server::new(),

            regx_status,
            regx_lobby,
            regx_disconnect,

            bot_checker,
        }
    }

    /// Checks if a valid rcon connection is currently established
    pub async fn rcon_connected(&mut self) -> bool {
        match &mut self.rcon {
            Ok(con) => match con.cmd("echo Ping").await {
                Ok(_) => {
                    return true;
                }
                Err(e) => {
                    self.rcon = Err(e);
                    return false;
                }
            },
            Err(_) => {
                match Connection::connect("127.0.0.1:27015", &self.settings.rcon_password).await {
                    Ok(con) => {
                        self.rcon = Ok(con);
                        return true;
                    }
                    Err(e) => {
                        self.rcon = Err(e);
                        return false;
                    }
                }
            }
        }
    }

    /// Begins a refresh on the local server state, any players unaccounted for since the last time this function was called will be removed.
    pub async fn refresh(&mut self) {
        if !self.rcon_connected().await {
            return;
        }
        self.server.prune();

        // Run status and tf_lobby_debug commands
        let status = self.rcon.as_mut().unwrap().cmd("status").await;
        let lobby = self.rcon.as_mut().unwrap().cmd("tf_lobby_debug").await;

        if status.is_err() || lobby.is_err() {
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
                        &mut self.bot_checker,
                    );
                }
            }
        }
    }
}
