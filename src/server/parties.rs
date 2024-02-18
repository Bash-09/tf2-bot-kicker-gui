use std::collections::{HashMap, HashSet, VecDeque};

use crate::player::{steamid_64_to_32, Player, Steamid32};

// taken from https://sashamaps.net/docs/resources/20-colors/
const COLOR_PALETTE: [egui::Color32; 21] = [
    egui::Color32::from_rgb(230, 25, 75),
    egui::Color32::from_rgb(60, 180, 75),
    egui::Color32::from_rgb(255, 225, 25),
    egui::Color32::from_rgb(0, 130, 200),
    egui::Color32::from_rgb(245, 130, 48),
    egui::Color32::from_rgb(145, 30, 180),
    egui::Color32::from_rgb(70, 240, 240),
    egui::Color32::from_rgb(240, 50, 230),
    egui::Color32::from_rgb(210, 245, 60),
    egui::Color32::from_rgb(250, 190, 212),
    egui::Color32::from_rgb(0, 128, 128),
    egui::Color32::from_rgb(220, 190, 255),
    egui::Color32::from_rgb(170, 110, 40),
    egui::Color32::from_rgb(255, 250, 200),
    egui::Color32::from_rgb(128, 0, 0),
    egui::Color32::from_rgb(170, 255, 195),
    egui::Color32::from_rgb(128, 128, 0),
    egui::Color32::from_rgb(255, 215, 180),
    egui::Color32::from_rgb(0, 0, 128),
    egui::Color32::from_rgb(128, 128, 128),
    egui::Color32::from_rgb(255, 255, 255),
];

/// Structure used to determine which players in the current server are friends
pub struct Parties{
    players: Vec<Steamid32>,
    friend_connections: HashMap<Steamid32, HashSet<Steamid32>>,

    parties: Vec<HashSet<Steamid32>>,
}

impl Parties {
    pub fn new() -> Parties {
        Parties{
            players: Vec::new(),
            friend_connections: HashMap::new(),
            parties: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.players.clear();
        self.friend_connections.clear();
        self.parties.clear();
    }

    /// Updates the internal graph of players
    pub fn update(&mut self, player_map: &HashMap<Steamid32, Player>){
        // Copy over the players
        self.players.clear();
        for p in player_map.keys() {
            self.players.push(p.clone());
        }

        self.friend_connections.clear();
        // Get friends of each player and add them to the connection map
        for p in player_map.values() {
            if let Some(Ok(acif)) = &p.account_info {
                if let Some(Ok(friends)) = &acif.friends {
                    for f in friends {
                        let id = steamid_64_to_32(&f.steamid).unwrap();
                        if self.players.contains(&id){
                            self.add_friend(&p.steamid32, &id);
                        }
                    }
                }
            }
        }

        self.find_parties();
    }

    /// Returns color to represent this player's party
    pub fn get_player_party_color(&self, p: &Player) -> Option<egui::Color32> {
        for i in 0..self.parties.len(){
            let party = self.parties.get(i).unwrap();
            if party.contains(&p.steamid32){
                return Some(COLOR_PALETTE[(i%COLOR_PALETTE.len()) as usize]);
            }
        }
        None
    }

    /// Determines the connected components of the player graph (aka. the friend groups)
    fn find_parties(&mut self){
        self.parties.clear();
        if self.players.is_empty() {
            return;
        }

        let mut remaining_players = self.players.clone(); // Vec to keep track of unhandled players
        let mut queue: VecDeque<Steamid32> = VecDeque::new(); // Queue for processing connected players

        // Perform a BFS over the graph to find the components and save them as parties
        while !remaining_players.is_empty() {
            // Start a new party and add an unhandled player to the queue
            queue.push_back(remaining_players.first().unwrap().clone());
            let mut party: HashSet<Steamid32> = HashSet::new();

            while !queue.is_empty() {
                let p = queue.pop_front().unwrap();
                party.insert(p.clone());
                remaining_players.retain(|rp|*rp != p);
                
                if let Some(friends) = self.friend_connections.get(&p) {
                    // Only push players not in the party into the queue
                    friends.iter().filter(|f|!party.contains(*f)).for_each(|f|queue.push_back(f.clone()));
                }
            }
            // Solo players are not in a party
            if party.len() > 1{
                self.parties.push(party);
            }
        }
    }

    /// Utility function to add bidirectional friend connections, so private accounts can also be accounted for as long as one of their friends has a public account
    fn add_friend(&mut self, user: &String, friend: &String){
        if let Some(set) = self.friend_connections.get_mut(user){
            set.insert(friend.clone());
        } else {
            self.friend_connections.insert(user.clone(), HashSet::from([friend.clone()]));
        }

        if let Some(set) = self.friend_connections.get_mut(friend){
            set.insert(user.clone());
        } else {
            self.friend_connections.insert(friend.clone(), HashSet::from([user.clone()]));
        }
    }
}