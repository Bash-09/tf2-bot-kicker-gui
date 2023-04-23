use json::JsonValue;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub window: WindowState,

    pub user: String,
    pub steamapi_key: String,

    pub announce_bots: bool,
    pub announce_cheaters: bool,
    pub announce_namesteal: bool,
    pub dont_announce_common_names: bool,

    pub message_bots: String,
    pub message_cheaters: String,
    pub message_both: String,

    pub message_same_team: String,
    pub message_enemy_team: String,
    pub message_both_teams: String,
    pub message_default: String,

    pub kick_bots: bool,
    pub kick_cheaters: bool,

    pub refresh_period: f32,
    pub kick_period: f32,
    pub alert_period: f32,

    pub paused: bool,

    pub rcon_password: String,
    pub tf2_directory: String,

    pub mark_name_stealers: bool,

    pub ignore_version: String,
    pub ignore_no_api_key: bool,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            window: WindowState {
                width: 1100,
                height: 500,
                x: 200,
                y: 200,
            },

            user: String::from("U:1:XXXXXXX"),
            steamapi_key: String::new(),

            announce_bots: false,
            announce_cheaters: false,
            announce_namesteal: true,
            dont_announce_common_names: true,

            message_bots: String::from("Bots joining"),
            message_cheaters: String::from("Cheaters joining"),
            message_both: String::from("Bots and Cheaters joining"),

            message_same_team: String::from("our team:"),
            message_enemy_team: String::from("the enemy team:"),
            message_both_teams: String::from("both teams:"),
            message_default: String::from("the server:"),

            kick_bots: true,
            kick_cheaters: false,

            refresh_period: 10.0,
            kick_period: 10.0,
            alert_period: 20.0,

            paused: false,

            rcon_password: String::from("tf2bk"),
            tf2_directory: String::new(),

            mark_name_stealers: true,
            ignore_version: String::new(),
            ignore_no_api_key: false,
        }
    }

    /// Attempts to import settings from a file, returning an error if there is no file or it could not be read and interpretted
    ///
    /// A default settings instance is created and each setting overridden individually if it can be read from the JSON object
    /// and ignored if not. This is to make the importer resilient to version changes such as when a new version introduces
    /// a new setting or changes/removes and old one and the struct cannot be directly deserialised from the JSON anymore.
    pub fn import(file: &str) -> Result<Settings, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(file)?;
        let json = json::parse(&contents)?;

        let mut set = Settings::new();

        if let JsonValue::Object(window) = &json["window"] {
            if let Some(width) = window["width"].as_i32() {
                set.window.width = width.try_into().unwrap_or(set.window.width);
            }
            if let Some(height) = window["height"].as_i32() {
                set.window.height = height.try_into().unwrap_or(set.window.height);
            }
            set.window.x = window["x"].as_i32().unwrap_or(set.window.x);
            set.window.y = window["y"].as_i32().unwrap_or(set.window.y);
        }

        set.user = json["user"].as_str().unwrap_or(&set.user).to_string();
        set.steamapi_key = json["steamapi_key"]
            .as_str()
            .unwrap_or(&set.steamapi_key)
            .to_string();

        set.announce_bots = json["announce_bots"].as_bool().unwrap_or(set.announce_bots);
        set.announce_cheaters = json["announce_cheaters"]
            .as_bool()
            .unwrap_or(set.announce_cheaters);
        set.announce_namesteal = json["announce_namesteal"]
            .as_bool()
            .unwrap_or(set.announce_namesteal);
        set.dont_announce_common_names = json["dont_announce_common_names"]
            .as_bool()
            .unwrap_or(set.dont_announce_common_names);

        set.message_bots = json["message_bots"]
            .as_str()
            .unwrap_or(&set.message_bots)
            .to_string();
        set.message_cheaters = json["message_cheaters"]
            .as_str()
            .unwrap_or(&set.message_cheaters)
            .to_string();
        set.message_both = json["message_both"]
            .as_str()
            .unwrap_or(&set.message_both)
            .to_string();

        set.message_same_team = json["message_same_team"]
            .as_str()
            .unwrap_or(&set.message_same_team)
            .to_string();
        set.message_enemy_team = json["message_enemy_team"]
            .as_str()
            .unwrap_or(&set.message_enemy_team)
            .to_string();
        set.message_both_teams = json["message_both_teams"]
            .as_str()
            .unwrap_or(&set.message_both_teams)
            .to_string();
        set.message_default = json["message_default"]
            .as_str()
            .unwrap_or(&set.message_default)
            .to_string();

        set.kick_bots = json["kick_bots"].as_bool().unwrap_or(set.kick_bots);
        set.kick_cheaters = json["kick_cheaters"].as_bool().unwrap_or(set.kick_cheaters);

        set.refresh_period = json["refresh_period"]
            .as_f32()
            .unwrap_or(set.refresh_period);
        set.kick_period = json["kick_period"].as_f32().unwrap_or(set.kick_period);
        set.alert_period = json["alert_period"].as_f32().unwrap_or(set.alert_period);

        set.paused = json["paused"].as_bool().unwrap_or(set.paused);

        set.rcon_password = json["rcon_password"]
            .as_str()
            .unwrap_or(&set.rcon_password)
            .to_string();
        set.tf2_directory = json["tf2_directory"]
            .as_str()
            .unwrap_or(&set.tf2_directory)
            .to_string();
        set.ignore_version = json["ignore_version"]
            .as_str()
            .unwrap_or(&set.ignore_version)
            .to_string();

        set.mark_name_stealers = json["mark_name_stealers"]
            .as_bool()
            .unwrap_or(set.mark_name_stealers);

        set.ignore_no_api_key = json["ignore_no_api_key"]
            .as_bool()
            .unwrap_or(set.ignore_no_api_key);

        Ok(set)
    }

    /// Directly serializes the object to JSON and attempts to write it to the specified file.
    pub fn export(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _new_dir = std::fs::create_dir("cfg");
        match serde_json::to_string(self) {
            Ok(contents) => match std::fs::write("cfg/settings.json", &contents) {
                Ok(_) => Ok(()),
                Err(e) => Err(Box::new(e)),
            },
            Err(e) => Err(Box::new(e)),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}
