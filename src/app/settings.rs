use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {

    pub user: String,
    pub join_alert: bool,
    pub chat_reminders: bool,
    pub kick: bool,
    pub period: f32,
    pub directory: String,
    // pub key: KeybdKey,

}


impl Settings {

    pub fn new() -> Settings {
        Settings {
            user: String::from("[U:X:XXXXXXX]"),
            join_alert: false,
            chat_reminders: false,
            kick: true,
            period: 10.0,
            directory: String::from(""),
        }
    }

    pub fn import(file: &str) -> Result<Settings, Box<dyn std::error::Error>> {

        return match std::fs::read_to_string(file) {
            Ok(contents) => {
                match serde_json::from_str::<Settings>(&contents) {
                    Ok(set) => {Ok(set)},
                    Err(e) => {Err(Box::new(e))}
                }

            },
            Err(e) => {
                Err(Box::new(e))
            }
        }

    }

    pub fn export(&self, file: &str) -> Result<(), Box<dyn std::error::Error>> {

        return match serde_json::to_string(self) {
            Ok(contents) => {
                match std::fs::write(file, &contents) {
                    Ok(_) => {Ok(())},
                    Err(e) => {Err(Box::new(e))}
                }

            },
            Err(e) => {
                Err(Box::new(e))
            }
        }
    }

}