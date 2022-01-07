#![allow(dead_code)]

use std::collections::HashMap;

pub mod player;
use player::Player;
use player::Team;
use player::State;

use super::commander::Commander;
use super::settings::Settings;

pub const COM_STATUS: &str = "status";
pub const COM_LOBBY: &str = "tf_lobby_debug";

pub struct Server {
    pub players: HashMap<String, Player>,
    // pub com: Commander,
    // pub bot_checker: BotChecker,
    pub new_bots: Vec<(String, Team)>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            players: HashMap::with_capacity(24),
            // bot_checker: BotChecker::new(),
            new_bots: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.players.clear();
        self.new_bots.clear();
    }

    pub fn get_bots(&self) -> Vec<&Player> {
        let mut bots: Vec<&Player> = Vec::new();

        for p in self.players.values().into_iter() {
            if p.bot {
                bots.push(p);
            }
        }

        bots
    }

    /// Call a votekick on any players detected as bots.
    /// If userid is set in cfg/settings.cfg then it will only attempt to call vote on bots in the same team
    /// There is no way of knowing if a vote is in progress or the user is on cooldown so votes will still be attempted
    pub fn kick_bots(&mut self, set: &Settings, com: &mut Commander) {

        if !set.kick {
            return;
        }

        let mut bots: Vec<&Player> = Vec::new();

        for p in self.players.values().into_iter() {
            if p.bot {
                bots.push(p);
            }
        }
        bots = bots
            .into_iter()
            .filter(|p| p.state == State::Active && p.accounted)
            .collect();

        for p in bots {
            match self.players.get(&set.user) {
                Some(user) => {
                    if user.team == p.team {
                        com.kick(p, set);
                    }
                },
                None => {
                    com.kick(p, set);
                }
            }
        }
    }

    /// Print bots to console and send chat message in-game if necessary of current bots
    pub fn announce_bots(&mut self, set: &Settings, com: &mut Commander) {
        let mut bots: Vec<String> = Vec::new();
        let mut new: bool = false;

        // Collect all bots in list bots
        let mut existing_bots: Vec<&Player> = Vec::new();
        for p in self.players.values().into_iter() {
            if p.bot {
                existing_bots.push(p);
            }
        }

        // Remove not-yet-active or unaccounted bots
        existing_bots = existing_bots
            .into_iter()
            .filter(|p| p.state == State::Active && p.accounted)
            .collect();

        //Check for teams
        let mut invaders = false;
        let mut defenders = false;

        if !existing_bots.is_empty() {
            println!("Bots on server: ");
        }
        // Create list of existing bot names/teams on server and list bots
        for p in existing_bots.iter() {
            if p.team == Team::Defenders {
                defenders = true;
            }
            if p.team == Team::Invaders {
                invaders = true;
            }

            bots.push(p.name.clone());
            println!("{}", p);
        }

        // Set to announce joining bots if there are any
        if !self.new_bots.is_empty() && set.join_alert {
            bots.clear();

            invaders = false;
            defenders = false;

            for p in self.new_bots.iter() {
                if p.1 == Team::Defenders {
                    defenders = true;
                }
                if p.1 == Team::Invaders {
                    invaders = true;
                }

                bots.push(p.0.clone());
            }
            self.new_bots.clear();
            new = true;
        }

        // Announce existing bots
        if bots.is_empty() {
            return;
        }

        // Don't bother if there's nothing to announce
        if !(set.chat_reminders || new) {
            return;
        }

        // Construct alert message
        let mut alert: String = String::new();

        // Prefix message with which teams the bots are on/joining
        if new {
            // Set which team they're joining
            if invaders && defenders {
                alert.push_str("BOTS joining both teams: ");
            } else {
                match self.players.get(&set.user) {
                    Some(p) => {
                        if (p.team == Team::Invaders && invaders)
                            || (p.team == Team::Defenders && defenders)
                        {
                            alert.push_str("BOTS joining our team: ");
                        } else {
                            alert.push_str("BOTS joining enemy team: ");
                        }
                    },
                    None => {
                        alert.push_str("BOTS joining: ");
                    }
                }
            }
        } else {
            // Set which team they're on
            if invaders && defenders {
                alert.push_str("BOT Alert: Both teams have BOTS: ");
            } else {
                match self.players.get(&set.user) {
                    Some(p) => {
                        if (p.team == Team::Invaders && invaders)
                            || (p.team == Team::Defenders && defenders)
                        {
                            alert.push_str("BOT Alert: Our team has BOTS: ");
                        } else {
                            alert.push_str("BOT Alert: Enemy team has BOTS: ");
                        }
                    },
                    None => {
                        alert.push_str("BOT Alert: The server has BOTS: ");
                    }
                }
            }
        }

        // List bots
        for p in bots {
            alert.push_str(&format!("{} ", p));
        }

        // Broadcast message
        com.say(&alert, set);
    }

    /// Update local info on server players
    pub fn refresh(&mut self, set: &Settings, com: &mut Commander) {
        println!("Refreshing server.");

        for p in self.players.values_mut().into_iter() {
            p.accounted = false;
        }

        com.run_command(&format!("{}; wait 200; {}; wait 100; echo refreshcomplete", COM_STATUS, COM_LOBBY), &set.key);
    }

    /// Remove players who aren't present on the server anymore
    /// (This method will be called automatically in a rexes command)
    pub fn prune(&mut self, set: &Settings, com: &mut Commander) {
        self.players.retain(|_, v| {
            if !v.accounted && v.bot {
                println!("Bot disconnected: {}", v.name);
            }
            v.accounted
        });

        com.run_command("wait 100; echo prunecomplete", &set.key);
    }
}
