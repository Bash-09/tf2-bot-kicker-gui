use core::fmt;

use chrono::{NaiveDateTime, Utc};
use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{Color32, Label, RichText, Ui, Vec2};
use egui_extras::RetainedImage;
use serde::Serialize;
use wgpu_app::utils::persistent_window::PersistentWindow;

const ORANGE: Color32 = Color32::from_rgb(255, 165, 0);

use crate::{
    gui::{
        format_time,
        player_windows::{create_edit_notes_window, player_type_combobox},
        regex_windows::new_regex_window,
        truncate, TRUNC_LEN,
    },
    io::command_manager::KickReason,
    player_checker::PlayerRecord,
    state::State,
    steamapi::AccountInfo,
};

pub type Steamid64 = String;
pub type Steamid32 = String;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Team {
    Defenders,
    Invaders,
    None,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
pub enum PlayerType {
    Player,
    Bot,
    Cheater,
    Suspicious,
}

#[derive(PartialEq, Eq)]
pub enum PlayerState {
    Spawning,
    Active,
}

/// An action on a player initiated by the user through the UI
pub enum UserAction {
    Update(PlayerRecord),
    Kick(KickReason),
    GetProfile(Steamid64),
    OpenWindow(PersistentWindow<State>),
}

pub struct Player {
    pub userid: String,
    pub name: String,
    pub steamid32: Steamid32,
    pub steamid64: Steamid64,
    pub time: u32,
    pub team: Team,
    pub state: PlayerState,
    pub player_type: PlayerType,
    pub notes: String,

    pub accounted: u8,
    pub stolen_name: bool,
    pub common_name: bool,

    pub account_info: Option<Result<AccountInfo, reqwest::Error>>,
    pub profile_image: Option<RetainedImage>,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.steamid32 == other.steamid32
    }
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} - {}, \tUID: {}, SteamID: {}, State: {}, Type: {:?}",
            self.team, self.name, self.userid, self.steamid32, self.state, self.player_type
        )
    }
}

impl Player {
    pub fn get_export_steamid(&self) -> String {
        format!("[{}] - {}", &self.steamid32, &self.name)
    }

    pub fn get_export_regex(&self) -> String {
        regex::escape(&self.name)
    }

    pub fn get_record(&self) -> PlayerRecord {
        PlayerRecord {
            steamid: self.steamid32.clone(),
            player_type: self.player_type,
            notes: self.notes.clone(),
        }
    }

