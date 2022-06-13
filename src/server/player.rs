use core::fmt;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Team {
    Defenders,
    Invaders,
    None,
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out: &str;
        match self {
            Team::Defenders => out = "DEF ",
            Team::Invaders => out = "INV ",
            Team::None => out = "NONE",
        }
        write!(f, "{}", out)
    }
}

#[derive(PartialEq, Eq)]
pub enum State {
    Spawning,
    Active,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out: &str;
        match self {
            State::Active => out = "Active  ",
            State::Spawning => out = "Spawning",
        }
        write!(f, "{}", out)
    }
}

pub struct Player {
    pub userid: String,
    pub name: String,
    pub steamid: String,
    pub known_steamid: bool,
    pub time: u32,
    pub team: Team,
    pub state: State,
    pub bot: bool,
    pub accounted: bool,
    pub new_connection: bool,
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut bot = "No";
        if self.bot {
            bot = "Yes";
        }
        write!(
            f,
            "{} - {}, \tUID: {}, SteamID: {}, State: {}, Bot: {}",
            self.team, self.name, self.userid, self.steamid, self.state, bot
        )
    }
}

impl Player {
    pub fn get_export_steamid(&self) -> String {
        format!("[{}] - {}", &self.steamid, &self.name)
    }

    pub fn get_export_regex(&self) -> String {
        regex::escape(&self.name)
    }
}
