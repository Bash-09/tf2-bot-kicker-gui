use std::ops::RangeInclusive;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{Color32, ComboBox, Context, Id, Label, RichText, Separator, Ui};
use glium_app::utils::persistent_window::{PersistentWindow, PersistentWindowManager};

use crate::{
    command_manager::CommandManager,
    logwatcher::LogWatcher,
    player_checker::PlayerRecord,
    server::player::{Player, PlayerState, PlayerType, Team},
    state::State,
    version::{self, VersionResponse},
};

use self::{
    player_windows::view_players_window,
    regex_windows::{new_regex_window, view_regexes_window},
};

pub mod player_windows;
pub mod regex_windows;

pub fn render(
    gui_ctx: &Context,
    windows: &mut PersistentWindowManager<State>,
    state: &mut State,
    cmd: &mut CommandManager,
) {
    // Top menu bar
    egui::TopBottomPanel::top("top_panel").show(gui_ctx, |ui| {
        // File
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Set TF2 Directory").clicked() {
                    match rfd::FileDialog::new().pick_folder() {
                        Some(pb) => {
                            let dir = match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                Ok(pb) => pb.to_string_lossy().to_string(),
                                Err(_) => pb.to_string_lossy().to_string(),
                            };
                            state.settings.tf2_directory = dir;
                            state.log = LogWatcher::use_directory(&state.settings.tf2_directory);
                        }
                        None => {}
                    }
                }
            });

            // Import Regexes and SteamIDs
            ui.menu_button("Import", |ui| {
                let mut import_list: Option<PlayerType> = None;
                if ui.button("Import SteamIDs as Bots").clicked() {
                    import_list = Some(PlayerType::Bot);
                }
                if ui.button("Import SteamIDs as Cheaters").clicked() {
                    import_list = Some(PlayerType::Cheater);
                }
                if ui.button("Import SteamIDs as Suspicious").clicked() {
                    import_list = Some(PlayerType::Suspicious);
                }

                if let Some(player_type) = import_list {
                    match rfd::FileDialog::new().set_directory("cfg").pick_file() {
                        Some(pb) => {
                            let dir = match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                Ok(pb) => pb.to_string_lossy().to_string(),
                                Err(_) => pb.to_string_lossy().to_string(),
                            };

                            match state
                                .player_checker
                                .read_from_steamid_list(&dir, player_type)
                            {
                                Ok(_) => {
                                    log::info!(
                                        "{}",
                                        format!(
                                            "Added {} as a steamid list",
                                            &dir.split('/').last().unwrap()
                                        )
                                    );
                                }
                                Err(e) => {
                                    log::error!("Failed to add steamid list: {}", format!("{}", e));
                                }
                            }
                        }
                        None => {}
                    }
                }

                if ui.button("Import regex list").clicked() {
                    if let Some(pb) = rfd::FileDialog::new().set_directory("cfg").pick_file() {
                        if let Err(e) = state.player_checker.read_regex_list(pb) {
                            log::error!("Failed to import regexes: {:?}", e);
                        }
                    }
                }
            });

            // Saved Data
            ui.menu_button("Saved Data", |ui| {
                if ui.button("Regexes").clicked() {
                    windows.push(view_regexes_window());
                }

                if ui.button("Saved Players").clicked() {
                    windows.push(view_players_window());
                }
            });

            if ui.button("Recent players").clicked() {
                windows.push(player_windows::recent_players_window());
            }

            if ui.button("Check for updates").clicked() && state.latest_version.is_none() {
                state.latest_version = Some(VersionResponse::request_latest_version());
                state.force_latest_version = true;
            }
        });
    });

    // Message and eframe/egui credits
    egui::TopBottomPanel::bottom("bottom_panel").show(gui_ctx, |ui| {
        // Credits at the bottom left
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("powered by ");
            ui.hyperlink_to("egui", "https://github.com/emilk/egui");

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                ui.label(version::VERSION);
            });
        });
    });

    // Left panel
    egui::SidePanel::left("side_panel").default_width(230.0).show(gui_ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {

            ui.heading("Settings");

            ui.horizontal(|ui| {
                ui.label("User: ");
                ui.text_edit_singleline(&mut state.settings.user);
            });

            ui.horizontal(|ui| {
                ui.label("RCon Password: ");
                ui.text_edit_singleline(&mut state.settings.rcon_password);
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut state.settings.refresh_period)
                        .speed(0.1)
                        .clamp_range(RangeInclusive::new(0.5, 60.0)),
                );
                ui.label("Refresh Period").on_hover_text("Time between refreshing the server information.");
            });

            ui.checkbox(&mut state.settings.paused, "Pause actions").on_hover_text("Prevents the program from calling any votekicks or sending chat messages.");

            ui.add(Separator::default().spacing(20.0));
            ui.heading("Kicking");

            ui.checkbox(&mut state.settings.kick_bots, "Kick Bots").on_hover_text("Automatically attempt to call votekicks on bots.");
            ui.checkbox(&mut state.settings.kick_cheaters, "Kick Cheaters").on_hover_text("Automatically attempt to call votekicks on cheaters.");

            ui.horizontal(|ui| {
                ui.add_enabled(state.settings.kick_bots || state.settings.kick_cheaters,
                    egui::DragValue::new(&mut state.settings.kick_period)
                        .speed(0.1)
                        .clamp_range(RangeInclusive::new(0.5, 60.0)),
                );
                ui.add_enabled(state.settings.kick_bots || state.settings.kick_cheaters,
                Label::new("Kick Period")).on_hover_text("Time between attempting to kick bots or cheaters.");
            });

            ui.add(Separator::default().spacing(20.0));
            ui.heading("Chat Messages");

            ui.checkbox(&mut state.settings.announce_bots, "Announce Bots").on_hover_text("Send a chat message indicating Bots joining the server.");
            ui.checkbox(&mut state.settings.announce_cheaters, "Announce Cheaters").on_hover_text("Send a chat message indicating cheaters joining the server.");
            ui.checkbox(&mut state.settings.announce_namesteal, "Announce Name-stealing").on_hover_text("Send a chat message when an account's name is changed to imitate another player (This is not affected by the chat message period).");
            ui.checkbox(&mut state.settings.dont_announce_common_names, "Ignore Bots with common names").on_hover_text("Don't announce bots who's name matches saved regexes, to avoid announcing well-known bots (e.g. DoesHotter, m4gic).");

            ui.horizontal(|ui| {
                ui.add_enabled(state.settings.announce_bots || state.settings.announce_cheaters,
                    egui::DragValue::new(&mut state.settings.alert_period)
                        .speed(0.1)
                        .clamp_range(RangeInclusive::new(0.5, 60.0)),
                );
                ui.add_enabled(state.settings.announce_bots || state.settings.announce_cheaters,
                    Label::new("Chat Message Period")).on_hover_text("Time between sending chat messages.");
            });

            ui.add(Separator::default().spacing(20.0));
            ui.heading("Bot Detection");

            ui.checkbox(&mut state.settings.mark_name_stealers, "Mark accounts with a stolen name as bots")
                .on_hover_text("Accounts that change their name to another account's name will be automatically marked as a name-stealing bot.");
        });
    });

    // Main window with info and players
    egui::CentralPanel::default().show(gui_ctx, |ui| {

        if state.log.is_none() {
            ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");
            ui.label("Instructions:");
            ui.horizontal(|ui| {
                ui.label("1. Add");
                copy_label("-condebug -conclearlog -usercon", ui);
                ui.label("to your TF2 launch options and start the game.");
            });

            ui.horizontal(|ui| {
                ui.label("2. Click");
                if ui.button("Set your TF2 directory").clicked() {
                    match rfd::FileDialog::new().pick_folder() {
                        Some(pb) => {
                            let dir = match pb.strip_prefix(std::env::current_dir().unwrap()) {
                                Ok(pb) => {
                                    pb.to_string_lossy().to_string()
                                },
                                Err(_) => {
                                    pb.to_string_lossy().to_string()
                                }
                            };
                            state.settings.tf2_directory = dir;
                            state.log = LogWatcher::use_directory(&state.settings.tf2_directory);
                        },
                        None => {}
                    }
                }
                ui.label("and navigate to your Team Fortress 2 folder");
            });
            ui.label("3. Start the program and enjoy the game!\n\n");
            ui.label("Note: If you have set your TF2 directory but are still seeing this message, ensure you have added the launch options and launched the game before trying again.");


        } else {
            match cmd.connected(&state.settings.rcon_password) {
                // Connected and good
                Ok(_) => {
                    if state.server.players.is_empty() {
                        ui.label("Not currently connected to a server.");
                    } else {
                        render_players(ui, state, windows, cmd);
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
                                copy_label(&format!("rcon_password {}", &state.settings.rcon_password), ui);
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
                                copy_label("net_start", ui);
                                ui.label("?");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Does your TF2 launch option include");
                                copy_label("-usercon", ui);
                                ui.label("?");
                            });
                        }
                    }
                }
            }
        }
    });
}

