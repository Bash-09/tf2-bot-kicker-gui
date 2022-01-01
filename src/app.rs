use std::{time::SystemTime, fs::read_dir};

use chrono::{Local, DateTime};
use clipboard::{ClipboardProvider, ClipboardContext};
use eframe::{egui::{self, Ui, Color32}, epi};

pub mod timer;
use regex::Regex;
use timer::*;

pub mod settings;
use settings::*;

pub mod server;
use server::*;

pub mod console;
use console::*;

mod regexes;
use self::{regexes::*, server::player::{Team, Player, State}};

pub mod bot_checker;
use bot_checker::*;

use self::{console::{commander::Commander, log_watcher::LogWatcher}, regexes::LogMatcher};

pub struct TemplateApp {

    timer: Timer,
    message: String,
    paused: bool,

    settings: Settings,
    console: Option<Console>,

    server: Server,
    regexes: Vec<LogMatcher>,
    bot_checker: BotChecker,

}

impl Default for TemplateApp {
    fn default() -> Self {

        let settings: Settings;

        let set = Settings::import("cfg/settings.json");
        if set.is_err() {
            settings = Settings::new();
        } else {
            settings = set.unwrap();
        }

        let console = use_directory(&settings.directory);

        let reg = vec![
            LogMatcher::new(Regex::new(r_status).unwrap(), f_status),
            LogMatcher::new(Regex::new(r_lobby).unwrap(), f_lobby),
            LogMatcher::new(Regex::new(r_user_connect).unwrap(), f_user_connect),
            LogMatcher::new(Regex::new(r_user_disconnect).unwrap(), f_user_disconnect),
            LogMatcher::new(Regex::new(r_list_players).unwrap(), f_list_players),
            LogMatcher::new(Regex::new(r_update).unwrap(), f_update),
            LogMatcher::new(Regex::new(r_inactive).unwrap(), f_inactive),
            LogMatcher::new(Regex::new(r_refresh_complete).unwrap(), f_refresh_complete),
        ];

        let mut message = String::from("Loaded");

        let mut bot_checker = BotChecker::new();
        for uuid_list in &settings.uuid_lists {
            match bot_checker.add_steamid_list(uuid_list) {
                Ok(_) => {},
                Err(e) => message = format!("{}", e),
            }
        }
        for regex_list in &settings.regex_lists {
            match bot_checker.add_regex_list(regex_list) {
                Ok(_) => {},
                Err(e) => message = format!("{}", e),
            }        
        }

        Self {
            timer: Timer::new(),
            settings,
            message,
            console,
            paused: true,
            server: Server::new(),
            regexes: reg,
            bot_checker,
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "TF2 Bot Kicker by Bash09/Googe14"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &eframe::epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {



    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &eframe::epi::Frame) {
        // Ensures update is called again as soon as this one is finished.
        ctx.request_repaint();

        // Skip the update if it hasn't been very long
        let t = self.timer.go(self.settings.period);
        if t.is_none() {return;}

        // Update
        if self.timer.update() && !self.paused && self.console.is_some() {
            let system_time = SystemTime::now();
            let datetime: DateTime<Local> = system_time.into();

            match &mut self.console {
                Some(con) => {
                    self.server.refresh(&self.settings, &mut con.com);
                    self.message = format!("Refreshing... ({})", datetime.format("%T"));
                },
                None => {}
            }

        }

        match &mut self.console {
            Some(con) => {
                loop {
                    match con.log.next_line() {
                        Some(line) => {
                            for r in self.regexes.iter() {
                                match r.r.captures(&line) {
                                    None => {}
                                    Some(c) => {
                                        (r.f)(&mut self.server, &line, c, &self.settings, &mut con.com, &mut self.paused, &mut self.bot_checker);
                                    }
                                }
                            }
                        },
                        None => {break;}
                    }
                }
            },
            None => {}
        }

        // Tracks if the settings need to be saved
        let mut settings_changed = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {

                    if ui.button("Set TF2 Directory").clicked() {
                        match rfd::FileDialog::new().pick_folder() {
                            Some(pb) => {
                                let dir = pb.to_string_lossy().to_string();
                                self.settings.directory = dir;
                                self.console = use_directory(&self.settings.directory);
                                settings_changed = true;
                            },
                            None => {}
                        }
                    }

                    if ui.button("Add Regex List").clicked() {
                        match rfd::FileDialog::new().set_directory("cfg").pick_file() {
                            Some(pb) => {
                                let file = pb.to_string_lossy().to_string();
                                match self.bot_checker.add_regex_list(&file) {
                                    Ok(_) => {
                                        self.message = format!("Added {} as a regex list", &file.split("/").last().unwrap());
                                    },
                                    Err(e) => {
                                        self.message = format!("{}", e);
                                    }
                                }
                                self.settings.regex_lists.push(file);
                                settings_changed = true;
                            },
                            None => {}
                        }
                    }

                    if ui.button("Add SteamID List").clicked() {
                        match rfd::FileDialog::new().set_directory("cfg").pick_file() {
                            Some(pb) => {
                                let file = pb.to_string_lossy().to_string();
                                match self.bot_checker.add_steamid_list(&file) {
                                    Ok(_) => {
                                        self.message = format!("Added {} as a steamid list", &file.split("/").last().unwrap());
                                    },
                                    Err(e) => {
                                        self.message = format!("{}", e);
                                    }
                                }
                                self.settings.uuid_lists.push(file);
                                settings_changed = true;
                            },
                            None => {}
                        }
                    }

                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });

                if ui.button(match self.paused {true => "Start", false => "Pause"}).clicked() {
                    self.paused = !self.paused;
                    self.message = String::from(match self.paused {true => "Paused", false => "Started"});
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {

            // Display a little bit of information
            ui.label(&self.message);

            // Credits at the bottom left
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("powered by ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label(" and ");
                ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
            });

        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Settings");

                ui.horizontal(|ui| {
                    ui.label("User: ");
                    settings_changed |= ui.text_edit_singleline(&mut self.settings.user).changed();
                });

                settings_changed |= ui.checkbox(&mut self.settings.kick, "Kick Bots").changed();
                settings_changed |= ui.checkbox(&mut self.settings.join_alert, "Join Alerts").changed();
                settings_changed |= ui.checkbox(&mut self.settings.chat_reminders, "Chat Reminders").changed();

                ui.horizontal(|ui| {
                    ui.label("Period: ");
                    settings_changed |= ui.add(egui::Slider::new(&mut self.settings.period, 1.0..=60.0)).changed();
                });

                // Command Key
                ui.horizontal(|ui| {
                    ui.label(&format!("Command key: {}", key_to_str(self.settings.key)));
                    ui.menu_button("Change", |ui| {

                        egui::ScrollArea::vertical().show(ui, |ui| {

                            if ui.button("F1").clicked() {self.settings.key = str_to_key("F1"); settings_changed = true;}
                            if ui.button("F2").clicked() {self.settings.key = str_to_key("F2"); settings_changed = true;}
                            if ui.button("F3").clicked() {self.settings.key = str_to_key("F3"); settings_changed = true;}
                            if ui.button("F4").clicked() {self.settings.key = str_to_key("F4"); settings_changed = true;}
                            if ui.button("F5").clicked() {self.settings.key = str_to_key("F5"); settings_changed = true;}
                            if ui.button("F6").clicked() {self.settings.key = str_to_key("F6"); settings_changed = true;}
                            if ui.button("F7").clicked() {self.settings.key = str_to_key("F7"); settings_changed = true;}
                            if ui.button("F8").clicked() {self.settings.key = str_to_key("F8"); settings_changed = true;}
                            if ui.button("F9").clicked() {self.settings.key = str_to_key("F9"); settings_changed = true;}
                            if ui.button("F10").clicked() {self.settings.key = str_to_key("F10"); settings_changed = true;}
                            if ui.button("F11").clicked() {self.settings.key = str_to_key("F11"); settings_changed = true;}
                            if ui.button("F12").clicked() {self.settings.key = str_to_key("F12"); settings_changed = true;}
                            if ui.button("kp_ins").clicked() {self.settings.key = str_to_key("kp_ins"); settings_changed = true;}
                            if ui.button("kp_end").clicked() {self.settings.key = str_to_key("kp_end"); settings_changed = true;}
                            if ui.button("kp_downarrow").clicked() {self.settings.key = str_to_key("kp_downarrow"); settings_changed = true;}
                            if ui.button("kp_pgdn").clicked() {self.settings.key = str_to_key("kp_pgdn"); settings_changed = true;}
                            if ui.button("kp_leftarrow").clicked() {self.settings.key = str_to_key("kp_leftarrow"); settings_changed = true;}
                            if ui.button("kp_5").clicked() {self.settings.key = str_to_key("kp_5"); settings_changed = true;}
                            if ui.button("kp_rightarrow").clicked() {self.settings.key = str_to_key("kp_rightarrow"); settings_changed = true;}
                            if ui.button("kp_home").clicked() {self.settings.key = str_to_key("kp_home"); settings_changed = true;}
                            if ui.button("kp_uparrow").clicked() {self.settings.key = str_to_key("kp_uparrow"); settings_changed = true;}
                            if ui.button("kp_pgup").clicked() {self.settings.key = str_to_key("kp_pgup"); settings_changed = true;}
                            if ui.button("numlock").clicked() {self.settings.key = str_to_key("numlock"); settings_changed = true;}
                            if ui.button("scrolllock").clicked() {self.settings.key = str_to_key("scrolllock"); settings_changed = true;}
                            if ui.button("capslock").clicked() {self.settings.key = str_to_key("capslock"); settings_changed = true;}
                            if ui.button("shift").clicked() {self.settings.key = str_to_key("shift"); settings_changed = true;}
                            if ui.button("A").clicked() {self.settings.key = str_to_key("A"); settings_changed = true;}
                            if ui.button("B").clicked() {self.settings.key = str_to_key("B"); settings_changed = true;}
                            if ui.button("C").clicked() {self.settings.key = str_to_key("C"); settings_changed = true;}
                            if ui.button("D").clicked() {self.settings.key = str_to_key("D"); settings_changed = true;}
                            if ui.button("E").clicked() {self.settings.key = str_to_key("E"); settings_changed = true;}
                            if ui.button("F").clicked() {self.settings.key = str_to_key("F"); settings_changed = true;}
                            if ui.button("G").clicked() {self.settings.key = str_to_key("G"); settings_changed = true;}
                            if ui.button("H").clicked() {self.settings.key = str_to_key("H"); settings_changed = true;}
                            if ui.button("I").clicked() {self.settings.key = str_to_key("I"); settings_changed = true;}
                            if ui.button("J").clicked() {self.settings.key = str_to_key("J"); settings_changed = true;}
                            if ui.button("K").clicked() {self.settings.key = str_to_key("K"); settings_changed = true;}
                            if ui.button("L").clicked() {self.settings.key = str_to_key("L"); settings_changed = true;}
                            if ui.button("M").clicked() {self.settings.key = str_to_key("M"); settings_changed = true;}
                            if ui.button("N").clicked() {self.settings.key = str_to_key("N"); settings_changed = true;}
                            if ui.button("O").clicked() {self.settings.key = str_to_key("O"); settings_changed = true;}
                            if ui.button("P").clicked() {self.settings.key = str_to_key("P"); settings_changed = true;}
                            if ui.button("Q").clicked() {self.settings.key = str_to_key("Q"); settings_changed = true;}
                            if ui.button("R").clicked() {self.settings.key = str_to_key("R"); settings_changed = true;}
                            if ui.button("S").clicked() {self.settings.key = str_to_key("S"); settings_changed = true;}
                            if ui.button("T").clicked() {self.settings.key = str_to_key("T"); settings_changed = true;}
                            if ui.button("U").clicked() {self.settings.key = str_to_key("U"); settings_changed = true;}
                            if ui.button("V").clicked() {self.settings.key = str_to_key("V"); settings_changed = true;}
                            if ui.button("W").clicked() {self.settings.key = str_to_key("W"); settings_changed = true;}
                            if ui.button("X").clicked() {self.settings.key = str_to_key("X"); settings_changed = true;}
                            if ui.button("Y").clicked() {self.settings.key = str_to_key("Y"); settings_changed = true;}
                            if ui.button("Z").clicked() {self.settings.key = str_to_key("Z"); settings_changed = true;}

                        });

                    });
                });

                ui.label("");

                ui.collapsing("Regex Lists", |ui| {
                    let mut ind: Option<usize> = None;
                    for (i, l) in self.settings.regex_lists.iter().enumerate() {
                        let lab = ui.selectable_label(false, l.split("/").last().unwrap());
                        if lab.clicked() {
                            ind = Some(i);
                        }
                        lab.on_hover_text("Click to remove");
                    }
                    match ind {
                        Some(i) => {
                            self.settings.regex_lists.remove(i);
                            settings_changed = true;
                        },
                        None => {}
                    }
                });

                ui.collapsing("SteamID Lists", |ui| {
                    let mut ind: Option<usize> = None;
                    for (i, l) in self.settings.uuid_lists.iter().enumerate() {
                        let lab = ui.selectable_label(false, l.split("/").last().unwrap());
                        if lab.clicked() {
                            ind = Some(i);
                        }
                        lab.on_hover_text("Click to remove");
                    }
                    match ind {
                        Some(i) => {
                            self.settings.uuid_lists.remove(i);
                            settings_changed = true;
                        },
                        None => {}
                    }
                });

            });

        });


        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            match &self.console {
                // Text for if there's no TF2 directory set yet
                None=> {
                    ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");

                    ui.label("Instructions:");

                    ui.horizontal(|ui| {
                        ui.label("1. Add");
                        copy_label(&mut self.message, "-condebug -conclearlog", ui);
                        ui.label("to your TF2 launch options and start the game.");
                    });

                    ui.horizontal(|ui| {
                        ui.label("2. Add");
                        copy_label(&mut self.message, &format!("bind {} \"exec command.cfg\"", key_to_str(self.settings.key)), ui);
                        ui.label("to your autoexec.cfg file. (or change it for whichever key you choose)");
                    });

                    ui.horizontal(|ui| {
                        ui.label("3. Click");
                        if ui.button("Set your TF2 directory").clicked() {

                            match rfd::FileDialog::new().pick_folder() {
                                Some(pb) => {
                                    let dir = pb.to_string_lossy().to_string();
                                    self.settings.directory = dir;
                                    self.console = use_directory(&self.settings.directory);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }
                        ui.label("and navigate to your Team Fortress 2 folder");
                    });
                    ui.label("4. Start the program and enjoy the game!\n\n");
                    ui.label("Note: If you have set your TF2 directory but are still seeing this message, ensure you have added the launch options and launched the game before trying again.");

                },

                // UI when there is a TF2 directory present
                Some(log) => {
                    if self.server.players.is_empty() {
                        ui.label("Not currently connected to a server.");
                    } else {

                        let width = (ui.available_width()-5.0)/2.0;

                        egui::ScrollArea::vertical().show(ui, |ui| {

                            egui::Grid::new("players").striped(true).show(ui, |ui| {

                                let mut team1 = Vec::new();
                                let mut team2 = Vec::new();

                                for (_, p) in &self.server.players {
                                    if p.team == Team::Invaders {
                                        team1.push(p);
                                    } else if p.team == Team::Defenders {
                                        team2.push(p);
                                    }

                                }

                                ui.horizontal(|ui| {
                                    ui.set_width(width);
                                    ui.colored_label(Color32::WHITE, "Player Name");
                            
                                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("   ");
                                            ui.colored_label(Color32::WHITE, "Time");
                                            ui.colored_label(Color32::WHITE, "Info");
                                        });
                                    });
                                });
                                ui.horizontal(|ui| {
                                    ui.set_width(width);
                                    ui.colored_label(Color32::WHITE, "Player Name");
                            
                                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("   ");
                                            ui.colored_label(Color32::WHITE, "Time");
                                            ui.colored_label(Color32::WHITE, "Info");
                                        });
                                    });
                                });
                                ui.end_row();

                                let mut i = 0usize;
                                loop {

                                    if team1.get(i).is_none() && team2.get(i).is_none() {
                                        break;
                                    }

                                    match team1.get(i) {
                                        Some(p) => {
                                            render_player(ui, &self.settings, p, width);
                                        },
                                        None => {}
                                    }
                                    match team2.get(i) {
                                        Some(p) => {
                                            render_player(ui, &self.settings, p, width);
                                        },
                                        None => {}
                                    }
                                    ui.end_row();

                                    i += 1;
                                }
                            });
                        });
                    }
                }
            }
        });

            // egui::Window::new("Window").show(ctx, |ui| {
            //     ui.label("Windows can be moved by dragging them.");
            //     ui.label("They are automatically sized based on contents.");
            //     ui.label("You can turn on resizing and scrolling if you like.");
            //     ui.label("You would normally chose either panels OR windows.");
            // });

        if settings_changed {
            match self.settings.export("cfg/settings.json") {
                Ok(_) => {},
                Err(e) => {
                    println!("Failed to export settings");
                    self.message = e.to_string();
                }
            }
        }

    }
}

