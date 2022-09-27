use core::fmt;

use egui::{Color32, RichText, Ui};
use serde::Serialize;

use crate::player_checker::PlayerRecord;

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

#[derive(PartialEq, Eq)]
pub enum PlayerState {
    Spawning,
    Active,
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

pub struct Player {
    pub userid: String,
    pub name: String,
    pub steamid32: String,
    pub steamid64: u64,
    pub time: u32,
    pub team: Team,
    pub state: PlayerState,
    pub player_type: PlayerType,
    pub notes: String,

    pub accounted: bool,
    pub stolen_name: bool,
    pub common_name: bool,
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
}

pub fn steamid_32_to_64(steamid32: &str) -> Option<u64> {
    let segments: Vec<&str> = steamid32.split(':').collect();

    let id32: u64 = if let Ok(id32) = segments.get(2)?.parse() {
        id32
    } else {
        return None;
    };

    Some(id32 + 76561197960265728)
}
