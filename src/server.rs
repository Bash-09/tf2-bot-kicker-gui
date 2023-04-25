#![allow(dead_code)]

use std::collections::HashMap;

pub mod player;
use player::Player;
use player::PlayerState;

use crate::io::command_manager::CommandManager;
use crate::io::command_manager::KickReason;
use crate::io::IOManager;
use crate::io::IORequest;
use crate::player_checker::PlayerRecord;
use crate::ringbuffer::RingBuffer;

use self::player::PlayerType;
use self::player::Steamid32;
use self::player::Team;

use super::settings::Settings;

pub const COM_STATUS: &str = "status";
pub const COM_LOBBY: &str = "tf_lobby_debug";
const RINGBUFFER_LEN: usize = 48;

pub struct Server {
    players: HashMap<String, Player>,
    pub new_connections: Vec<String>,
    pub pending_lookup: Vec<String>,
    previous_players: RingBuffer<Player>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            players: HashMap::with_capacity(24),
            new_connections: Vec::new(),
            pending_lookup: Vec::new(),
            previous_players: RingBuffer::new(RINGBUFFER_LEN),
        }
    }

    pub fn clear(&mut self) {
        let mut players: HashMap<String, Player> = HashMap::new();
        std::mem::swap(&mut players, &mut self.players);

        'outer: for p in players.into_values() {
            for prev in self.previous_players.inner() {
                if p.steamid32 == prev.steamid32 {
                    continue 'outer;
                }
            }
            self.previous_players.push(p);
        }

        self.new_connections.clear();
    }

    pub fn get_players(&self) -> &HashMap<String, Player> {
        &self.players
    }

    pub fn get_previous_players(&self) -> &RingBuffer<Player> {
        &self.previous_players
    }

    pub fn get_player_mut(&mut self, steamid: &Steamid32) -> Option<&mut Player> {
        self.players.get_mut(steamid)
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.insert(player.steamid32.clone(), player);
    }

    pub fn remove_player(&mut self, steamid32: &Steamid32) {
        if let Some(player) = self.players.remove(steamid32) {
            for prev in self.previous_players.inner() {
                if prev.steamid32 == player.steamid32 {
                    return;
                }
            }
            self.previous_players.push(player);
        }
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

    /// Updating any existing copies of a player (in current players or recent players) to match
    /// the provided PlayerRecord
    pub fn update_player_from_record(&mut self, record: PlayerRecord) {
        for p in self.previous_players.inner_mut() {
            if p.steamid32 == record.steamid {
                p.player_type = record.player_type;
                p.notes = record.notes.clone();
            }
        }

        if let Some(p) = self.players.get_mut(&record.steamid) {
            p.player_type = record.player_type;
            p.notes = record.notes;
        }
    }

    /// Call a votekick on any players detected as bots.
    /// If userid is set in cfg/settings.cfg then it will only attempt to call vote on bots in the same team
    /// There is no way of knowing if a vote is in progress or the user is on cooldown so votes will still be attempted
    pub fn kick_players_of_type(
        &mut self,
        settings: &Settings,
        io: &mut IOManager,
        player_type: PlayerType,
    ) {
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
                        io.send(IORequest::RunCommand(CommandManager::kick_player_command(
                            &p.userid,
                            KickReason::Cheating,
                        )));
                    }
                }
                None => {
                    io.send(IORequest::RunCommand(CommandManager::kick_player_command(
                        &p.userid,
                        KickReason::Cheating,
                    )));
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
        'outer: for (_, p) in self.players.drain_filter(|_, v| {
            if !v.accounted && v.player_type == PlayerType::Bot {
                log::info!("Bot disconnected: {}", v.name);
            }
            if !v.accounted {
                log::debug!("Player Pruned: {}", v.name);
            }

            !v.accounted
        }) {
            log::info!("Pruning player {}", &p.name);
            for prev in self.previous_players.inner() {
                if p.steamid32 == prev.steamid32 {
                    continue 'outer;
                }
            }
            self.previous_players.push(p);
        }
    }

    pub fn send_chat_messages(&mut self, settings: &Settings, io: &mut IOManager) {
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

        if bots && cheaters {
            message.push_str(&format!("{} ", settings.message_both.trim()));
        } else if bots {
            message.push_str(&format!("{} ", settings.message_bots.trim()));
        } else if cheaters {
            message.push_str(&format!("{} ", settings.message_cheaters.trim()));
        }

        // Team
        match self.players.get(&settings.user) {
            Some(user) => {
                if (invaders && defenders) || user.team == Team::None {
                    message.push_str(&format!("{} ", settings.message_both_teams.trim()));
                } else if (invaders && user.team == Team::Invaders)
                    || (defenders && user.team == Team::Defenders)
                {
                    message.push_str(&format!("{} ", settings.message_same_team.trim()));
                } else if (invaders && user.team == Team::Defenders)
                    || (defenders && user.team == Team::Invaders)
                {
                    message.push_str(&format!("{} ", settings.message_enemy_team.trim()));
                } else {
                    message.push_str(&format!("{} ", settings.message_default.trim()));
                    log::error!("Announcing bot that doesn't have a team.");
                }
            }
            None => {
                message.push_str(&format!("{} ", settings.message_default.trim()));
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
        io.send(IORequest::RunCommand(CommandManager::send_chat_command(
            &message,
        )));
    }

    /// Create and add a demo player to the server list to test with
    pub fn add_demo_player(&mut self, name: String, steamid32: String, team: Team) {
        let player = player::create_demo_player(name, steamid32, team);
        self.players.insert(player.steamid32.clone(), player);
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
