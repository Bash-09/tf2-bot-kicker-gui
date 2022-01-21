use inputbot::KeybdKey;
use serde::{Deserialize, Serialize};

pub const DEFAULT_REGEX_LIST: &str = "regx.txt";
pub const DEFAULT_STEAMID_LIST: &str = "steamids.txt";

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {

    pub user: String,
    pub join_alert: bool,
    pub chat_reminders: bool,
    pub kick: bool,

    pub refresh_period: f32,
    pub kick_period: f32,
    pub alert_period: f32,

    pub rcon_password: String,
    pub tf2_directory: String,

    pub record_steamids: bool,

    pub uuid_lists: Vec<String>,
    pub regex_lists: Vec<String>,

}

impl Settings {

    pub fn new() -> Settings {
        Settings {
            user: String::from("U:1:XXXXXXX"),
            join_alert: false,
            chat_reminders: false,
            kick: true,

            refresh_period: 5.0,
            kick_period: 10.0,
            alert_period: 20.0,

            rcon_password: String::from("tf2bk"),
            tf2_directory: String::new(),

            record_steamids: true,

            uuid_lists: vec![format!("cfg/{}", DEFAULT_STEAMID_LIST)],
            regex_lists: vec![format!("cfg/{}", DEFAULT_REGEX_LIST)],
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

pub fn key_to_str(key: KeybdKey) -> &'static str {
    match key {
        KeybdKey::BackspaceKey => "backspace",
        KeybdKey::TabKey => "tab",
        KeybdKey::EnterKey => "enter",
        KeybdKey::EscapeKey => "escape",
        KeybdKey::SpaceKey => "space",
        KeybdKey::HomeKey => "home",
        KeybdKey::LeftKey => "leftarrow",
        KeybdKey::UpKey => "uparrow",
        KeybdKey::DownKey => "downarrow",
        KeybdKey::RightKey => "rightarrow",
        KeybdKey::InsertKey => "insert",
        KeybdKey::DeleteKey => "delete",
        KeybdKey::Numpad0Key => "kp_ins",
        KeybdKey::Numpad1Key => "kp_end",
        KeybdKey::Numpad2Key => "kp_downarrow",
        KeybdKey::Numpad3Key => "kp_pgdn",
        KeybdKey::Numpad4Key => "kp_leftarrow",
        KeybdKey::Numpad5Key => "kp_5",
        KeybdKey::Numpad6Key => "kp_rightarrow",
        KeybdKey::Numpad7Key => "kp_home",
        KeybdKey::Numpad8Key => "kp_uparrow",
        KeybdKey::Numpad9Key => "kp_pgup",
        KeybdKey::AKey => "A",
        KeybdKey::BKey => "B",
        KeybdKey::CKey => "C",
        KeybdKey::DKey => "D",
        KeybdKey::EKey => "E",
        KeybdKey::FKey => "F",
        KeybdKey::GKey => "G",
        KeybdKey::HKey => "H",
        KeybdKey::IKey => "I",
        KeybdKey::JKey => "J",
        KeybdKey::KKey => "K",
        KeybdKey::LKey => "L",
        KeybdKey::MKey => "M",
        KeybdKey::NKey => "N",
        KeybdKey::OKey => "O",
        KeybdKey::PKey => "P",
        KeybdKey::QKey => "Q",
        KeybdKey::RKey => "R",
        KeybdKey::SKey => "S",
        KeybdKey::TKey => "T",
        KeybdKey::UKey => "U",
        KeybdKey::VKey => "V",
        KeybdKey::WKey => "W",
        KeybdKey::XKey => "X",
        KeybdKey::YKey => "Y",
        KeybdKey::ZKey => "Z",
        KeybdKey::F1Key => "F1",
        KeybdKey::F2Key => "F2",
        KeybdKey::F3Key => "F3",
        KeybdKey::F4Key => "F4",
        KeybdKey::F5Key => "F5",
        KeybdKey::F6Key => "F6",
        KeybdKey::F7Key => "F7",
        KeybdKey::F8Key => "F8",
        KeybdKey::F9Key => "F9",
        KeybdKey::F10Key => "F10",
        KeybdKey::F11Key => "F11",
        KeybdKey::F12Key => "F12",
        KeybdKey::NumLockKey => "numlock",
        KeybdKey::ScrollLockKey => "scrolllock",
        KeybdKey::CapsLockKey => "capslock",
        KeybdKey::LShiftKey => "shift",
        _ => "F8"
    }
}

pub fn str_to_key(str: &str) -> KeybdKey {
    match str.to_ascii_lowercase().trim() {
        "backspace" => KeybdKey::BackspaceKey,
        "tab" => KeybdKey::TabKey,
        "enter" => KeybdKey::EnterKey,
        "escape" => KeybdKey::EscapeKey,
        "space" => KeybdKey::SpaceKey,
        "home" => KeybdKey::HomeKey,
        "left" | "leftarrow" => KeybdKey::LeftKey,
        "up" | "uparrow" => KeybdKey::UpKey,
        "right" | "rightarrow" => KeybdKey::RightKey,
        "down" | "downarrow" => KeybdKey::DownKey,
        "ins" | "insert" => KeybdKey::InsertKey,
        "del" | "delete" => KeybdKey::DeleteKey,
        "np0" | "numpad0" | "kp_ins" => KeybdKey::Numpad0Key,
        "np1" | "numpad1" | "kp_end" => KeybdKey::Numpad1Key,
        "np2" | "numpad2" | "kp_downarrow" => KeybdKey::Numpad2Key,
        "np3" | "numpad3" | "kp_pgdn" => KeybdKey::Numpad3Key,
        "np4" | "numpad4" | "kp_leftarrow" => KeybdKey::Numpad4Key,
        "np5" | "numpad5" | "kp_5" => KeybdKey::Numpad5Key,
        "np6" | "numpad6" | "kp_rightarrow" => KeybdKey::Numpad6Key,
        "np7" | "numpad7" | "kp_home" => KeybdKey::Numpad7Key,
        "np8" | "numpad8" | "kp_uparrow" => KeybdKey::Numpad8Key,
        "np9" | "numpad9" | "kp_pgup" => KeybdKey::Numpad9Key,
        "a" => KeybdKey::AKey,
        "b" => KeybdKey::BKey,
        "c" => KeybdKey::CKey,
        "d" => KeybdKey::DKey,
        "e" => KeybdKey::EKey,
        "f" => KeybdKey::FKey,
        "g" => KeybdKey::GKey,
        "h" => KeybdKey::HKey,
        "i" => KeybdKey::IKey,
        "j" => KeybdKey::JKey,
        "k" => KeybdKey::KKey,
        "l" => KeybdKey::LKey,
        "m" => KeybdKey::MKey,
        "n" => KeybdKey::NKey,
        "o" => KeybdKey::OKey,
        "p" => KeybdKey::PKey,
        "q" => KeybdKey::QKey,
        "r" => KeybdKey::RKey,
        "s" => KeybdKey::SKey,
        "t" => KeybdKey::TKey,
        "u" => KeybdKey::UKey,
        "v" => KeybdKey::VKey,
        "w" => KeybdKey::WKey,
        "x" => KeybdKey::XKey,
        "y" => KeybdKey::YKey,
        "z" => KeybdKey::ZKey,
        "f1" => KeybdKey::F1Key,
        "f2" => KeybdKey::F2Key,
        "f3" => KeybdKey::F3Key,
        "f4" => KeybdKey::F4Key,
        "f5" => KeybdKey::F5Key,
        "f6" => KeybdKey::F6Key,
        "f7" => KeybdKey::F7Key,
        "f8" => KeybdKey::F8Key,
        "f9" => KeybdKey::F9Key,
        "f10" => KeybdKey::F10Key,
        "f11" => KeybdKey::F11Key,
        "f12" => KeybdKey::F12Key,
        "numlock" => KeybdKey::NumLockKey,
        "scrolllock" => KeybdKey::ScrollLockKey,
        "capslock" | "caps" => KeybdKey::CapsLockKey,
        "shift" | "lshift" => KeybdKey::LShiftKey,
        _ => KeybdKey::F8Key
    }
}