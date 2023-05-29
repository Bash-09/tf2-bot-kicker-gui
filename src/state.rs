use std::error::Error;

use crossbeam_channel::{Receiver, Sender};
use wgpu_app::utils::persistent_window::PersistentWindow;

use crate::{
    gui,
    io::{
        command_manager,
        regexes::{ChatMessage, LobbyLine, PlayerKill, StatusLine},
        IOManager, IORequest, IOResponse,
    },
    player_checker::{PlayerChecker, PLAYER_LIST, REGEX_LIST},
    server::{
        player::{steamid_32_to_64, Player, PlayerType, Team},
        Server,
    },
    settings::Settings,
    steamapi::{self, AccountInfoReceiver},
    timer::Timer,
    version::VersionResponse,
};

pub struct State {
    pub refresh_timer: Timer,
    pub alert_timer: Timer,
    pub kick_timer: Timer,

    pub settings: Settings,
    pub server: Server,
    pub player_checker: PlayerChecker,

    pub latest_version: Option<Receiver<Result<VersionResponse, Box<dyn Error + Send>>>>,
    pub force_latest_version: bool,

    pub steamapi_request_sender: Sender<String>,
    pub steamapi_request_receiver: AccountInfoReceiver,

    has_connected: bool,
    is_connected: Result<bool, rcon::Error>,
    pub log_open: Result<bool, std::io::Error>,

    pub io: IOManager,

    pub ui_context_menu_open: Option<usize>,
    pub new_persistent_windows: Vec<PersistentWindow<State>>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl egui_dock::TabViewer for State {
    type Tab = gui::GuiTab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut gui::GuiTab) {
        match tab {
            gui::GuiTab::Settings => gui::render_settings(ui, self),
            gui::GuiTab::Players => gui::render_players(ui, self),
            gui::GuiTab::ChatLog => gui::render_chat(ui, self),
            gui::GuiTab::DeathLog => gui::render_kills(ui, self),
        }
    }

    fn title(&mut self, tab: &mut gui::GuiTab) -> egui::WidgetText {
        match tab {
            gui::GuiTab::Settings => "Settings",
            gui::GuiTab::Players => "Players",
            gui::GuiTab::ChatLog => "Chat",
            gui::GuiTab::DeathLog => "Kills",
        }
        .into()
    }
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

        let (steamapi_request_sender, steamapi_request_receiver) =
            steamapi::create_api_thread(settings.steamapi_key.clone());

        let server = Server::new();
        let io = IOManager::start(&settings);

