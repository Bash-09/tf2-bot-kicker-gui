use std::fs::File;
use std::io::Read;

use regex::Regex;

use super::player::Player;

enum ParseType {
    None,
    Regex,
    Uuid,
    List,
}

pub struct BotChecker {
    bots_regx: Vec<Regex>,
    bots_uuid: Vec<String>,
}

impl BotChecker {
    pub fn new() -> BotChecker {
        let filename = "cfg/bots.cfg";

        let mut file = File::open(filename)
            .unwrap_or_else(|_| panic!("No bot config file found in cfg/{}!", filename));
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("Failed to read file cfg/{} for bot configuration.", filename));

        let mut bots_regx: Vec<Regex> = Vec::new();
        let mut bots_uuid: Vec<String> = Vec::new();

        let mut pt = ParseType::None;

        let reg_regx = Regex::new(r#"^regex:\s*$"#).unwrap();
        let reg_uuid = Regex::new(r#"^uuid:\s*$"#).unwrap();
        let reg_list = Regex::new(r#"^list:\s*$"#).unwrap();

        let reg_get_uuid = Regex::new(r#"\[?(?P<uuid>U:\d:\d+)\]?"#).unwrap();

        for line in contents.lines() {
            if line.trim().eq("") {
                continue;
            }
            if reg_regx.is_match(line) {
                pt = ParseType::Regex;
                continue;
            }
            if reg_uuid.is_match(line) {
                pt = ParseType::Uuid;
                continue;
            }
            if reg_list.is_match(line) {
                pt = ParseType::List;
                continue;
            }

            match pt {
                ParseType::None => continue,
                ParseType::Regex => match Regex::new(line) {
                    Ok(r) => {
                        bots_regx.push(r);
                    }
                    Err(_) => {
                        eprintln!("Failed to compile regex for: {}", line);
                    }
                },
                ParseType::Uuid => match reg_get_uuid.captures(line) {
                    None => {}
                    Some(c) => {
                        bots_uuid.push(c["uuid"].to_string());
                    }
                },
                ParseType::List => {
                    let mut list: Vec<String> = read_steamid3_list(line);
                    bots_uuid.append(&mut list);
                }
            }
        }

        BotChecker {
            bots_regx,
            bots_uuid,
        }
    }

    pub fn check_bot_name(&self, player_name: &str) -> bool {
        for regx in self.bots_regx.iter() {
            if regx.captures(player_name).is_some() {
                return true;
            }
        }
        false
    }

    pub fn check_bot_steamid(&self, player_steamid: &str) -> bool {
        for uuid in self.bots_uuid.iter() {
            if uuid.eq(player_steamid) {
                return true;
            }
        }
        false
    }

    pub fn check_bot(&self, p: &Player) -> bool {
        self.check_bot_steamid(&p.steamid) || self.check_bot_name(&p.name)
    }

    pub fn append_uuid(&mut self, uuid: String) {
        self.bots_uuid.push(uuid);
    }
}

/// Create a vector storing all steamid3's found within a file
fn read_steamid3_list(filename: &str) -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    let reg = Regex::new(r#"\[?(?P<uuid>U:\d:\d+)\]?"#).unwrap();

    if let Ok(mut file) = File::open(format!("cfg/{}", filename)) {
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("Failed to read file cfg/{} for bot configuration.", filename));

        for m in reg.find_iter(&contents) {
            match reg.captures(m.as_str()) {
                None => {}
                Some(c) => {
                    list.push(c["uuid"].to_string());
                }
            }
        }
    } else {
        println!("Could not get file cfg/{} to bot IDs", filename);
    }

    list
}
