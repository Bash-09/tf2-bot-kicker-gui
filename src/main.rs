extern crate chrono;
extern crate clipboard;
extern crate rfd;
extern crate serde;

fn main() {
    let wb = WindowBuilder::new()
        .with_title("TF2 Bot Kicker by Bash09/Googe14")
        .with_resizable(true)
        .with_inner_size(PhysicalSize::new(800, 400));
    let app = TF2BotKicker::new();
    run(app, wb);
}

use std::{fs::OpenOptions, io::Write, ops::RangeInclusive, time::SystemTime};

use chrono::{DateTime, Local};
use clipboard::{ClipboardContext, ClipboardProvider};

pub mod timer;
use egui::{CollapsingHeader, Color32, RichText, Ui};
use egui_winit::winit::{dpi::PhysicalSize, window::WindowBuilder};
use glium_app::{context::Context, run, Application};
use regex::Regex;
use state::State;
use timer::*;

pub mod settings;
use settings::*;

pub mod server;
use server::*;

use tokio::runtime::Runtime;

mod regexes;
use self::{
    logwatcher::LogWatcher,
    server::player::{Player, Team},
};

pub mod bot_checker;

pub mod logwatcher;
pub mod state;

pub struct TF2BotKicker {
    refresh_timer: Timer,
    kick_timer: Timer,
    alert_timer: Timer,

    state: State,

    runtime: Runtime,
}

