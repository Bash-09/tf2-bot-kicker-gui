#![allow(dead_code)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::server::player::PlayerType;

use super::player::Player;

#[derive(Debug, Serialize)]
pub struct PlayerRecord {
    pub steamid: String,
    pub player_type: PlayerType,
    pub notes: Option<String>,
}

pub struct PlayerChecker {
    pub bots_regx: Vec<Regex>,

    pub players: HashMap<String, PlayerRecord>,
}

impl PlayerChecker {
    pub fn new() -> PlayerChecker {
        PlayerChecker {
            bots_regx: Vec::new(),

            players: HashMap::new(),
        }
    }

    /// Marks a player as a bot based on their name compared to a list of regexes.
    /// If the name matches a bot regex the player will be marked as a bot and
    /// a note appended to them indicating the regex that caught them.
    ///
    /// Returns true if a regex was matched and false otherwise.
    pub fn check_player_name(&mut self, player: &mut Player) -> bool {
        for regx in self.bots_regx.iter() {
            if regx.captures(&player.name).is_some() {
                player.player_type = PlayerType::Bot;

                let note = format!("Matched bot regex: {}", regx.as_str());
                if let Some(notes) = &mut player.notes {
                    notes.push('\n');
                    notes.push_str(&note);
                } else {
                    player.notes = Some(note);
                }

                self.update_player(player);
                return true;
            }
        }
        false
    }

    /// Loads a player's record from the persistent record if it exists and restores
    /// their data. e.g. marking the player as a bot or cheater or just
    pub fn check_player_steamid(&self, player: &mut Player) -> bool {
        if let Some(record) = self.players.get(&player.steamid) {
            player.player_type = record.player_type;
            player.notes = record.notes.clone();

            return true;
        }

        false
    }

    /// Inserts the player into the saved record of players
    pub fn update_player(&mut self, player: &Player) {
        self.players
            .insert(player.steamid.clone(), player.get_record());
    }

    /// Removes the player from the saved record of players
    pub fn remove_player(&mut self, player: &Player) {
        self.players.remove(&player.steamid);
    }

    /// Saves a new regex to match bots against
    pub fn append_regex(&mut self, reg: Regex) {
        self.bots_regx.push(reg);
    }

    /// Import all players' steamID from the provided file as a particular player type
    pub fn read_from_steamid_list(
        &mut self,
        filename: &str,
        as_player_type: PlayerType,
    ) -> Result<(), std::io::Error> {
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
                    let steamid = c["uuid"].to_string();

                    if self.players.contains_key(&steamid) {
                        continue;
                    } else {
                        let record = PlayerRecord {
                            steamid,
                            player_type: as_player_type,
                            notes: Some(format!(
                                "Imported from {} as {:?}",
                                filename, as_player_type
                            )),
                        };
                        self.players.insert(record.steamid.clone(), record);
                    }
                }
            }
        }

        Ok(())
    }

    /// Read a list of regexes to match bots against from a file
    pub fn read_regex_list<P: AsRef<Path>>(
        &mut self,
        filename: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut list: Vec<Regex> = Vec::new();

        let mut file = File::open(filename)?;

        let mut contents: String = String::new();
        file.read_to_string(&mut contents)?;

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

    /// Save the current player record to a file
    pub fn save_players<P: AsRef<Path>>(&self, file: P) -> std::io::Result<()> {
        let players: Vec<&PlayerRecord> = self.players.values().collect();

        match serde_json::to_string(&players) {
            Ok(contents) => std::fs::write(file, &contents)?,
            Err(e) => {
                log::error!("Failed to serialize players: {:?}", e);
            }
        }

        Ok(())
    }

    pub fn read_players<P: AsRef<Path>>(&mut self, file: P) -> Result<(), Box<dyn Error>> {
        let contents = std::fs::read_to_string(file)?;
        let json = json::parse(&contents)?;

        for p in json.members() {
            let steamid = p["steamid"].as_str().unwrap_or("");
            let player_type = p["player_type"].as_str().unwrap_or("");
            let notes = p["notes"].as_str().unwrap_or("");

            if steamid == "" {
                continue;
            }
            let player_type = match player_type {
                "Player" => PlayerType::Player,
                "Bot" => PlayerType::Bot,
                "Cheater" => PlayerType::Cheater,
                _ => continue,
            };

            let notes = if notes.is_empty() {
                None
            } else {
                Some(notes.to_string())
            };

            let record = PlayerRecord {
                steamid: steamid.to_string(),
                player_type,
                notes: notes,
            };

            self.players.insert(steamid.to_string(), record);
        }

        Ok(())
    }
}
