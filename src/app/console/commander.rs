use std::fs::{read_dir, File, OpenOptions};
use std::io::prelude::*;

use inputbot::KeybdKey;

use crate::app::settings::Settings;
use crate::server::player::Player;

pub struct Commander {
    file: File,
    file_name: String,
}

impl Commander {
    pub fn new(directory: &str) -> Commander {

        #[cfg(not(windows))]
        inputbot::init_device();

        let dir: String = directory.to_string();

        if !check_directory(directory) {
            println!("Could not find tf2 directory in {}", directory);
            if !check_directory(".") {
                println!("Could not find tf2 directory in current folder. Please set a valid path in settings.cfg or run this program from the Team Fortress 2 folder.");
                std::process::exit(1);
            }
        }

        let file_name = format!("{}/tf/cfg/command.cfg", dir);

        Commander {
            file: create_command_file(&file_name),
            file_name,
        }
    }

    /// Clears queued / recently run commands
    pub fn clear(&mut self) {
        match File::create(&self.file_name) {
            Err(_) => {
                eprintln!("Couldn't clear command file!");
            }
            Ok(file) => {
                self.file = file;
            }
        }
    }

    /// Pushes a new command to the queue
    pub fn push(&mut self, command: &str) {
        if self
            .file
            .write_all(format!("{}; ", command).as_bytes())
            .is_err()
        {
            eprintln!("Could not write command to command.cfg file!");
        }
    }

    /// Runs all queued commands
    pub fn run(&self, key: &KeybdKey) {
        key.press();
        key.release();
    }

    /// Clears queue and runs a command
    pub fn run_command(&mut self, command: &str, key: &KeybdKey) {
        self.clear();
        self.push(command);
        self.run(key);
    }

    pub fn say(&mut self, s: &str, settings: &Settings) {
        self.run_command(&format!("say \"{}\"", s), &settings.key);
    }

    pub fn kick(&mut self, p: &Player, settings: &Settings) {
        self.run_command(&format!("callvote kick {}", p.userid), &settings.key);
    }
}

fn create_command_file(file_name: &str) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name)
        .unwrap()
}

fn check_directory(dir: &str) -> bool {
    //Check if valid TF2 directory
    read_dir(format!("{}/tf/cfg", dir)).is_ok()
}
