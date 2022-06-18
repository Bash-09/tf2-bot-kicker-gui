#![allow(non_upper_case_globals)]
#![allow(unused_variables)]

use crate::server::*;

use regex::{Captures, Regex};

use crate::server::player::*;

use super::{player_checker::PlayerChecker, settings::Settings};

pub struct LogMatcher {
    pub r: Regex,
    pub f: fn(
        serv: &mut Server,
        str: &str,
        caps: Captures,
        set: &Settings,
        bot_checker: &mut PlayerChecker,
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

// Reads lines from output of the "status" command
// Includes players on server, player name, state, steamid, time connected
// If no player exists on the server with a steamid from here, it creates a new player and adds it to the list
pub const REGEX_STATUS: &str =
    r#"^#\s*(\d+)\s"(.*)"\s+\[(U:\d:\d+)\]\s+(\d*:?\d\d:\d\d)\s+\d+\s*\d+\s*(\w+).*$"#;
pub fn fn_status(
    serv: &mut Server,
    str: &str,
    caps: Captures,
    set: &Settings,
    player_checker: &mut PlayerChecker,
) {
    let steamid = caps[3].replace("[⁣឴؜ᅟ ­͏]", "").to_string();

    let mut state = PlayerState::Spawning;
    if caps[5].eq("active") {
        state = PlayerState::Active;
    }

    // Get connected time of player
    let time = get_time(caps[4].to_string()).unwrap_or(0);

    // Check for name stealing
    let name = caps[2].trim().to_string();
    let mut stolen_name = false;
    for (k, p) in &serv.players {
        if steamid == p.steamid || time > p.time {
            continue;
        }
        stolen_name |= name == p.name;
    }

    // Update an existing player
    if let Some(p) = serv.players.get_mut(&steamid) {
        p.time = time;
        p.state = state;
        p.accounted = true;
        p.stolen_name = stolen_name;

        if p.stolen_name {
            p.name = name;
            player_checker.check_player_name(p);
        }

    // Create a new player entry
    } else {
        let mut new_connection: bool = false;
        if (time as f32) < set.alert_period {
            new_connection = true;
        }

        // Construct new player for the list
        let mut p = Player {
            userid: caps[1].to_string(),
            name,
            steamid,
            time,
            team: Team::None,
            state,
            player_type: PlayerType::Player,
            notes: None,

            accounted: true,
            new_connection,
            stolen_name,
        };

        if player_checker.check_player_steamid(&mut p) {
            log::info!("Known {:?} joining: {}", p.player_type, p.name);
        } else if player_checker.check_player_name(&mut p) {
            log::info!("Unknown {:?} joining: {}", p.player_type, p.name);
        }

        serv.players.insert(p.steamid.clone(), p);
    }
}

// Converts a given string time (e.g. 57:48 or 1:14:46) as an integer number of seconds
fn get_time(input: String) -> Option<u32> {
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
pub fn fn_lobby(
    serv: &mut Server,
    str: &str,
    caps: Captures,
    set: &Settings,
    bot_checker: &mut PlayerChecker,
) {
    let mut team = Team::None;

    match &caps[3] {
        "INVADERS" => team = Team::Invaders,
        "DEFENDERS" => team = Team::Defenders,
        _ => {}
    }

    match serv.players.get_mut(&caps[2].to_string()) {
        None => {}
        Some(p) => {
            p.team = team;
            p.accounted = true;

            // Alert server of bot joining the server
            if p.new_connection && p.player_type == PlayerType::Bot && set.join_alert {
                serv.new_bots.push((p.name.clone(), p.team));
                p.new_connection = false;
            }
        }
    }
}

pub const REGEX_USER_DISCONNECTED: &str = r#"^Disconnecting from .*"#;
pub fn fn_user_disconnect(
    serv: &mut Server,
    str: &str,
    caps: Captures,
    set: &Settings,
    bot_checker: &mut PlayerChecker,
) {
    serv.clear();
}
