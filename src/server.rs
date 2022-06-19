#![allow(dead_code)]

use std::collections::HashMap;

pub mod player;
use player::Player;
use player::PlayerState;
use player::Team;

use crate::command_manager::CommandManager;

use self::player::PlayerType;

use super::settings::Settings;

pub const COM_STATUS: &str = "status";
pub const COM_LOBBY: &str = "tf_lobby_debug";

pub struct Server {
    pub players: HashMap<String, Player>,
    pub new_bots: Vec<(String, Team)>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            players: HashMap::with_capacity(24),
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
            if p.player_type == PlayerType::Bot {
                bots.push(p);
            }
        }

        bots
    }

    /// Call a votekick on any players detected as bots.
    /// If userid is set in cfg/settings.cfg then it will only attempt to call vote on bots in the same team
    /// There is no way of knowing if a vote is in progress or the user is on cooldown so votes will still be attempted
    pub fn kick_players_of_type(
        &mut self,
        set: &Settings,
        cmd: &mut CommandManager,
        player_type: PlayerType,
    ) {
        if cmd.connected(&set.rcon_password).is_err() {
            return;
        }

        if !set.kick {
            return;
        }

        for p in self.players.values().into_iter() {
            if p.state != PlayerState::Active || !p.accounted || p.player_type != player_type {
                continue;
            }
            match self.players.get(&set.user) {
                Some(user) => {
                    if user.team == p.team {
                        cmd.kick_player(&p.userid);
                    }
                }
                None => {
                    cmd.kick_player(&p.userid);
                }
            }
        }
    }

    // /// Print bots to console and send chat message in-game if necessary of current bots
    // pub async fn announce_bots(&mut self, set: &Settings, rcon: &mut Connection<TcpStream>) {
    //     if !set.join_alert && !set.chat_reminders {
    //         return;
    //     }

    //     let mut bots: Vec<String> = Vec::new();
    //     let mut new: bool = false;

    //     // Collect all bots in list bots
    //     let mut existing_bots: Vec<&Player> = Vec::new();
    //     for p in self.players.values().into_iter() {
    //         if p.playerType == PlayerType::Bot {
    //             existing_bots.push(p);
    //         }
    //     }

    //     // Remove not-yet-active or unaccounted bots
    //     existing_bots = existing_bots
    //         .into_iter()
    //         .filter(|p| p.state == PlayState::Active && p.accounted)
    //         .collect();

    //     //Check for teams
    //     let mut invaders = false;
    //     let mut defenders = false;

    //     // Create list of existing bot names/teams on server and list bots
    //     for p in existing_bots.iter() {
    //         if p.team == Team::Defenders {
    //             defenders = true;
    //         }
    //         if p.team == Team::Invaders {
    //             invaders = true;
    //         }

    //         bots.push(p.name.clone());
    //     }

    //     // Set to announce joining bots if there are any
    //     if !self.new_bots.is_empty() && set.join_alert {
    //         bots.clear();

    //         invaders = false;
    //         defenders = false;

    //         for p in self.new_bots.iter() {
    //             if p.1 == Team::Defenders {
    //                 defenders = true;
    //             }
    //             if p.1 == Team::Invaders {
    //                 invaders = true;
    //             }

    //             bots.push(p.0.clone());
    //         }
    //         self.new_bots.clear();
    //         new = true;
    //     } else {
    //         self.new_bots.clear();
    //     }

    //     // Announce existing bots
    //     if bots.is_empty() {
    //         return;
    //     }

    //     // Don't bother if there's nothing to announce
    //     if !(set.chat_reminders || new) {
    //         return;
    //     }

    //     // Construct alert message
    //     let mut alert: String = String::new();

    //     // Prefix message with which teams the bots are on/joining
    //     if new && set.join_alert {
    //         // Set which team they're joining
    //         if invaders && defenders {
    //             alert.push_str("Cheaters joining both teams: ");
    //         } else {
    //             match self.players.get(&set.user) {
    //                 Some(p) => {
    //                     if (p.team == Team::Invaders && invaders)
    //                         || (p.team == Team::Defenders && defenders)
    //                     {
    //                         alert.push_str("Cheaters joining our team: ");
    //                     } else {
    //                         alert.push_str("Cheaters joining enemy team: ");
    //         bots to console and send chat message in-game if necessary of current bots
    // pub async fn announce_bots(&mut self, set: &Settings, rcon: &mut Connection<TcpStream>) {
    //     if !set.join_alert && !set.chat_reminders {
    //         return;
    //     }

    //     let mut bots: Vec<String> = Vec::new();
    //     let mut new: bool = false;

    //     // Collect all bots in list bots
    //     let mut existing_bots: Vec<&Player> = Vec::new();
    //     for p in self.players.values().into_iter() {
    //         if p.playerType == PlayerType::Bot {
    //             existing_bots.push(p);
    //         }
    //     }

    //     // Remove not-yet-active or unaccounted bots
    //     existing_bots = existing_bots
    //         .into_iter()
    //         .filter(|p| p.state == PlayState::Active && p.accounted)
    //         .collect();

    //     //Check for teams
    //     let mut invaders = false;
    //     let mut defenders = false;

    //     // Create list of existing bot names/teams on server and list bots
    //     for p in existing_bots.iter() {
    //         if p.team == Team::Defenders {
    //             defenders = true;
    //         }
    //         if p.team == Team::Invaders {
    //             invaders = true;
    //         }

    //         bots.push(p.name.clone());
    //     }

    //     // Set to announce joining bots if there are any
    //     if !self.new_bots.is_empty() && set.join_alert {
    //         bots.clear();

    //         invaders = false;
    //         defenders = false;

    //         for p in self.new_bots.iter() {
    //             if p.1 == Team::Defenders {
    //                 defenders = true;
    //             }
    //             if p.1 == Team::Invaders {
    //                 invaders = true;
    //             }

    //             bots.push(p.0.clone());
    //         }
    //         self.new_bots.clear();
    //         new = true;
    //     } else {
    //         self.new_bots.clear();
    //     }

    //     // Announce existing bots
    //     if bots.is_empty() {
    //         return;
    //     }

    //     // Don't bother if there's nothing to announce
    //     if !(set.chat_reminders || new) {
    //         return;
    //     }

    //     // Construct alert message
    //     let mut alert: String = String::new();

    //     // Prefix message with which teams the bots are on/joining
    //     if new && set.join_alert {
    //         // Set which team they're joining
    //         if invaders && defenders {
    //             alert.push_str("Cheaters joining both teams: ");
    //         } else {
    //        // Set which team they're on
    //         if invaders && defenders {
    //             alert.push_str("Both teams have Cheaters: ");
    //         } else {
    //             match self.players.get(&set.user) {
    //                 Some(p) => {
    //                     if (p.team == Team::Invaders && invaders)
    //                         || (p.team == Team::Defenders && defenders)
    //                     {
    //                         alert.push_str("Cheaters on our team: ");
    //                     } else {
    //                         alert.push_str("Cheaters on enemy team: ");
    //                     }
    //                 }
    //                 None => {
    //                     alert.push_str("Cheaters on this server: ");
    //                 }
    //             }
    //         }
    //     }

    //     // List bots
    //     for p in bots {
    //         alert.push_str(&format!("{} ", p));
    //     }

    //     // Broadcast message
    //     let _cmd = rcon.cmd(&format!("say \"{}\"", alert)).await;
    // }

    /// Update local info on server players
    pub fn refresh(&mut self) {
        log::debug!("Refreshing server.");

        for p in self.players.values_mut().into_iter() {
            p.accounted = false;
        }
    }

    /// Remove players who aren't present on the server anymore
    /// (This method will be called automatically in a rexes command)
    pub fn prune(&mut self) {
        self.players.retain(|_, v| {
            if !v.accounted && v.player_type == PlayerType::Bot {
                log::info!("Bot disconnected: {}", v.name);
            }
            if !v.accounted {
                log::debug!("Player Pruned: {}", v.name);
            }

            v.accounted
        });
    }
}