    /// Renders an editable summary of a player
    /// `allow_kick` enables a button in the context menu to call a votekick on the player
    /// `allow_steamapi` enables a button to request the user's steam account info and displays
    /// that info when hovering the player's name
    pub fn render_player(
        &self,
        ui: &mut Ui,
        user: &str,
        allow_kick: bool,
        allow_steamapi: bool,
    ) -> Option<UserAction> {
        static mut CONTEXT_MENU_OPEN: Option<String> = None;

        let mut ui_action: Option<UserAction> = None;

        // Player type
        let mut new_type = self.player_type;
        if player_type_combobox(&self.steamid32, &mut new_type, ui) {
            let mut record = self.get_record();
            record.player_type = new_type;
            ui_action = Some(UserAction::Update(record));
        }

        // Player name
        let text = if self.steamid32 == user {
            egui::RichText::new(truncate(&self.name, TRUNC_LEN)).color(Color32::GREEN)
        } else if self.player_type == PlayerType::Bot || self.player_type == PlayerType::Cheater {
            egui::RichText::new(truncate(&self.name, TRUNC_LEN)).color(self.player_type.color(ui))
        } else if self.stolen_name {
            egui::RichText::new(truncate(&self.name, TRUNC_LEN)).color(Color32::YELLOW)
        } else {
            egui::RichText::new(truncate(&self.name, TRUNC_LEN))
        };

        // Player name button styling
        ui.style_mut().visuals.widgets.inactive.bg_fill = ui.style().visuals.window_fill();

        // Player actions context menu
        let mut menu_open = false;
        let header = ui.menu_button(text, |ui| {
            // Don't show the hover ui if a menu is open, otherwise it can overlap the
            // currently open manu and be annoying
            menu_open = true;

            // Workaround to prevent opening a menu button then hovering a different
            // one changing the source of the menu
            // This is unsafe because I am using a static mut object inside the method,
            // which cases race conditions across threads, however since the UI will
            // only ever be manipulated from 1 thread I think it is safe.
            unsafe {
                match &CONTEXT_MENU_OPEN {
                    None => {
                        CONTEXT_MENU_OPEN = Some(self.steamid32.clone());
                    }
                    Some(id) => {
                        if id != &self.steamid32 {
                            CONTEXT_MENU_OPEN = None;
                            ui.close_menu();
                            return;
                        }
                    }
                }
            }

            if ui.button("Copy SteamID32").clicked() {
                let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                    ClipboardProvider::new();
                ctx.unwrap().set_contents(self.steamid32.clone()).unwrap();
                log::info!("{}", format!("Copied \"{}\"", self.steamid32));
            }

            if ui.button("Copy SteamID64").clicked() {
                let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                    ClipboardProvider::new();
                ctx.unwrap().set_contents(self.steamid64.clone()).unwrap();
                log::info!("{}", format!("Copied \"{}\"", self.steamid64));
            }

            if ui.button("Copy Name").clicked() {
                let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                    ClipboardProvider::new();
                ctx.unwrap().set_contents(self.name.clone()).unwrap();
                log::info!("{}", format!("Copied \"{}\"", self.name));
            }

            // Copy SteamID and Name buttons
            if ui.button("Edit Notes").clicked() {
                ui_action = Some(UserAction::OpenWindow(create_edit_notes_window(
                    self.get_record(),
                )));
            }

            if allow_steamapi {
                let refresh_button = ui.button("Refresh profile info");
                if refresh_button.clicked() {
                    ui_action = Some(UserAction::GetProfile(self.steamid64.clone()));
                }
            }

            ui.hyperlink_to(
                "Visit profile",
                format!("https://steamcommunity.com/profiles/{}", &self.steamid64),
            );

            // Other actions button
            if allow_kick
                || self.player_type == PlayerType::Bot
                || self.player_type == PlayerType::Cheater
            {
                // Call votekick button
                if allow_kick {
                    ui.menu_button(RichText::new("Call votekick").color(Color32::RED), |ui| {
                        let mut reason: Option<KickReason> = None;
                        if ui.button("No reason").clicked() {
                            reason = Some(KickReason::None);
                        }
                        if ui.button("Idle").clicked() {
                            reason = Some(KickReason::Idle);
                        }
                        if ui.button("Cheating").clicked() {
                            reason = Some(KickReason::Cheating);
                        }
                        if ui.button("Scamming").clicked() {
                            reason = Some(KickReason::Scamming);
                        }

                        if let Some(reason) = reason {
                            ui_action = Some(UserAction::Kick(reason));
                        }
                    });
                }

                // Save Name button
                if self.player_type == PlayerType::Bot || self.player_type == PlayerType::Cheater {
                    let but = ui.button(RichText::new("Save Name").color(Color32::RED));
                    if but.clicked() {
                        ui_action = Some(UserAction::OpenWindow(new_regex_window(
                            self.get_export_regex(),
                        )));
                    }
                    but.on_hover_text(
                        RichText::new("Players with this name will always be recognized as a bot")
                            .color(Color32::RED),
                    );
                }
            }
        });

        // Close context menu
        if header.response.clicked_elsewhere() {
            unsafe {
                CONTEXT_MENU_OPEN = None;
            }
        }

        // Don't show the hover ui if a menu is open, otherwise it can overlap the
        // currently open manu and be annoying. Only show the hover menu if there are
        // steam details (arrived or outstanding doesn't matter) or there is
        // information to show (i.e. notes or stolen name notification)
        if (allow_steamapi || !self.notes.is_empty() || self.stolen_name) && !menu_open {
            header.response.on_hover_ui(|ui| {
                self.render_account_info(ui);
                self.render_notes(ui);
            });
        }

        // Cheater, Bot and Joining labels
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            ui.add_space(15.0);

            // Time
            ui.label(&format_time(self.time));

            // Notes indicator
            if !self.notes.is_empty() {
                ui.label("☑");
            }

            // VAC and game bans, young account, or couldn't fetch profile
            if let Some(Ok(info)) = &self.account_info {
                if info.bans.VACBanned {
                    ui.label(RichText::new("V").color(Color32::RED));
                }
                if info.bans.NumberOfGameBans > 0 {
                    ui.label(RichText::new("G").color(Color32::RED));
                }
                if let Some(time) = info.summary.timecreated {
                    let age = Utc::now()
                        .naive_local()
                        .signed_duration_since(NaiveDateTime::from_timestamp(time as i64, 0));
                    if age.num_days() < (70) {
                        ui.label(RichText::new("Y").color(Color32::RED));
                    } else if age.num_days() < (365) {
                        ui.label(RichText::new("Y").color(ORANGE));
                    }
                }
                if info.summary.communityvisibilitystate == 1 {
                    ui.label(RichText::new("P").color(Color32::RED));
                } else if info.summary.communityvisibilitystate == 2 {
                    ui.label(RichText::new("F").color(Color32::YELLOW));
                }
            } else if let Some(Err(_)) = &self.account_info {
                ui.label(RichText::new("N").color(ORANGE));
            }

            // Cheater / Bot / Joining
            if self.player_type != PlayerType::Player {
                ui.add(Label::new(self.player_type.rich_text()));
            }
            if self.state == PlayerState::Spawning {
                ui.add(Label::new(RichText::new("Joining").color(Color32::YELLOW)));
            }
        });

        ui_action
    }

    /// Renders a view of the player's steam account info
    pub fn render_account_info(&self, ui: &mut Ui) {
        if let Some(info_request) = &self.account_info {
            match info_request {
                Ok(info) => {
                    let AccountInfo {
                        summary,
                        bans,
                        friends: _,
                    } = info;

                    ui.horizontal(|ui| {
                        if let Some(profile_img) = &self.profile_image {
                            profile_img.show_size(ui, Vec2::new(64.0, 64.0));
                        }

                        ui.vertical(|ui| {
                            ui.label(&summary.personaname);
                            ui.horizontal(|ui| {
                                ui.label("Profile: ");
                                ui.label(match summary.communityvisibilitystate {
                                    1 => RichText::new("Private").color(Color32::RED),
                                    2 => RichText::new("Friends-only").color(Color32::YELLOW),
                                    3 => RichText::new("Public").color(Color32::GREEN),
                                    _ => RichText::new("Invalid value"),
                                });
                            });

                            if let Some(time) = summary.timecreated {
                                let age = Utc::now().naive_local().signed_duration_since(
                                    NaiveDateTime::from_timestamp(time as i64, 0),
                                );
                                let years = age.num_days() / 365;
                                let days = age.num_days() - years * 365;

                                if years > 0 {
                                    ui.label(&format!(
                                        "Account Age: {} years, {} days",
                                        years, days
                                    ));
                                } else {
                                    ui.label(&format!("Account Age: {} days", days));
                                }

                                if age.num_days() < (70) {
                                    ui.label(
                                        RichText::new("(Very) Young account").color(Color32::RED),
                                    );
                                } else if age.num_days() < (365) {
                                    ui.label(
                                        RichText::new("Young account")
                                            .color(Color32::from_rgb(255, 165, 0)),
                                    );
                                }
                            }

                            if bans.VACBanned {
                                ui.label(
                                    RichText::new(format!(
                                        "This player has VAC bans: {}",
                                        bans.NumberOfVACBans
                                    ))
                                    .color(Color32::RED),
                                );
                            }

                            if bans.NumberOfGameBans > 0 {
                                ui.label(
                                    RichText::new(format!(
                                        "This player has Game bans: {}",
                                        bans.NumberOfGameBans
                                    ))
                                    .color(Color32::RED),
                                );
                            }

                            if bans.VACBanned || bans.NumberOfGameBans > 0 {
                                ui.label(
                                    RichText::new(format!(
                                        "Days since last ban: {}",
                                        bans.DaysSinceLastBan
                                    ))
                                    .color(Color32::RED),
                                );
                            }
                        });
                    });
                }
                Err(e) => {
                    let string = format!("{}", e);
                    ui.label(RichText::new("No profile could be retrieved").color(ORANGE));
                    if string.contains("missing field `profilestate`") {
                        ui.label("Profile may not be set up.");
                    }
                    ui.label(&format!("{}", e));
                }
            }
            ui.add_space(10.0);
        }
    }

    /// Render any notes saved for this account
    pub fn render_notes(&self, ui: &mut Ui) {
        if self.stolen_name || !self.notes.is_empty() {
            if self.stolen_name {
                ui.label(
                    RichText::new("A player with this name is already on the server.")
                        .color(Color32::YELLOW),
                );
            }
            if !self.notes.is_empty() {
                ui.label(&self.notes);
            }
        }
    }
}

