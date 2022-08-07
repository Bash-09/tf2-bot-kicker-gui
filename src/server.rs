#![allow(dead_code)]

use std::collections::HashMap;

pub mod player;
use player::Player;
use player::PlayerState;

use crate::command_manager::CommandManager;
use crate::ringbuffer::RingBuffer;

use self::player::PlayerType;
use self::player::Team;

use super::settings::Settings;

pub const COM_STATUS: &str = "status";
pub const COM_LOBBY: &str = "tf_lobby_debug";

pub struct Server {
    pub players: HashMap<String, Player>,
    pub new_connections: Vec<String>,
    pub previous_players: RingBuffer<Player>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            players: HashMap::with_capacity(24),
            new_connections: Vec::new(),
            previous_players: RingBuffer::new(48),
        }
    }

    pub fn clear(&mut self) {
        self.players.clear();
        self.new_connections.clear();
    }

    pub fn get_bots(&self) -> Vec<&Player> {
        let mut bots: Vec<&Player> = Vec::new();

        for p in self.players.values() {
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
        settings: &Settings,
        cmd: &mut CommandManager,
        player_type: PlayerType,
    ) {
        if cmd.connected(&settings.rcon_password).is_err() {
            return;
        }

        if !settings.kick_bots {
            return;
        }

        // Don't attempt to kick if too early
        if let Some(user) = self.players.get(&settings.user) {
            if user.time < 120 {
                return;
            }
        }

        for p in self.players.values() {
            if p.state != PlayerState::Active || !p.accounted || p.player_type != player_type {
                continue;
            }
            match self.players.get(&settings.user) {
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

        for p in self.players.values_mut() {
            p.accounted = false;
        }
    }

    /// Remove players who aren't present on the server anymore
    /// (This method will be called automatically in a rexes command)
    pub fn prune(&mut self) {
        for (_, p) in self.players.drain_filter(|_, v| {
            if !v.accounted && v.player_type == PlayerType::Bot {
                log::info!("Bot disconnected: {}", v.name);
            }
            if !v.accounted {
                log::debug!("Player Pruned: {}", v.name);
            }

            !v.accounted
        }) {
            log::info!("Pruning player {}", &p.name);
            self.previous_players.push(p);
        }
    }

    pub fn send_chat_messages(&mut self, settings: &Settings, cmd: &mut CommandManager) {

        let mut message = String::new();

        let mut bots = false;
        let mut cheaters = false;
        let mut names: Vec<&str> = Vec::new();

        let mut invaders = false;
        let mut defenders = false;

        // Remove accounts we don't want to announce, record the details of accounts we want to
        // announce now, and leave the rest for later
        self.new_connections.retain(|p| {
            if let Some(p) = self.players.get(p) {
                // Make sure it's a bot or cheater
                if !(settings.announce_bots && p.player_type == PlayerType::Bot
                    || settings.announce_cheaters && p.player_type == PlayerType::Cheater)
                {
                    return false;
                }

                // Don't announce common names
                if settings.dont_announce_common_names && p.common_name {
                    return false;
                }

                // Ignore accounts that haven't been assigned a team yet
                if p.team == Team::None {
                    return true;
                }

                // Record details of account for announcement
                bots |= p.player_type == PlayerType::Bot;
                cheaters |= p.player_type == PlayerType::Cheater;
                invaders |= p.team == Team::Invaders;
                defenders |= p.team == Team::Defenders;
                names.push(&p.name);
            }
            false
        });

        if names.is_empty() {
            return;
        }

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
                } else if (invaders && user.team == Team::Invaders)
                    || (defenders && user.team == Team::Defenders)
                {
                    message.push_str("our team: ");
                } else if (invaders && user.team == Team::Defenders)
                    || (defenders && user.team == Team::Invaders) 
                {
                    message.push_str("the enemy team: ");
                } else {
                    message.push_str("the server: ");
                    log::error!("Announcing bot that doesn't have a team.");
                }
            }
            None => {
                message.push_str("the server: ");
            }
        }

        // Player names
        let mut account_peekable = names.iter().peekable();
        while let Some(name) = account_peekable.next() {
            message.push_str(name);

            if account_peekable.peek().is_some() {
                message.push_str(", ");
            } else {
                message.push('.');
            }
        }

        // Send message
        cmd.send_chat(&message);
        self.new_connections.clear();
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