impl TF2BotKicker {
    // Create the application
    pub fn new() -> TF2BotKicker {
        let runtime = Runtime::new().expect("Failed to create async runtime");
        let state = State::new(&runtime);

        Self {
            refresh_timer: Timer::new(),
            alert_timer: Timer::new(),
            kick_timer: Timer::new(),
            state,

            runtime,
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        let mut target = ctx.dis.draw();

        let _ = ctx.gui.run(&ctx.dis, |gui_ctx| {

            // Tracks if the settings need to be saved
            let mut settings_changed = false;

            // Top menu bar
            egui::TopBottomPanel::top("top_panel").show(gui_ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {

                        if ui.button("Set TF2 Directory").clicked() {
                            match rfd::FileDialog::new().pick_folder() {
                                Some(pb) => {
                                    let dir;
                                    match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                        Ok(pb) => {
                                            dir = pb.to_string_lossy().to_string();
                                        },
                                        Err(_) => {
                                            dir = pb.to_string_lossy().to_string();
                                        }
                                    }
                                    self.state.settings.tf2_directory = dir;
                                    self.state.log = LogWatcher::use_directory(&self.state.settings.tf2_directory);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }

                        if ui.button("Add Regex List").clicked() {
                            match rfd::FileDialog::new().set_directory("cfg").pick_file() {
                                Some(pb) => {
                                    let dir;
                                    // Try to make it a relative directory instead of going from root
                                    match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                        Ok(pb) => {
                                            dir = pb.to_string_lossy().to_string();
                                        },
                                        Err(_) => {
                                            dir = pb.to_string_lossy().to_string();
                                        }
                                    }
                                    match self.state.bot_checker.add_regex_list(&dir) {
                                        Ok(_) => {
                                            self.state.message = format!("Added {} as a regex list", &dir.split("/").last().unwrap());
                                        },
                                        Err(e) => {
                                            self.state.message = format!("{}", e);
                                        }
                                    }
                                    self.state.settings.regex_lists.push(dir);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }

                        if ui.button("Add SteamID List").clicked() {
                            match rfd::FileDialog::new().set_directory("cfg").pick_file() {
                                Some(pb) => {
                                    let dir;
                                    match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                        Ok(pb) => {
                                            dir = pb.to_string_lossy().to_string();
                                        },
                                        Err(_) => {
                                            dir = pb.to_string_lossy().to_string();
                                        }
                                    }
                                    match self.state.bot_checker.add_steamid_list(&dir) {
                                        Ok(_) => {
                                            self.state.message = format!("Added {} as a steamid list", &dir.split("/").last().unwrap());
                                        },
                                        Err(e) => {
                                            self.state.message = format!("{}", e);
                                        }
                                    }
                                    self.state.settings.steamid_lists.push(dir);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }

                    });
                });
            });

            // Message and eframe/egui credits
            egui::TopBottomPanel::bottom("bottom_panel").show(gui_ctx, |ui| {

                // Display a little bit of information
                ui.label(&self.state.message);

                // Credits at the bottom left
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                });
            });

            // Left panel
            egui::SidePanel::left("side_panel").show(gui_ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Settings");

                    ui.horizontal(|ui| {
                        ui.label("User: ");
                        settings_changed |= ui.text_edit_singleline(&mut self.state.settings.user).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("RCon Password: ");
                        settings_changed |= ui.text_edit_singleline(&mut self.state.settings.rcon_password).changed();
                    });

                    ui.label("");

                    ui.horizontal(|ui| {
                        settings_changed |= ui.add(egui::DragValue::new(&mut self.state.settings.refresh_period)
                        .speed(0.1)
                        .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                        ui.label("Refresh Period");
                    });

                    settings_changed |= ui.checkbox(&mut self.state.settings.kick, "Kick Bots").changed();
                    if self.state.settings.kick {
                        ui.horizontal(|ui| {
                            settings_changed |= ui.add(egui::DragValue::new(&mut self.state.settings.kick_period)
                            .speed(0.1)
                            .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                            ui.label("Kick Period");
                        });
                    }

                    
                    settings_changed |= ui.checkbox(&mut self.state.settings.join_alert, "Join Alerts").changed();
                    settings_changed |= ui.checkbox(&mut self.state.settings.chat_reminders, "Chat Reminders").changed();

                    if self.state.settings.join_alert || self.state.settings.chat_reminders {
                        ui.horizontal(|ui| {
                            settings_changed |= ui.add(egui::DragValue::new(&mut self.state.settings.alert_period)
                            .speed(0.1)
                            .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                            ui.label("Chat Alert Period");
                        });
                    }

                    ui.label("");
                    ui.heading("Bot Detection Rules");

                    settings_changed |= ui.checkbox(&mut self.state.settings.record_steamids, &format!("Automatically record bot SteamIDs")).changed();

                    ui.label("");
                    ui.collapsing("Regex Lists", |ui| {
                        let mut ind: Option<usize> = None;
                        for (i, l) in self.state.settings.regex_lists.iter().enumerate() {

                            let active = l.eq(&self.state.settings.regex_list);
                            let mut text = RichText::new(l.split("/").last().unwrap());
                            if active {
                                text = text.color(Color32::LIGHT_GREEN);
                            }
                            ui.collapsing(text, |ui| {
                                if ui.button("Remove").clicked() {
                                    ind = Some(i);
                                }
                                if !active {
                                    let set = ui.button("Set Active");
                                    let set = set.on_hover_text("Recorded Regexes will be added to this file");
                                    if set.clicked() {
                                        self.state.settings.regex_list = l.clone();
                                        settings_changed = true;
                                    }
                                }
                            });
                        }
                        match ind {
                            Some(i) => {
                                self.state.settings.regex_lists.remove(i);
                                settings_changed = true;
                            },
                            None => {}
                        }
                    });

                    ui.collapsing("SteamID Lists", |ui| {
                        let mut ind: Option<usize> = None;
                        for (i, l) in self.state.settings.steamid_lists.iter().enumerate() {

                            let active = l.eq(&self.state.settings.steamid_list);
                            let mut text = RichText::new(l.split("/").last().unwrap());
                            if active {
                                text = text.color(Color32::LIGHT_GREEN);
                            }
                            ui.collapsing(text, |ui| {
                                if ui.button("Remove").clicked() {
                                    ind = Some(i);
                                }
                                if !active {
                                    let set = ui.button("Set Active");
                                    let set = set.on_hover_text("Recorded SteamIDs will be added to this file");
                                    if set.clicked() {
                                        self.state.settings.steamid_list = l.clone();
                                        settings_changed = true;
                                    }
                                }
                            });
                        }
                        match ind {
                            Some(i) => {
                                self.state.settings.steamid_lists.remove(i);
                                settings_changed = true;
                            },
                            None => {}
                        }
                    });
                });
            });

            // Main window with info and players
            egui::CentralPanel::default().show(gui_ctx, |ui| {

                if self.state.log.is_none() {

                    ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");

                    ui.label("Instructions:");

                    ui.horizontal(|ui| {
                        ui.label("1. Add");
                        copy_label(&mut self.state.message, "-condebug -conclearlog -usercon", ui);
                        ui.label("to your TF2 launch options and start the game.");
                    });

                    ui.horizontal(|ui| {
                        ui.label("2. Click");
                        if ui.button("Set your TF2 directory").clicked() {

                            match rfd::FileDialog::new().pick_folder() {
                                Some(pb) => {
                                    let dir;
                                    match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                        Ok(pb) => {
                                            dir = pb.to_string_lossy().to_string();
                                        },
                                        Err(_) => {
                                            dir = pb.to_string_lossy().to_string();
                                        }
                                    }
                                    self.state.settings.tf2_directory = dir;
                                    self.state.log = LogWatcher::use_directory(&self.state.settings.tf2_directory);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }
                        ui.label("and navigate to your Team Fortress 2 folder");
                    });
                    ui.label("3. Start the program and enjoy the game!\n\n");
                    ui.label("Note: If you have set your TF2 directory but are still seeing this message, ensure you have added the launch options and launched the game before trying again.");


                } else {
                    match &self.state.rcon {
                        // Connected and good
                        Ok(_) => {

                            if self.state.server.players.is_empty() {
                                ui.label("Not currently connected to a server.");
                            } else {

                                let width = (ui.available_width()-5.0)/2.0;

                                egui::ScrollArea::vertical().show(ui, |ui| {

                                    ui.columns(2, |cols| {

                                        // Headings
                                        cols[0].horizontal(|ui| {
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

                                        cols[1].horizontal(|ui| {
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

                                        for (_, p) in &mut self.state.server.players {

                                            if p.team == Team::Invaders {
                                                render_player(&mut cols[0], &self.state.settings, &mut self.state.message, p, width, &mut self.state.export_steamid, &mut self.state.export_regex, &mut self.state.steamid);
                                            }
        
                                            if p.team == Team::Defenders {
                                                render_player(&mut cols[1], &self.state.settings, &mut self.state.message, p, width, &mut self.state.export_steamid, &mut self.state.export_regex, &mut self.state.steamid);
                                            }
        
                                        }

                                    });
                                });
                            }
                        },

                        // RCON couldn't connect
                        Err(e) => {
                            match e {
                                // Wrong password
                                rcon::Error::Auth => {
                                    ui.heading("Failed to authorise RCON - Password incorrect");

                                    ui.horizontal(|ui| {
                                        ui.label("Run ");
                                        copy_label(&mut self.state.message, &format!("rcon_password {}", &self.state.settings.rcon_password), ui);
                                        ui.label("in your TF2 console, and make sure it is in your autoexec.cfg file.");
                                    });
                                },
                                // Connection issue
                                _ => {
                                    ui.heading("Could not connect to TF2:");

                                    ui.label("");
                                    ui.label("Is TF2 running?");
                                    ui.horizontal(|ui| {
                                        ui.label("Does your autoexec.cfg file contain");
                                        copy_label(&mut self.state.message, "net_start", ui);
                                        ui.label("?");
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Does your TF2 launch option include");
                                        copy_label(&mut self.state.message, "-usercon", ui);
                                        ui.label("?");
                                    });
                                }
                            }
                        }
                    }
                }


                match &mut self.state.export_steamid {
                    Some(text) => {
                        let mut open = true;
                        let mut exported = false;
                        egui::Window::new("Export SteamID")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .open(&mut open)
                        .collapsible(false)
                        .show(gui_ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.text_edit_singleline(text);
                                if ui.button("Confirm").clicked() {
                                    append_line(&text, &self.state.settings.steamid_list);
                                    self.state.bot_checker.append_steamid(&self.state.steamid);
                                    self.state.message = format!("Saved \"{}\" to {}", &text, &self.state.settings.steamid_list);
                                    exported = true;
                                }
                            });
                        });
                        if !open || exported {
                            self.state.export_steamid = None;
                        }
                    },
                    None => {}
                }

                match &mut self.state.export_regex {
                    Some(text) => {
                        let mut open = true;
                        let mut exported = false;
                        egui::Window::new("Export Name/Regex")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .open(&mut open)
                        .collapsible(false)
                        .show(gui_ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.text_edit_singleline(text);
                                if ui.button("Confirm").clicked() {

                                    let reg = Regex::new(&text);
                                    match reg {
                                        Ok(reg) => {
                                            append_line(&text, &self.state.settings.regex_list);
                                            self.state.bot_checker.append_regex(reg);
                                            self.state.message = format!("Saved \"{}\" to {}", &text, &self.state.settings.regex_list);
                                            exported = true;
                                        },
                                        Err(e) => {
                                            self.state.message = format!("Invalid Regex: {}", e);
                                        }
                                    }
                                }
                            });
                        });
                        if !open || exported {
                            self.state.export_regex = None;
                        }
                    },
                    None => {}
                }
            });