pub fn create_demo_player(name: String, steamid32: String, team: Team) -> Player {
    let steamid64 = steamid_32_to_64(&steamid32).unwrap_or_default();

    Player {
        userid: String::from("0"),
        name,
        steamid32,
        steamid64,
        time: 69,
        team,
        state: PlayerState::Active,
        player_type: PlayerType::Player,
        notes: String::new(),

        accounted: 0,
        stolen_name: false,
        common_name: false,

        account_info: None,
        profile_image: None,
    }
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out: &str = match self {
            PlayerState::Active => "Active  ",
            PlayerState::Spawning => "Spawning",
        };
        write!(f, "{}", out)
    }
}

impl PlayerType {
    pub fn color(&self, ui: &Ui) -> Color32 {
        use PlayerType::*;
        match self {
            Player => ui.visuals().text_color(),
            Bot => Color32::RED,
            Cheater => Color32::from_rgb(255, 165, 0),
            Suspicious => Color32::LIGHT_RED,
        }
    }

    pub fn rich_text(&self) -> RichText {
        use PlayerType::*;
        match self {
            Player => RichText::new("Player"),
            Bot => RichText::new("Bot").color(Color32::RED),
            Cheater => RichText::new("Cheater").color(Color32::from_rgb(255, 165, 0)),
            Suspicious => RichText::new("Suspicious").color(Color32::LIGHT_RED),
        }
    }
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out: &str = match self {
            Team::Defenders => "DEF ",
            Team::Invaders => "INV ",
            Team::None => "NONE",
        };
        write!(f, "{}", out)
    }
}

/// Convert a steamid32 (U:0:1234567) to a steamid64 (76561197960265728)
pub fn steamid_32_to_64(steamid32: &Steamid32) -> Option<Steamid64> {
    let segments: Vec<&str> = steamid32.split(':').collect();

    let id32: u64 = if let Ok(id32) = segments.get(2)?.parse() {
        id32
    } else {
        return None;
    };

    Some(format!("{}", id32 + 76561197960265728))
}

/// Convert a steamid64 (76561197960265728) to a steamid32 (U:0:1234567)
pub fn steamid_64_to_32(steamid64: &Steamid64) -> Result<Steamid32, std::num::ParseIntError> {
    let id64: u64 = steamid64.parse()?;
    Ok(format!("U:1:{}", id64 - 76561197960265728))
}
