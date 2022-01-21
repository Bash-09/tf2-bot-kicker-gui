use std::{ops::RangeInclusive, time::SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use clipboard::{ClipboardContext, ClipboardProvider};

pub mod timer;
use egui::{Color32, RichText, Ui};
use rcon::Connection;
use regex::Regex;
use timer::*;

pub mod settings;
use settings::*;

pub mod server;
use server::*;

use tokio::net::TcpStream;

mod regexes;
use self::{
    logwatcher::LogWatcher,
    regexes::*,
    server::player::{Player, State, Team},
};

pub mod bot_checker;
use bot_checker::*;

pub mod logwatcher;

use glium_app::Surface;

pub struct TF2BotKicker {
    refresh_timer: Timer,
    kick_timer: Timer,
    alert_timer: Timer,

    message: String,

    settings: Settings,
    rcon: rcon::Result<Connection<TcpStream>>,
    log: Option<LogWatcher>,

    server: Server,

    regx_status: LogMatcher,
    regx_lobby: LogMatcher,
    regx_disconnect: LogMatcher,

    bot_checker: BotChecker,
}

impl TF2BotKicker {
    // Create the application
    pub async fn new() -> TF2BotKicker {
        let settings: Settings;

        // Attempt to load settings, create new default settings if it can't load an existing file
        let set = Settings::import("cfg/settings.json");
        if set.is_err() {
            settings = Settings::new();
        } else {
            settings = set.unwrap();
        }

        // Load regexes
        let regx_status = LogMatcher::new(Regex::new(r_status).unwrap(), f_status);
        let regx_lobby = LogMatcher::new(Regex::new(r_lobby).unwrap(), f_lobby);
        let regx_disconnect =
            LogMatcher::new(Regex::new(r_user_disconnect).unwrap(), f_user_disconnect);

        let mut message = String::from("Loaded");

        // Create bot checker and load any bot detection rules saved
        let mut bot_checker = BotChecker::new();
        for uuid_list in &settings.uuid_lists {
            match bot_checker.add_steamid_list(uuid_list) {
                Ok(_) => {}
                Err(e) => message = format!("Error loading {}: {}", uuid_list, e),
            }
        }
        for regex_list in &settings.regex_lists {
            match bot_checker.add_regex_list(regex_list) {
                Ok(_) => {}
                Err(e) => message = format!("Error loading {}: {}", regex_list, e),
            }
        }

        let rcon = Connection::connect("127.0.0.1:27015", &settings.rcon_password).await;
        let log = LogWatcher::use_directory(&settings.tf2_directory);

        Self {
            refresh_timer: Timer::new(),
            alert_timer: Timer::new(),
            kick_timer: Timer::new(),
            message,
            settings,
            rcon,
            log,
            server: Server::new(),

            regx_status,
            regx_lobby,
            regx_disconnect,

            bot_checker,
        }
    }

    pub async fn rcon_connected(&mut self) -> bool {
        match &mut self.rcon {
            Ok(con) => match con.cmd("echo Ping").await {
                Ok(_) => {
                    return true;
                }
                Err(e) => {
                    println!("Error with rcon: {:?}", &e);
                    self.rcon = Err(e);
                    return false;
                }
            },
            Err(_) => {
                match Connection::connect("127.0.0.1:27015", &self.settings.rcon_password).await {
                    Ok(con) => {
                        self.rcon = Ok(con);
                        return true;
                    }
                    Err(e) => {
                        println!("Error with rcon: {:?}", &e);
                        self.rcon = Err(e);
                        return false;
                    }
                }
            }
        }
    }

    pub async fn refresh(&mut self) {
        if !self.rcon_connected().await {
            return;
        }
        self.server.prune();

        let status = self
            .rcon
            .as_mut()
            .unwrap()
            .cmd("status; wait 200; echo \"refreshcomplete\"")
            .await;
        let lobby = self.rcon.as_mut().unwrap().cmd("tf_lobby_debug").await;

        if status.is_err() || lobby.is_err() {
            return;
        }

        let lobby = lobby.unwrap();
        println!("Lobby:\n {}", lobby);

        if lobby.contains("Failed to find lobby shared object") {
            self.server.clear();
            return;
        }

        self.server.refresh();

        for l in lobby.lines() {
            match self.regx_lobby.r.captures(l) {
                None => {}
                Some(c) => {
                    (self.regx_lobby.f)(
                        &mut self.server,
                        l,
                        c,
                        &self.settings,
                        &mut self.bot_checker,
                    );
                }
            }
        }
    }
}

#[async_trait]
impl glium_app::Application for TF2BotKicker {
    fn launch_settings(&self) -> glium_app::WindowBuilder {
        glium_app::WindowBuilder::new()
            .with_title("TF2 Bot Kicker by Bash09/Googe14")
            .with_resizable(true)
            .with_inner_size(glium_app::PhysicalSize::new(800, 350))
    }

    fn init(&mut self, _ctx: &mut glium_app::context::Context) {
        self.refresh_timer.reset();
        self.kick_timer.reset();
        self.alert_timer.reset();
    }

    async fn update(&mut self, _t: &glium_app::timer::Timer) {
        let refresh = self.refresh_timer.go(self.settings.refresh_period);

        if refresh.is_none() {
            return;
        }

        self.kick_timer.go(self.settings.kick_period);
        self.alert_timer.go(self.settings.alert_period);

        // Refresh server
        if self.refresh_timer.update() {
            self.refresh().await;

            let system_time = SystemTime::now();
            let datetime: DateTime<Local> = system_time.into();
            self.message = format!("Refreshed ({})", datetime.format("%T"));
        }

        match &mut self.log {
            Some(lw) => {
                // If there is a loaded dir, process any new console lines
                loop {
                    match lw.next_line() {
                        Some(line) => {
                            match self.regx_disconnect.r.captures(&line) {
                                None => {}
                                Some(c) => {
                                    (self.regx_disconnect.f)(
                                        &mut self.server,
                                        &line,
                                        c,
                                        &self.settings,
                                        &mut self.bot_checker,
                                    );
                                    continue;
                                }
                            }

                            match self.regx_status.r.captures(&line) {
                                Some(c) => {
                                    (self.regx_status.f)(
                                        &mut self.server,
                                        &line,
                                        c,
                                        &self.settings,
                                        &mut self.bot_checker,
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

        // Kick Bots
        if self.kick_timer.update() {
            if self.rcon_connected().await {
                self.server
                    .kick_bots(&self.settings, self.rcon.as_mut().unwrap())
                    .await;
            }
        }

        // Send chat alerts
        if self.alert_timer.update() {
            if self.rcon_connected().await {
                self.server
                    .announce_bots(&self.settings, self.rcon.as_mut().unwrap())
                    .await;
            }
        }
    }

    fn render(&mut self, ctx: &mut glium_app::context::Context) {
        let mut target = ctx.dis.draw();
        target.clear_color_and_depth((0.5, 0.7, 0.8, 1.0), 1.0);

        let (_, shapes) = ctx.gui.run(&ctx.dis, |ctx| {

            // Tracks if the settings need to be saved
            let mut settings_changed = false;

            // Top menu bar
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
                                    self.settings.tf2_directory = dir;
                                    self.log = LogWatcher::use_directory(&self.settings.tf2_directory);
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
                                    match self.bot_checker.add_regex_list(&dir) {
                                        Ok(_) => {
                                            self.message = format!("Added {} as a regex list", &dir.split("/").last().unwrap());
                                        },
                                        Err(e) => {
                                            self.message = format!("{}", e);
                                        }
                                    }
                                    self.settings.regex_lists.push(dir);
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
                                    match self.bot_checker.add_steamid_list(&dir) {
                                        Ok(_) => {
                                            self.message = format!("Added {} as a steamid list", &dir.split("/").last().unwrap());
                                        },
                                        Err(e) => {
                                            self.message = format!("{}", e);
                                        }
                                    }
                                    self.settings.uuid_lists.push(dir);
                                    settings_changed = true;
                                },
                                None => {}
                            }
                        }

                    });
                });
            });

            // Message and eframe/egui credits
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

            // Left panel
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Settings");

                    ui.horizontal(|ui| {
                        ui.label("User: ");
                        settings_changed |= ui.text_edit_singleline(&mut self.settings.user).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("RCon Password: ");
                        settings_changed |= ui.text_edit_singleline(&mut self.settings.rcon_password).changed();
                    });

                    ui.label("");

                    ui.horizontal(|ui| {
                        settings_changed |= ui.add(egui::DragValue::new(&mut self.settings.refresh_period)
                        .speed(0.1)
                        .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                        ui.label("Refresh Period");
                    });

                    settings_changed |= ui.checkbox(&mut self.settings.kick, "Kick Bots").changed();
                    if self.settings.kick {
                        ui.horizontal(|ui| {
                            settings_changed |= ui.add(egui::DragValue::new(&mut self.settings.kick_period)
                            .speed(0.1)
                            .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                            ui.label("Kick Period");
                        });
                    }

                    
                    settings_changed |= ui.checkbox(&mut self.settings.join_alert, "Join Alerts").changed();
                    settings_changed |= ui.checkbox(&mut self.settings.chat_reminders, "Chat Reminders").changed();

                    if self.settings.join_alert || self.settings.chat_reminders {
                        ui.horizontal(|ui| {
                            settings_changed |= ui.add(egui::DragValue::new(&mut self.settings.alert_period)
                            .speed(0.1)
                            .clamp_range(RangeInclusive::new(0.5, 60.0))).changed();
                            ui.label("Chat Alert Period");
                        });
                    }

                    ui.label("");
                    ui.heading("Bot Detection Rules");

                    ui.checkbox(&mut self.settings.record_steamids, &format!("Automatically append bot steamids to {}", DEFAULT_STEAMID_LIST));

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

            // Main window with info and players
            egui::CentralPanel::default().show(ctx, |ui| {

                if self.log.is_none() {

                    ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");

                    ui.label("Instructions:");

                    ui.horizontal(|ui| {
                        ui.label("1. Add");
                        copy_label(&mut self.message, "-condebug -conclearlog", ui);
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
                                    self.settings.tf2_directory = dir;
                                    self.log = LogWatcher::use_directory(&self.settings.tf2_directory);
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
                    match &self.rcon {
                        // Connected and good
                        Ok(_) => {

                            if self.server.players.is_empty() {
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

                                        for (_, p) in &mut self.server.players {

                                            if p.team == Team::Invaders {
                                                render_player(&mut cols[0], &self.settings, &mut self.message, p, width);
                                            }

                                            if p.team == Team::Defenders {
                                                render_player(&mut cols[1], &self.settings, &mut self.message, p, width);
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
                                        copy_label(&mut self.message, &format!("rcon_password {}", &self.settings.rcon_password), ui);
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
                                        copy_label(&mut self.message, "net_start", ui);
                                        ui.label("?");
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Does your TF2 launch option include");
                                        copy_label(&mut self.message, "-usercon", ui);
                                        ui.label("?");
                                    });
                                }
                            }
                        }
                    }
                }
            });

            // Export settings if they've changed
            if settings_changed {
                let _new_dir = std::fs::create_dir("cfg");
                match self.settings.export("cfg/settings.json") {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Failed to export settings");
                        self.message = e.to_string();
                    }
                }
            }

        });

        ctx.gui.paint(&ctx.dis, &mut target, shapes);
        target.finish().unwrap();
    }

    fn close(&mut self) {}
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

// Ui for a player
fn render_player(ui: &mut Ui, set: &Settings, mes: &mut String, p: &mut Player, width: f32) {
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

        ui.collapsing(text, |ui| {
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
                if ui.selectable_label(false, "Copy Name").clicked() {
                    let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                        ClipboardProvider::new();
                    ctx.unwrap().set_contents(p.name.clone()).unwrap();
                    mes.clear();
                    mes.push_str(&format!("Copied \"{}\"", p.name));
                }
                if ui.selectable_label(false, "Copy SteamID").clicked() {
                    let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                        ClipboardProvider::new();
                    ctx.unwrap().set_contents(p.steamid.clone()).unwrap();
                    mes.clear();
                    mes.push_str(&format!("Copied \"{}\"", p.steamid));
                }
            });

            if p.bot {
                ui.horizontal(|ui| {
                    let lab = ui.selectable_label(
                        false,
                        RichText::new("Save SteamID").color(Color32::LIGHT_RED),
                    );
                    if lab.clicked() {
                        p.export_steamid();
                        *mes = format!("Saved {}'s SteamID to {}", &p.name, DEFAULT_STEAMID_LIST);
                    }
                    lab.on_hover_text(
                        RichText::new("This player will always be recognized as a bot")
                            .color(Color32::RED),
                    );

                    let lab = ui.selectable_label(
                        false,
                        RichText::new("Save Name").color(Color32::LIGHT_RED),
                    );
                    if lab.clicked() {
                        p.export_regex();
                        *mes = format!("Saved {}'s Name to {}", &p.name, DEFAULT_REGEX_LIST);
                    }
                    lab.on_hover_text(
                        RichText::new("Players with this name will always be recognized as a bot")
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

                if p.state == State::Spawning {
                    ui.colored_label(Color32::YELLOW, "Joining");
                }
            });
        });
    });
}

/// Truncates a &str
fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
