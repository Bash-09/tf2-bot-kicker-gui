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
}
