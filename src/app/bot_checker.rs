use std::fs::File;
use std::io::Read;

use regex::Regex;

use super::player::Player;

pub struct BotChecker {
    bots_regx: Vec<Regex>,
    bots_uuid: Vec<String>,
}

impl BotChecker {
    pub fn new() -> BotChecker {
        BotChecker {
            bots_regx: Vec::new(),
            bots_uuid: Vec::new(),
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

    #[allow(dead_code)]
    pub fn check_bot(&self, p: &Player) -> bool {
        self.check_bot_steamid(&p.steamid) || self.check_bot_name(&p.name)
    }

    pub fn append_uuid(&mut self, uuid: String) {
        self.bots_uuid.push(uuid);
    }

    /// Create a vector storing all steamid3's found within a file
    pub fn add_steamid_list(&mut self, filename: &str) -> Result<(), std::io::Error> {
        let mut list: Vec<String> = Vec::new();
        let reg = Regex::new(r#"\[?(?P<uuid>U:\d:\d+)\]?"#).unwrap();

        let mut file = File::open(filename)?;

        let mut contents: String = String::new();
        file.read_to_string(&mut contents).unwrap_or_else(|_| {
            panic!(
                "Failed to read file cfg/{} for bot configuration.",
                filename
            )
        });

        for m in reg.find_iter(&contents) {
            match reg.captures(m.as_str()) {
                None => {}
                Some(c) => {
                    list.push(c["uuid"].to_string());
                }
            }
        }

        self.bots_uuid.append(&mut list);
        Ok(())
    }

    pub fn add_regex_list(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut list: Vec<Regex> = Vec::new();

        let mut file = File::open(filename)?;

        let mut contents: String = String::new();
        file.read_to_string(&mut contents).unwrap_or_else(|_| {
            panic!(
                "Failed to read file cfg/{} for bot configuration.",
                filename
            )
        });

        for line in contents.lines() {
            let txt = line.trim();
            if txt.is_empty() {
                continue;
            }

            list.push(Regex::new(txt)?);
        }

        self.bots_regx.append(&mut list);
        Ok(())
    }
}
