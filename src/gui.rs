use std::ops::RangeInclusive;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{CollapsingHeader, Color32, Context, Id, RichText, Ui};
use glium_app::utils::persistent_window::{PersistentWindow, PersistentWindowManager};
use regex::Regex;

use crate::{
    append_line,
    logwatcher::LogWatcher,
    server::player::{self, Team},
    state::State,
};

pub fn render(gui_ctx: &Context, windows: &mut PersistentWindowManager<State>, state: &mut State) {
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
                                }
                                Err(_) => {
                                    dir = pb.to_string_lossy().to_string();
                                }
                            }
                            state.settings.tf2_directory = dir;
                            state.log = LogWatcher::use_directory(&state.settings.tf2_directory);
                            settings_changed = true;
                        }
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
                                }
                                Err(_) => {
                                    dir = pb.to_string_lossy().to_string();
                                }
                            }
                            match state.bot_checker.add_regex_list(&dir) {
                                Ok(_) => {
                                    state.message = format!(
                                        "Added {} as a regex list",
                                        &dir.split("/").last().unwrap()
                                    );
                                    log::info!("{}", state.message);
                                }
                                Err(e) => {
                                    state.message = format!("{}", e);
                                    log::error!("{}", state.message);
                                }
                            }
                            state.settings.regex_lists.push(dir);
                            settings_changed = true;
                        }
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
                                }
                                Err(_) => {
                                    dir = pb.to_string_lossy().to_string();
                                }
                            }
                            match state.bot_checker.add_steamid_list(&dir) {
                                Ok(_) => {
                                    state.message = format!(
                                        "Added {} as a steamid list",
                                        &dir.split("/").last().unwrap()
                                    );
                                    log::info!("{}", state.message);
                                }
                                Err(e) => {
                                    state.message = format!("{}", e);
                                    log::error!("Failed to add steamid list: {}", state.message);
                                }
                            }
                            state.settings.steamid_lists.push(dir);
                            settings_changed = true;
                        }
                        None => {}
                    }
                }
            });
        });
    });

    // Message and eframe/egui credits
    egui::TopBottomPanel::bottom("bottom_panel").show(gui_ctx, |ui| {
        // Display a little bit of information
        ui.label(&state.message);

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
                settings_changed |= ui.text_edit_singleline(&mut state.settings.user).changed();
            });

            ui.horizontal(|ui| {
                ui.label("RCon Password: ");
                settings_changed |= ui
                    .text_edit_singleline(&mut state.settings.rcon_password)
                    .changed();
            });

            ui.label("");

            ui.horizontal(|ui| {
                settings_changed |= ui
                    .add(
                        egui::DragValue::new(&mut state.settings.refresh_period)
                            .speed(0.1)
                            .clamp_range(RangeInclusive::new(0.5, 60.0)),
                    )
                    .changed();
                ui.label("Refresh Period");
            });

            settings_changed |= ui.checkbox(&mut state.settings.kick, "Kick Bots").changed();
            if state.settings.kick {
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut state.settings.kick_period)
                                .speed(0.1)
                                .clamp_range(RangeInclusive::new(0.5, 60.0)),
                        )
                        .changed();
                    ui.label("Kick Period");
                });
            }

            settings_changed |= ui
                .checkbox(&mut state.settings.join_alert, "Join Alerts")
                .changed();
            settings_changed |= ui
                .checkbox(&mut state.settings.chat_reminders, "Chat Reminders")
                .changed();

            if state.settings.join_alert || state.settings.chat_reminders {
                ui.horizontal(|ui| {
                    settings_changed |= ui
                        .add(
                            egui::DragValue::new(&mut state.settings.alert_period)
                                .speed(0.1)
                                .clamp_range(RangeInclusive::new(0.5, 60.0)),
                        )
                        .changed();
                    ui.label("Chat Alert Period");
                });
            }

            ui.label("");
            ui.heading("Bot Detection Rules");

            settings_changed |= ui
                .checkbox(
                    &mut state.settings.record_steamids,
                    &format!("Automatically record bot SteamIDs"),
                )
                .changed();

            ui.label("");
            ui.collapsing("Regex Lists", |ui| {
                let mut ind: Option<usize> = None;
                for (i, l) in state.settings.regex_lists.iter().enumerate() {
                    let active = l.eq(&state.settings.regex_list);
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
                            let set =
                                set.on_hover_text("Recorded Regexes will be added to this file");
                            if set.clicked() {
                                state.settings.regex_list = l.clone();
                                settings_changed = true;
                            }
                        }
                    });
                }
                match ind {
                    Some(i) => {
                        state.settings.regex_lists.remove(i);
                        settings_changed = true;
                    }
                    None => {}
                }
            });

            ui.collapsing("SteamID Lists", |ui| {
                let mut ind: Option<usize> = None;
                for (i, l) in state.settings.steamid_lists.iter().enumerate() {
                    let active = l.eq(&state.settings.steamid_list);
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
                            let set =
                                set.on_hover_text("Recorded SteamIDs will be added to this file");
                            if set.clicked() {
                                state.settings.steamid_list = l.clone();
                                settings_changed = true;
                            }
                        }
                    });
                }
                match ind {
                    Some(i) => {
                        state.settings.steamid_lists.remove(i);
                        settings_changed = true;
                    }
                    None => {}
                }
            });
        });
    });

    // Main window with info and players
    egui::CentralPanel::default().show(gui_ctx, |ui| {

            if state.log.is_none() {

                ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");

                ui.label("Instructions:");

                ui.horizontal(|ui| {
                    ui.label("1. Add");
                    copy_label(&mut state.message, "-condebug -conclearlog -usercon", ui);
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
                                state.settings.tf2_directory = dir;
                                state.log = LogWatcher::use_directory(&state.settings.tf2_directory);
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
                match &state.rcon {
                    // Connected and good
                    Ok(_) => {

                        if state.server.players.is_empty() {
                            ui.label("Not currently connected to a server.");
                        } else {

                            render_players(ui, state, windows);
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
                                    copy_label(&mut state.message, &format!("rcon_password {}", &state.settings.rcon_password), ui);
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
                                    copy_label(&mut state.message, "net_start", ui);
                                    ui.label("?");
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Does your TF2 launch option include");
                                    copy_label(&mut state.message, "-usercon", ui);
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
        match state.settings.export("cfg/settings.json") {
            Ok(_) => {
                log::info!("Saved settings");
            }
            Err(e) => {
                log::error!("Failed to export settings: {:?}", e);
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

// Ui for a player
fn render_players(ui: &mut Ui, state: &mut State, windows: &mut PersistentWindowManager<State>) {
    let width = (ui.available_width() - 5.0) / 2.0;

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

            for (_, player) in &mut state.server.players {

                let team_ui = match player.team {
                    Team::Invaders => &mut cols[0],
                    Team::Defenders => &mut cols[1],
                    Team::None => continue,
                };

                team_ui.horizontal(|ui| {
                    ui.set_width(width);

                    let text;
                    if player.steamid == state.settings.user {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN)).color(Color32::GREEN);
                    } else if player.bot {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN)).color(Color32::RED);
                    } else {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN));
                    }

                    CollapsingHeader::new(text)
                        .id_source(&player.userid)
                        .show(ui, |ui| {
                            let prefix = match player.bot {
                                true => "NOT ",
                                false => "",
                            };
                            let mut text = RichText::new(&format!("Mark as {}Bot", prefix));
                            if !player.bot {
                                text = text.color(Color32::LIGHT_RED);
                            } else {
                                text = text.color(Color32::LIGHT_GREEN);
                            }

                            if ui.selectable_label(false, text).clicked() {
                                player.bot = !player.bot;
                            }

                            ui.horizontal(|ui| {
                                if ui.button("Copy Name").clicked() {
                                    let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                        ClipboardProvider::new();
                                    ctx.unwrap().set_contents(player.name.clone()).unwrap();
                                    state.message.clear();
                                    state.message.push_str(&format!("Copied \"{}\"", player.name));
                                    log::info!("{}", state.message);
                                }
                                if ui.button("Copy SteamID").clicked() {
                                    let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                        ClipboardProvider::new();
                                    ctx.unwrap().set_contents(player.steamid.clone()).unwrap();
                                    state.message.clear();
                                    state.message.push_str(&format!("Copied \"{}\"", player.steamid));
                                    log::info!("{}", state.message);
                                }
                            });

                            if player.bot {
                                ui.horizontal(|ui| {
                                    let but = ui.button(RichText::new("Save Name").color(Color32::LIGHT_RED));
                                    if but.clicked() {
                                        windows.push(create_export_regex_window(player.get_export_regex()));
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
                                        windows.push(create_export_steamid_window(player.get_export_steamid()));
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
                            ui.label(&format_time(player.time));

                            if player.bot {
                                ui.colored_label(Color32::RED, "BOT");
                            }

                            if player.state == player::State::Spawning {
                                ui.colored_label(Color32::YELLOW, "Joining");
                            }
                        });
                    });
                });
            }

        });
    });
}

fn create_export_steamid_window(mut steamid: String) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, gui_ctx, state| {
        let mut open = true;
        let mut exported = false;

        egui::Window::new("Export SteamID")
            .id(Id::new(id))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .collapsible(false)
            .show(gui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.text_edit_singleline(&mut steamid);
                    if ui.button("Confirm").clicked() {
                        append_line(&steamid, &state.settings.steamid_list);
                        state.bot_checker.append_steamid(&steamid);
                        state.message =
                            format!("Saved \"{}\" to {}", &steamid, &state.settings.steamid_list);
                        log::info!("{}", state.message);
                        exported = true;
                    }
                });
            });

        open & !exported
    }))
}

fn create_export_regex_window(mut regex: String) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, gui_ctx, state| {
        let mut open = true;

        let mut exported = false;
        egui::Window::new("Export Name/Regex")
            .id(Id::new(id))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .collapsible(false)
            .show(gui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.text_edit_singleline(&mut regex);
                    if ui.button("Confirm").clicked() {
                        let reg = Regex::new(&regex);
                        match reg {
                            Ok(reg) => {
                                append_line(&regex, &state.settings.regex_list);
                                state.bot_checker.append_regex(reg);
                                state.message = format!(
                                    "Saved \"{}\" to {}",
                                    &regex, &state.settings.regex_list
                                );
                                log::info!("{}", state.message);
                                exported = true;
                            }
                            Err(e) => {
                                state.message = format!("Invalid Regex: {}", e);
                                log::error!("{}", state.message);
                            }
                        }
                    }
                });
            });

        open & !exported
    }))
}