// Make a selectable label which copies it's text to the clipboard on click
fn copy_label(text: &str, ui: &mut Ui) {
    let lab = ui.selectable_label(false, text);
    if lab.clicked() {
        let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> = ClipboardProvider::new();
        if let Ok(mut ctx) = ctx {
            ctx.set_contents(text.to_string()).ok();
        }
    }
    lab.on_hover_text("Copy");
}

// u32 -> minutes:seconds
fn format_time(time: u32) -> String {
    format!("{:2}:{:02}", time / 60, time % 60)
}

const TRUNC_LEN: usize = 40;

/// Truncates a &str
fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

// Ui for a player
fn render_players(
    ui: &mut Ui,
    state: &mut State,
    windows: &mut PersistentWindowManager<State>,
    cmd: &mut CommandManager,
) {
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
            playerlist.sort_by(|a, b| b.time.cmp(&a.time));

            for player in playerlist {
                let team_ui = match player.team {
                    Team::Invaders => &mut cols[0],
                    Team::Defenders => &mut cols[1],
                    Team::None => continue,
                };

                team_ui.horizontal(|ui| {
                    ui.set_width(width);

                    let text;
                    if player.steamid32 == state.settings.user {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN))
                            .color(Color32::GREEN);
                    } else if player.player_type == PlayerType::Bot
                        || player.player_type == PlayerType::Cheater
                    {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN))
                            .color(player.player_type.color(ui));
                    } else if player.stolen_name {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN))
                            .color(Color32::YELLOW);
                    } else {
                        text = egui::RichText::new(truncate(&player.name, TRUNC_LEN));
                    }

                    // Player Type combobox
                    if player_type_combobox(&player.steamid32, &mut player.player_type, ui) {
                        if player.player_type == PlayerType::Player && player.notes.is_empty() {
                            state.player_checker.players.remove(&player.steamid32);
                        } else {
                            state.player_checker.update_player(player);
                        }
                    }

                    // Player name button
                    ui.style_mut().visuals.widgets.inactive.bg_fill =
                        ui.style().visuals.window_fill();

                    let header = ui.menu_button(text, |ui| {
                        if ui.button("Copy SteamID32").clicked() {
                            let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                ClipboardProvider::new();
                            ctx.unwrap().set_contents(player.steamid32.clone()).unwrap();
                            log::info!("{}", format!("Copied \"{}\"", player.steamid32));
                        }

                        if ui.button("Copy SteamID64").clicked() {
                            let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                ClipboardProvider::new();
                            ctx.unwrap()
                                .set_contents(format!("{}", player.steamid64))
                                .unwrap();
                            log::info!("{}", format!("Copied \"{}\"", player.steamid64));
                        }

                        if ui.button("Copy Name").clicked() {
                            let ctx: Result<ClipboardContext, Box<dyn std::error::Error>> =
                                ClipboardProvider::new();
                            ctx.unwrap().set_contents(player.name.clone()).unwrap();
                            log::info!("{}", format!("Copied \"{}\"", player.name));
                        }

                        // Copy SteamID and Name buttons
                        if ui.button("Edit Notes").clicked() {
                            windows.push(create_edit_notes_window(player.get_record()));
                        }

                        ui.menu_button(RichText::new("Other actions").color(Color32::RED), |ui| {
                            // Call votekick button
                            if ui
                                .button(RichText::new("Call votekick").color(Color32::RED))
                                .clicked()
                            {
                                cmd.kick_player(&player.userid);
                            }

                            // Save Name button
                            if player.player_type == PlayerType::Bot
                                || player.player_type == PlayerType::Cheater
                            {
                                let but = ui.button(RichText::new("Save Name").color(Color32::RED));
                                if but.clicked() {
                                    windows.push(new_regex_window(player.get_export_regex()));
                                }
                                but.on_hover_text(
                                    RichText::new(
                                        "Players with this name will always be recognized as a bot",
                                    )
                                    .color(Color32::RED),
                                );
                            }
                        });
                    });

                    // Notes / Stolen name warning
                    if player.stolen_name || !player.notes.is_empty() {
                        header.response.on_hover_ui(|ui| {
                            if player.stolen_name {
                                ui.label(
                                    RichText::new(
                                        "A player with this name is already on the server.",
                                    )
                                    .color(Color32::YELLOW),
                                );
                            }
                            if !player.notes.is_empty() {
                                ui.label(&player.notes);
                            }
                        });
                    }

                    // Cheater, Bot and Joining labels
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.add_space(15.0);

                        // Time
                        ui.label(&format_time(player.time));

                        if !player.notes.is_empty() {
                            ui.label("☑");
                        }

                        // Cheater / Bot / Joining
                        if player.player_type != PlayerType::Player {
                            ui.add(Label::new(player.player_type.rich_text()));
                        }
                        if player.state == PlayerState::Spawning {
                            ui.add(Label::new(RichText::new("Joining").color(Color32::YELLOW)));
                        }
                    });
                });
            }
        });
    });
}