            // Export settings if they've changed
            if settings_changed {
                let _new_dir = std::fs::create_dir("cfg");
                match self.state.settings.export("cfg/settings.json") {
                    Ok(_) => {
                        println!("Saved settings");
                    },
                    Err(e) => {
                        eprintln!("Failed to export settings");
                        self.state.message = e.to_string();
                    }
                }
            }

        });

        ctx.gui.paint(&mut ctx.dis, &mut target);
        target.finish().unwrap();
    }
}

impl Application for TF2BotKicker {
    fn init(&mut self, _ctx: &mut glium_app::context::Context) {
        self.refresh_timer.reset();
        self.kick_timer.reset();
        self.alert_timer.reset();
    }

    fn update(&mut self, _t: &glium_app::Timer, ctx: &mut Context) {
        let TF2BotKicker {
            refresh_timer,
            kick_timer,
            alert_timer,

            state,

            runtime,
        } = self;

        let refresh = refresh_timer.go(state.settings.refresh_period);

        if refresh.is_none() {
            return;
        }

        kick_timer.go(state.settings.kick_period);
        alert_timer.go(state.settings.alert_period);

        runtime.block_on(async {
            // Refresh server
            if refresh_timer.update() {
                state.refresh().await;

                let system_time = SystemTime::now();
                let datetime: DateTime<Local> = system_time.into();
                state.message = format!("Refreshed ({})", datetime.format("%T"));
            }
        });

        match &mut state.log {
            Some(lw) => {
                // If there is a loaded dir, process any new console lines
                loop {
                    match lw.next_line() {
                        Some(line) => {
                            match state.regx_disconnect.r.captures(&line) {
                                None => {}
                                Some(c) => {
                                    (state.regx_disconnect.f)(
                                        &mut state.server,
                                        &line,
                                        c,
                                        &state.settings,
                                        &mut state.bot_checker,
                                    );
                                    continue;
                                }
                            }

                            match state.regx_status.r.captures(&line) {
                                Some(c) => {
                                    (state.regx_status.f)(
                                        &mut state.server,
                                        &line,
                                        c,
                                        &state.settings,
                                        &mut state.bot_checker,
                                    );
                                    continue;
                                }
                                None => {}
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
            None => {}
        }

        runtime.block_on(async {
            // Kick Bots
            if kick_timer.update() {
                if state.rcon_connected().await {
                    state
                        .server
                        .kick_bots(&state.settings, state.rcon.as_mut().unwrap())
                        .await;
                }
            }

            // Send chat alerts
            if alert_timer.update() {
                if state.rcon_connected().await {
                    state
                        .server
                        .announce_bots(&state.settings, state.rcon.as_mut().unwrap())
                        .await;
                }
            }
        });

        self.render(ctx);
    }

    fn close(&mut self) {}

    fn handle_event(&mut self, ctx: &mut Context, event: &egui_winit::winit::event::Event<()>) {}
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
            }
            Err(e) => {
                log.clear();
                log.push_str(&format!("Couldn't copy text to clipboard: {}", e));
            }
        }
    }
    lab.on_hover_text("Copy");
}

// u32 -> minutes:seconds
fn format_time(time: u32) -> String {
    format!("{}:{}", time / 60, time % 60)
}

const TRUNC_LEN: usize = 20;

/// Truncates a &str
fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

pub fn append_line(data: &str, target: &str) {
    // Add suspected bot steamid and name to file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(target)
        .expect(&format!("Failed to Open or Write to {}", target));

    if let Err(_) = write!(file, "\n{}", data) {
        eprintln!("Failed to Open or Write to {}", target);
    }
}

// Ui for a player
fn render_player(
    ui: &mut Ui,
    set: &Settings,
    mes: &mut String,
    p: &mut Player,
    width: f32,
    export_steamid: &mut Option<String>,
    export_regex: &mut Option<String>,
    steamid: &mut String,
) {
    ui.horizontal(|ui| {
        ui.set_width(width);

        let text;
        if p.steamid == set.user {
            text = egui::RichText::new(truncate(&p.name, TRUNC_LEN)).color(Color32::GREEN);
        } else if p.bot {
            text = egui::RichText::new(truncate(&p.name, TRUNC_LEN)).color(Color32::RED);
        } else {
            text = egui::RichText::new(truncate(&p.name, TRUNC_LEN));
        }

        CollapsingHeader::new(text)
            .id_source(&p.userid)
            .show(ui, |ui| {
                let prefix = match p.bot {
                    true => "NOT ",
                    false => "",
                };
                let mut text = RichText::new(&format!("Mark as {}Bot", prefix));
                if !p.bot {
                    text = text.color(Color32::LIGHT_RED);
                } else {
                    text = text.color(Color32::LIGHT_GREEN);
                }

                if ui.selectable_label(false, text).clicked() {
                    p.bot = !p.bot;
                }

                ui.horizontal(|ui| {
                    if ui.button("Copy Name").clicked() {
                        let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                            ClipboardProvider::new();
                        ctx.unwrap().set_contents(p.name.clone()).unwrap();
                        mes.clear();
                        mes.push_str(&format!("Copied \"{}\"", p.name));
                    }
                    if ui.button("Copy SteamID").clicked() {
                        let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                            ClipboardProvider::new();
                        ctx.unwrap().set_contents(p.steamid.clone()).unwrap();
                        mes.clear();
                        mes.push_str(&format!("Copied \"{}\"", p.steamid));
                    }
                });

                if p.bot {
                    ui.horizontal(|ui| {
                        let but = ui.button(RichText::new("Save Name").color(Color32::LIGHT_RED));
                        if but.clicked() {
                            *export_regex = Some(p.get_export_regex());
                        }
                        but.on_hover_text(
                            RichText::new(
                                "Players with this name will always be recognized as a bot",
                            )
                            .color(Color32::RED),
                        );

                        let but =
                            ui.button(RichText::new("Save SteamID").color(Color32::LIGHT_RED));
                        if but.clicked() {
                            *export_steamid = Some(p.get_export_steamid());
                            *steamid = p.steamid.clone();
                        }
                        but.on_hover_text(
                            RichText::new("This player will always be recognized as a bot")
                                .color(Color32::RED),
                        );
                    });
                }
            });

        ui.with_layout(egui::Layout::right_to_left(), |ui| {
            ui.horizontal(|ui| {
                ui.label("   ");
                ui.label(&format_time(p.time));

                if p.bot {
                    ui.colored_label(Color32::RED, "BOT");
                }

                if p.state == player::State::Spawning {
                    ui.colored_label(Color32::YELLOW, "Joining");
                }
            });
        });
    });
}
