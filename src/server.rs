#![allow(dead_code)]

use std::collections::HashMap;

pub mod player;
use player::Player;
use player::PlayerState;

use crate::command_manager::CommandManager;

use self::player::PlayerType;
use self::player::Team;

use super::settings::Settings;

pub const COM_STATUS: &str = "status";
pub const COM_LOBBY: &str = "tf_lobby_debug";

pub struct Server {
    pub players: HashMap<String, Player>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            players: HashMap::with_capacity(24),
        }
    }

    pub fn clear(&mut self) {
        self.players.clear();
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

        if !set.kick_bots {
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

    pub fn send_chat_messages(&self, settings: &Settings, cmd: &mut CommandManager) {
        if !settings.announce_bots && !settings.announce_cheaters {
            return;
        }

        let mut message = String::new();

        let mut bots = false;
        let mut cheaters = false;

        let mut invaders = false;
        let mut defenders = false;

        // Get all illegitimate accounts
        let mut accounts: Vec<&Player> = Vec::new();
        for p in self.players.values().into_iter() {
            if p.time > settings.alert_period as u32 {continue}

            match p.player_type {
                PlayerType::Player => continue,
                PlayerType::Bot => {
                    if !settings.announce_bots {continue}
                    accounts.push(p);
                    bots = true;
                    invaders |= p.team == Team::Invaders;
                    defenders |= p.team == Team::Defenders;
                },
                PlayerType::Cheater => {
                    if !settings.announce_cheaters {continue}
                    accounts.push(p);
                    cheaters = true;
                    invaders |= p.team == Team::Invaders;
                    defenders |= p.team == Team::Defenders;
                }
            }
        }

        if accounts.is_empty() {return}

        // Players joining
        if bots && cheaters {
            message.push_str("Bots and Cheaters joining ");
        } else if bots {
            message.push_str("Bots joining ");
        } else if cheaters {
            message.push_str("Cheaters joining ");
        }

        // Team
        match self.players.get(&settings.user) {
            Some(user) => {
                if (invaders && defenders) || user.team == Team::None {
                    message.push_str("the server: ");
                } else if (invaders && user.team == Team::Invaders) || (defenders && user.team == Team::Defenders) {
                    message.push_str("our team: ");
                } else {
                    message.push_str("the enemy team: ");
                }
            },
            None => {
                message.push_str("the server: ");
            }
        }

        // Player names
        let mut account_peekable = accounts.into_iter().peekable();
        while let Some(account) = account_peekable.next() {
            log::debug!("Bot time: {}", account.time);

            message.push_str(&account.name);

            if account_peekable.peek().is_some() {
                message.push_str(", ");
            } else {
                message.push('.');
            }
        }

        // Send message
        cmd.send_chat(&message);
    }
}