// Make a selectable label which copies it's text to the clipboard on click
fn copy_label(log: &mut String, text: &str, ui: &mut Ui) {
    let lab = ui.selectable_label(false, text);
        if lab.clicked() {
            let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> = ClipboardProvider::new();
            match ctx {
                Ok(mut ctx) => {
                    if ctx.set_contents(text.to_string()).is_ok() {
                        log.clear();
                        log.push_str(&format!("Copied '{}' to clipboard.", text));
                    } else {
                        log.clear();
                        log.push_str("Couldn't copy text to clipboard");
                    }
                },
                Err(e) => {
                    log.clear();
                    log.push_str(&format!("Couldn't copy text to clipboard: {}", e));
                }
            }
        }
        lab.on_hover_text("Copy");
}

// Try to open this TF2 directory
fn use_directory(dir: &str) -> Option<Console> {

    if read_dir(format!("{}/tf/cfg", dir)).is_ok() {
        
        match LogWatcher::register(&format!("{}/tf/console.log", dir)) {
            Ok(lw) => {
                return Some(Console{
                    log: lw,
                    com: Commander::new(dir),
                });
            },
            Err(e) => {
                println!("Failed to register log file: {}", e);
            }
        }

    }

    None
}

fn format_time(time: u32) -> String {
    format!("{}:{}", time/60, time%60)
}

fn render_player(ui: &mut Ui, set: &Settings, p: &Player, width: f32) {
    ui.horizontal(|ui| {
        ui.set_width(width);
        if p.steamid == set.user {
            ui.colored_label(Color32::LIGHT_GREEN, truncate(&p.name, 23));
        } else if p.bot {
            ui.colored_label(Color32::RED, truncate(&p.name, 23));
        } else {
            ui.label(truncate(&p.name, 23));
        }

        ui.with_layout(egui::Layout::right_to_left(), |ui| {
            ui.horizontal(|ui| {
                ui.label("   ");
                ui.label(&format_time(p.time));

                if p.bot {
                    ui.colored_label(Color32::RED, "BOT");
                }

                if p.state == State::Spawning {
                    ui.colored_label(Color32::YELLOW, "Joining");
                }
            });
        });

    });
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}