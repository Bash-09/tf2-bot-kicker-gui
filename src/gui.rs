use std::ops::RangeInclusive;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{CollapsingHeader, Color32, ComboBox, Context, Id, Label, RichText, Ui};
use glium_app::utils::persistent_window::{PersistentWindow, PersistentWindowManager};
use regex::Regex;

use crate::{
    append_line,
    logwatcher::LogWatcher,
    server::player::{Player, PlayerState, PlayerType, Team},
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
                            match state.player_checker.read_regex_list(&dir) {
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

                let mut import_list: Option<PlayerType> = None;
                if ui.button("Import SteamIDs as Bots").clicked() {
                    import_list = Some(PlayerType::Bot);
                }
                if ui.button("Import SteamIDs as Cheaters").clicked() {
                    import_list = Some(PlayerType::Cheater);
                }

                if let Some(player_type) = import_list {
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

                            match state.player_checker.read_from_steamid_list(&dir, player_type) {
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
    format!("{:2}:{:02}", time / 60, time % 60)
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

            // Render players
            let mut playerlist: Vec<&mut Player> = state.server.players.values_mut().collect();
            playerlist.sort_by(|a, b| {
                b.time.cmp(&a.time)
            });

            for player in playerlist {
                let saved = state.player_checker.players.contains_key(&player.steamid);

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
                    } else if player.player_type == PlayerType::Bot || player.player_type == PlayerType::Cheater {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN)).color(player.player_type.color(ui));
                    } else if player.stolen_name {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN)).color(Color32::YELLOW);
                    } else {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN));
                    }

                    let header = CollapsingHeader::new(text)
                    .id_source(&player.userid)
                    .show(ui, |ui| {

                        // Player Type combobox
                        ui.horizontal(|ui| {
                            ui.label("Player Type");
                            let mut changed = false;
                            ComboBox::from_id_source(&player.steamid)
                            .selected_text(RichText::new(format!("{:?}", player.player_type)).color(player.player_type.color(ui)))
                            .show_ui(ui, |ui| {
                                changed |= ui.selectable_value(&mut player.player_type, PlayerType::Player, "Player").clicked();
                                changed |= ui.selectable_value(&mut player.player_type, PlayerType::Bot, RichText::new("Bot").color(PlayerType::Bot.color(ui))).clicked();
                                changed |= ui.selectable_value(&mut player.player_type, PlayerType::Cheater, RichText::new("Cheater").color(PlayerType::Cheater.color(ui))).clicked();
                            });

                            if changed && saved {
                                state.player_checker.update_player(player);
                            }
                        });

                        // Copy SteamID and Name buttons
                        ui.horizontal(|ui| {
                            if ui.button("Copy SteamID").clicked() {
                                let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                    ClipboardProvider::new();
                                ctx.unwrap().set_contents(player.steamid.clone()).unwrap();
                                state.message.clear();
                                state.message.push_str(&format!("Copied \"{}\"", player.steamid));
                                log::info!("{}", state.message);
                            }
                            if ui.button("Copy Name").clicked() {
                                let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                    ClipboardProvider::new();
                                ctx.unwrap().set_contents(player.name.clone()).unwrap();
                                state.message.clear();
                                state.message.push_str(&format!("Copied \"{}\"", player.name));
                                log::info!("{}", state.message);
                            }
                        });

                        // Save and Delete buttons
                        if !saved {
                            let but = ui.button("Save Player")
                                .on_hover_text("This player's information will be saved in case you see them again.");
                            if but.clicked() {
                                state.player_checker.update_player(player);
                            }
                        } else {
                            let but = ui.button("Delete Player")
                                .on_hover_text("This player will not be recognised next time you see them.");
                            if but.clicked() {
                                state.player_checker.remove_player(player);
                            }
                        }

                        // Save Name button
                        if player.player_type == PlayerType::Bot || player.player_type == PlayerType::Cheater {
                            let but = ui.button(RichText::new("Save Name").color(Color32::RED));
                            if but.clicked() {
                                windows.push(create_export_regex_window(player.get_export_regex()));
                            }
                            but.on_hover_text(
                                RichText::new(
                                    "Players with this name will always be recognized as a bot",
                                )
                                .color(Color32::RED),
                            );
                        }

                        // Call votekick button
                        if ui.button(RichText::new("Call votekick").color(Color32::RED)).clicked() {
                            log::error!("Not implemented yet");
                        }
                    });

                    // Notes / Stolen name warning
                    if player.stolen_name || player.notes.is_some() {
                        header.header_response.on_hover_ui(|ui| {
                            if player.stolen_name {
                                ui.label(RichText::new("A player with this name is already on the server.").color(Color32::YELLOW));
                            }
                            if let Some(notes) = &player.notes {
                                ui.label(notes);
                            }
                        });
                    }

                    // Cheater, Bot and Joining labels
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(15.0);
                            ui.label(&format_time(player.time));

                            if player.player_type == PlayerType::Cheater {
                                ui.add(Label::new(RichText::new("Cheater").color(PlayerType::Cheater.color(ui))));
                            }
                            if player.player_type == PlayerType::Bot {
                                ui.add(Label::new(RichText::new("Bot").color(PlayerType::Bot.color(ui))));
                            }
                            if player.state == PlayerState::Spawning {
                                ui.add(Label::new(RichText::new("Joining").color(Color32::YELLOW)));
                            }
                        });
                    });
                });
            }

        });
    });
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
                                state.player_checker.append_regex(reg);
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
