use std::collections::{HashMap, HashSet, VecDeque};

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::player::{steamid_64_to_32, Player, Steamid32};

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
        //self.friend_connections.clear();
        self.parties.clear();
    }

    pub fn update(&mut self, player_map: &HashMap<Steamid32, Player>){
        self.players.clear();
        for p in player_map.keys() {
            self.players.push(p.clone());
        }

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

    fn find_parties(&mut self){
        self.parties.clear();
        if self.players.is_empty() {
            return;
        }
        let mut remaining_players = self.players.clone();

        let mut handled: HashSet<Steamid32> = HashSet::new();
        let mut queue: VecDeque<Steamid32> = VecDeque::new();

        while !remaining_players.is_empty() {
            queue.push_back(remaining_players.pop().unwrap());
            let mut party: HashSet<Steamid32> = HashSet::new();

            while !queue.is_empty() {
                let p = queue.pop_front().unwrap();
                party.insert(p.clone());
                handled.insert(p.clone());
                
                if let Some(friends) = self.friend_connections.get(&p) {
                    friends.iter().filter(|f|!handled.contains(f.clone())).for_each(|f|queue.push_back(f.clone()));
                }
            }
            if party.len() > 1{
                self.parties.push(party);
            }
        }
    }

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

    pub fn get_parties(&self) -> &Vec<HashSet<Steamid32>> {
        &self.parties
    }

    pub fn get_player_party_color(&self, p: &Player) -> Option<egui::Color32> {
        for i in 0..self.parties.len(){
            let party = self.parties.get(i).unwrap();
            if party.contains(&p.steamid32){
                return Some(COLOR_PALETTE[(i%COLOR_PALETTE.len()) as usize]);
            }
        }
        None
    }
}