fn create_edit_notes_window(mut record: PlayerRecord) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _, gui_ctx, state| {
        let mut open = true;
        let mut saved = false;
        egui::Window::new(format!("Edit notes for {}", &record.steamid))
            .id(Id::new(id))
            .open(&mut open)
            .show(gui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.text_edit_multiline(&mut record.notes);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            saved = true;
                            if let Some(p) = &mut state.server.players.get_mut(&record.steamid) {
                                p.notes = record.notes.clone();
                            }

                            if record.notes.is_empty() && record.player_type == PlayerType::Player {
                                state.player_checker.players.remove(&record.steamid);
                                return;
                            }

                            state.player_checker.update_player_record(record.clone());
                        }
                    });
                });
            });
        open & !saved
    }))
}

fn create_dialog_box(title: String, text: String) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _, ctx, _| {
        let mut open = true;

        egui::Window::new(&title)
            .id(Id::new(id))
            .open(&mut open)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(&text);
            });

        open
    }))
}

/// Creates a dropdown combobox to select a player type
pub fn player_type_combobox(id: &str, player_type: &mut PlayerType, ui: &mut Ui) -> bool {
    let mut changed = false;
    ComboBox::from_id_source(id)
        .selected_text(player_type.rich_text())
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(
                    player_type,
                    PlayerType::Player,
                    PlayerType::Player.rich_text(),
                )
                .clicked();
            changed |= ui
                .selectable_value(player_type, PlayerType::Bot, PlayerType::Bot.rich_text())
                .clicked();
            changed |= ui
                .selectable_value(
                    player_type,
                    PlayerType::Cheater,
                    PlayerType::Cheater.rich_text(),
                )
                .clicked();
            changed |= ui
                .selectable_value(
                    player_type,
                    PlayerType::Suspicious,
                    PlayerType::Suspicious.rich_text(),
                )
                .clicked();
        });
    changed
}