        State {
            refresh_timer: Timer::new(),
            alert_timer: Timer::new(),
            kick_timer: Timer::new(),

            settings,
            server,

            player_checker,
            latest_version: None,
            force_latest_version: false,

            steamapi_request_sender,
            steamapi_request_receiver,

            has_connected: false,
            is_connected: Ok(false),
            log_open: Ok(false),

            io,

            ui_context_menu_open: None,
            new_persistent_windows: Vec::new(),
        }
    }

    pub fn has_connected(&self) -> bool {
        self.has_connected
    }

    pub fn is_connected(&self) -> &Result<bool, rcon::Error> {
        &self.is_connected
    }

    /// Begins a refresh on the local server state, any players unaccounted for since the last time this function was called will be removed.
    pub fn refresh(&mut self) {
        self.server.prune();

        // Run status and tf_lobby_debug commands
        self.io.send(IORequest::RunCommand(
            command_manager::CMD_STATUS.to_string(),
        ));
        self.io.send(IORequest::RunCommand(
            command_manager::CMD_TF_LOBBY_DEBUG.to_string(),
        ));

        self.server.refresh();
    }

    pub fn handle_messages(&mut self) {
        while let Some(resp) = self.io.recv() {
            match resp {
                IOResponse::NoLogFile(e) => self.log_open = Err(e),
                IOResponse::LogFileOpened => self.log_open = Ok(true),
                IOResponse::NoRCON(e) => {
                    self.is_connected = Err(e);

                    if self.has_connected && self.settings.close_on_disconnect {
                        log::info!("Connection to TF2 has been lost, closing.");
                        std::process::exit(0);
                    }
                }
                IOResponse::RCONConnected => {
                    self.is_connected = Ok(true);
                    self.has_connected = true;
                }
                IOResponse::Status(status) => self.handle_status(status),
                IOResponse::Lobby(lobby) => self.handle_lobby(lobby),
                IOResponse::Chat(chat) => self.handle_chat(chat),
                IOResponse::Kill(kill) => self.handle_kill(kill),
            }
        }
    }

    fn handle_status(&mut self, status: StatusLine) {
        let steamid64 = steamid_32_to_64(&status.steamid).unwrap_or_default();
        if steamid64.is_empty() {
            log::error!(
                "Could not convert steamid32 to steamid64: {}",
                status.steamid
            );
        }

        //     // Check for name stealing
        //     let mut stolen_name = false;
        //     for (k, p) in server.get_players() {
        //         if steamid32 == p.steamid32 || time > p.time {
        //             continue;
        //         }
        //         stolen_name |= name == p.name;
        //     }

        // Update existing player
        if let Some(p) = self.server.get_player_mut(&status.steamid) {
            p.userid = status.userid;
            p.time = status.time;
            p.state = status.state;
            p.accounted = 0;

        //         p.stolen_name = stolen_name;
        //         if p.name != name {
        //             log::debug!("Different name! {}, {}", &p.name, &name);
        //             p.name = name;

        //             // Handle name stealing
        //             if p.stolen_name && settings.announce_namesteal {
        //                 cmd.send_chat(&format!("A bot has stolen {}'s name.", &p.name));
        //             }
        //             if p.stolen_name && settings.mark_name_stealers && p.player_type == PlayerType::Player {
        //                 p.player_type = PlayerType::Bot;

        //                 if !p.notes.is_empty() {
        //                     p.notes.push('\n');
        //                 }
        //                 p.notes
        //                     .push_str("Automatically marked as name-stealing bot.");
        //                 player_checker.update_player(p);
        //             }
        //         }
        } else {
            // Create a new player entry
            let mut p = Player {
                userid: status.userid,
                name: status.name,
                steamid32: status.steamid,
                steamid64,
                time: status.time,
                team: Team::None,
                state: status.state,
                player_type: PlayerType::Player,
                notes: String::new(),
                accounted: 0,
                stolen_name: false,
                //                stolen_name,
                common_name: false,
                account_info: None,
                profile_image: None,
            };

            self.server.pending_lookup.push(p.steamid64.clone());

            // Lookup player
            if let Some(record) = self.player_checker.check_player_steamid(&p.steamid32) {
                p.player_type = record.player_type;
                p.notes = record.notes;
                log::info!("Known {:?} joining: {}", p.player_type, &p.name);

                if self.player_checker.check_player_name(&p.name).is_some() {
                    p.common_name = true;
                }
            }

            // Check player name
            if let Some(regx) = self.player_checker.check_player_name(&p.name) {
                p.player_type = PlayerType::Bot;
                p.common_name = true;
                if p.notes.is_empty() {
                    p.notes = format!("Matched regex {}", regx.as_str());
                }

                self.player_checker.update_player(&p);
                log::info!("Unknown {:?} joining: {}", p.player_type, p.name);
            }

            //         // Handle name stealing
            //         if stolen_name && settings.announce_namesteal && p.time < settings.refresh_period as u32 {
            //             cmd.send_chat(&format!("A bot has stolen {}'s name.", &p.name));
            //         }
            //         if p.stolen_name && settings.mark_name_stealers && p.player_type == PlayerType::Player {
            //             p.player_type = PlayerType::Bot;

            //             if !p.notes.is_empty() {
            //                 p.notes.push('\n');
            //             }
            //             p.notes
            //                 .push_str("Automatically marked as name-stealing bot.");
            //             player_checker.update_player(&p);
            //         }

            if p.time <= (self.settings.refresh_period * 1.5).ceil() as u32 {
                self.server.new_connections.push(p.steamid32.clone());
            }

            self.server.add_player(p);
        }
    }

    fn handle_lobby(&mut self, lobby: LobbyLine) {
        if let Some(p) = self.server.get_player_mut(&lobby.steamid) {
            p.team = lobby.team;
            p.accounted = 0;
        }
    }

    fn handle_chat(&mut self, mut chat: ChatMessage) {
        log::info!(
            "Got chat message from {}: {}",
            chat.player_name,
            chat.message
        );

        if let Some((k, _)) = self
            .server
            .get_players()
            .iter()
            .find(|(_, v)| v.name == chat.player_name)
        {
            chat.steamid = Some(k.clone());
        }

        self.server.add_chat(chat);
    }

    fn handle_kill(&mut self, mut kill: PlayerKill) {
        log::info!(
            "{} killed {} with {}{}",
            kill.killer_name,
            kill.victim_name,
            kill.weapon,
            if kill.crit { " (crit)" } else { "" }
        );

        if let Some((k, _)) = self
            .server
            .get_players()
            .iter()
            .find(|(_, v)| v.name == kill.killer_name)
        {
            kill.killer_steamid = Some(k.clone());
        } else {
            log::error!(
                "Player {} could not be found when processing kill.",
                kill.killer_name
            );
        }
        if let Some((k, _)) = self
            .server
            .get_players()
            .iter()
            .find(|(_, v)| v.name == kill.victim_name)
        {
            kill.victim_steamid = Some(k.clone());
        } else {
            log::error!(
                "Player {} could not be found when processing kill.",
                kill.victim_name
            );
        }

        self.server.add_kill(kill);
    }
}
