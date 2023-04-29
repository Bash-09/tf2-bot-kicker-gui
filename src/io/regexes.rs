#![allow(non_upper_case_globals)]
#![allow(unused_variables)]

use regex::{Captures, Regex};

use crate::{
    player_checker::PlayerChecker,
    server::{player::*, Server},
};

use super::{command_manager::CommandManager, settings::Settings};

pub struct LogMatcher {
    pub r: Regex,
    pub f: fn(
        serv: &mut Server,
        str: &str,
        caps: Captures,
        set: &Settings,
        bot_checker: &mut PlayerChecker,
        cmd: &mut CommandManager,
    ),
}

impl LogMatcher {
    pub fn new(
        r: Regex,
        f: fn(
            serv: &mut Server,
            str: &str,
            caps: Captures,
            set: &Settings,
            bot_checker: &mut PlayerChecker,
            cmd: &mut CommandManager,
        ),
    ) -> LogMatcher {
        LogMatcher { r, f }
    }
}

/*
    Useful commands:
        status
        tf_lobby_debug
        tf_party_debug //Not sure if this is actually useful, not really necessary

        callvote kick <userid>
        vote option<1/2> // Can't really use

*/

/// Player killed someone
/// Matches:
///    0: Killer
///    1: Victim
///    2: Weapon
///    3: Crit?
pub const REGEX_KILL: &str = r#"^(.*)\skilled\s(.*)\swith\s(.*)\.(\s\(crit\))?$"#;

/// Chat message
/// Matches:
///    0: Player
///    1: Message
pub const REGEX_CHAT: &str = r#"^(?:\*DEAD\*\s)?(.*)\s:\s\s(.*)$"#;

// Reads lines from output of the "status" command
// Includes players on server, player name, state, steamid, time connected
// If no player exists on the server with a steamid from here, it creates a new player and adds it to the list
pub const REGEX_STATUS: &str =
    r#"^#\s*(\d+)\s"(.*)"\s+\[(U:\d:\d+)\]\s+(\d*:?\d\d:\d\d)\s+\d+\s*\d+\s*(\w+).*$"#;

pub struct StatusLine {
    pub userid: String,
    pub name: String,
    pub steamid: String,
    pub time: u32,
    pub state: PlayerState,
}

impl StatusLine {
    pub fn parse(caps: Captures) -> StatusLine {
        let mut player_state = PlayerState::Spawning;
        if caps[5].eq("active") {
            player_state = PlayerState::Active;
        }

        StatusLine {
            userid: caps[1].to_string(),
            name: caps[2].replace(INVIS_CHARS, "").trim().to_string(),
            steamid: caps[3].to_string(),
            time: get_time(&caps[4]).unwrap_or(0),
            state: player_state,
        }
    }
}

// Converts a given string time (e.g. 57:48 or 1:14:46) as an integer number of seconds
fn get_time(input: &str) -> Option<u32> {
    let mut t: u32 = 0;

    let splits: Vec<&str> = input.split(':').collect();
    let n = splits.len();

    for (i, v) in splits.iter().enumerate() {
        // let dt: u32 = v.parse::<u32>().expect(&format!("Had trouble parsing {} as u32", v));
        let dt = v.parse::<u32>();

        if dt.is_err() {
            return None;
        }

        t += 60u32.pow((n - i - 1) as u32) * dt.unwrap();
    }

    Some(t)
}

// Reads lines from output of the "tf_lobby_debug" command
// Includes the team of players on the server
// NOTE: Teams are stored as INVADERS/DEFENDERS and does not swap when Red/Blu swaps so it cannot
// be used to reliably check which team the user is on, it can only check relative to the user (same/opposite team)
pub const REGEX_LOBBY: &str =
    r#"^  Member\[(\d+)] \[(U:\d:\d+)]  team = TF_GC_TEAM_(\w+)  type = MATCH_PLAYER\s*$"#;

pub struct LobbyLine {
    pub steamid: String,
    pub team: Team,
}

impl LobbyLine {
    pub fn parse(caps: &Captures) -> LobbyLine {
        let mut team = Team::None;
        match &caps[3] {
            "INVADERS" => team = Team::Invaders,
            "DEFENDERS" => team = Team::Defenders,
            _ => {}
        }

        LobbyLine {
            steamid: caps[2].to_string(),
            team,
        }
    }
}

const INVIS_CHARS: &[char] = &[
    '\u{00a0}',
    '\u{00ad}',
    '\u{034f}',
    '\u{061c}',
    '\u{115f}',
    '\u{1160}',
    '\u{17b4}',
    '\u{17b5}',
    '\u{180e}',
    '\u{2000}',
    '\u{2001}',
    '\u{2002}',
    '\u{2003}',
    '\u{2004}',
    '\u{2005}',
    '\u{2006}',
    '\u{2007}',
    '\u{2008}',
    '\u{2009}',
    '\u{200a}',
    '\u{200b}',
    '\u{200c}',
    '\u{200d}',
    '\u{200e}',
    '\u{200f}',
    '\u{202f}',
    '\u{205f}',
    '\u{2060}',
    '\u{2061}',
    '\u{2062}',
    '\u{2063}',
    '\u{2064}',
    '\u{206a}',
    '\u{206b}',
    '\u{206c}',
    '\u{206d}',
    '\u{206e}',
    '\u{206f}',
    '\u{3000}',
    '\u{2800}',
    '\u{3164}',
    '\u{feff}',
    '\u{ffa0}',
    '\u{1d159}',
    '\u{1d173}',
    '\u{1d174}',
    '\u{1d175}',
    '\u{1d176}',
    '\u{1d177}',
    '\u{1d178}',
    '\u{1d179}',
    '\u{1d17a}',
];
