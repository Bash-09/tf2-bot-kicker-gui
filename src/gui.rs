use std::ops::RangeInclusive;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{Color32, Id, Label, Separator, Ui};
use wgpu_app::utils::persistent_window::{PersistentWindow, PersistentWindowManager};

use crate::{
    io::{command_manager::CommandManager, IORequest},
    server::player::{Player, PlayerType, Team, UserAction},
    state::State,
    steamapi,
    version::{self, VersionResponse},
};

use self::{
    chat_window::view_chat_window, player_windows::saved_players_window,
    regex_windows::view_regexes_window,
};

pub mod chat_window;
pub mod player_windows;
pub mod regex_windows;

pub fn render(
    gui_ctx: &egui::Context,
    windows: &mut PersistentWindowManager<State>,
    state: &mut State,
) {
    // Top menu bar
    egui::TopBottomPanel::top("top_panel").show(gui_ctx, |ui| {
        // File
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Set TF2 Directory").clicked() {
                    if let Some(pb) = rfd::FileDialog::new().pick_folder() {
                        let dir = match pb.strip_prefix(std::env::current_dir().unwrap()) {
                            Ok(pb) => pb.to_string_lossy().to_string(),
                            Err(_) => pb.to_string_lossy().to_string(),
                        };
                        state.settings.tf2_directory = dir;
                        state.io.send(crate::io::IORequest::UpdateDirectory(
                            state.settings.tf2_directory.clone(),
                        ));
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
                    if let Some(pb) = rfd::FileDialog::new().set_directory("cfg").pick_file() {
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
                    windows.push(saved_players_window());
                }
            });

            if ui.button("Recent players").clicked() {
                windows.push(player_windows::recent_players_window());
            }

            if ui.button("Chat settings").clicked() {
                windows.push(view_chat_window());
            }

            if ui.button("Check for updates").clicked() && state.latest_version.is_none() {
                state.latest_version = Some(VersionResponse::request_latest_version());
                state.force_latest_version = true;
            }

            if ui.button("Steam API").clicked() {
                windows.push(steamapi::create_set_api_key_window(
                    state.settings.steamapi_key.clone(),
                ));
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
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
                if ui.text_edit_singleline(&mut state.settings.rcon_password).changed() {
                    state.io.send(IORequest::UpdateRconPassword(state.settings.rcon_password.clone()));
                }
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
            ui.checkbox(&mut state.settings.launch_tf2, "Launch TF2").on_hover_text("Launch TF2 when this program is started.");
            ui.checkbox(&mut state.settings.close_on_disconnect, "Close with TF2").on_hover_text("Close this program automatically when it disconnects from TF2.");

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
        if let Err(e) = &state.log_open {
            ui.label(&format!("Could not open log file: {}", e));
            ui.label("Have you set your TF2 directory properly? (It should be the one inside \"common\")\n\n");
            ui.label("Instructions:");
            ui.horizontal(|ui| {
                ui.label("1. Add");
                copy_label("-condebug -conclearlog -usercon", ui);
                ui.label("to your TF2 launch options and start the game.");
            });

            ui.horizontal(|ui| {
                ui.label("2. Click");
                if ui.button("Set your TF2 directory").clicked() {
                    if let Some(pb) = rfd::FileDialog::new().pick_folder() {
                        let dir = match pb.strip_prefix(std::env::current_dir().unwrap()) {
                            Ok(pb) => {
                                pb.to_string_lossy().to_string()
                            },
                            Err(_) => {
                                pb.to_string_lossy().to_string()
                            }
                        };
                        state.settings.tf2_directory = dir;
                        state.io.send(IORequest::UpdateDirectory(state.settings.tf2_directory.clone()));
                    }
                }
                ui.label("and navigate to your Team Fortress 2 folder");
            });
            ui.label("3. Start the program and enjoy the game!\n\n");
            ui.label("Note: If you have set your TF2 directory but are still seeing this message, ensure you have added the launch options and launched the game before trying again.");
            ui.add_space(15.0);
            ui.label("If you have set your TF2 directory and the appropriate launch settings, try launching the game and reopening this application.");
        } else {
            match state.is_connected() {
                // Connected and good
                Ok(false) => {
                    ui.label("Connecting...");
                }
                Ok(true) => {
                    if state.server.get_players().is_empty() {
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
pub fn format_time(time: u32) -> String {
    format!("{:2}:{:02}", time / 60, time % 60)
}

pub const TRUNC_LEN: usize = 40;

/// Truncates a &str
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

// Ui for a player
fn render_players(ui: &mut Ui, state: &mut State, windows: &mut PersistentWindowManager<State>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut remaining_players = Vec::new();
        let mut action: Option<(UserAction, &Player)> = None;
        let width = (ui.available_width() - 5.0) / 2.0;

        ui.columns(2, |cols| {
            // Headings
            cols[0].horizontal(|ui| {
                ui.set_width(width);
                ui.colored_label(Color32::WHITE, "Player Name");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("   ");
                        ui.colored_label(Color32::WHITE, "Time");
                        ui.colored_label(Color32::WHITE, "Info");
                    });
                });
            });

            // Render players
            let mut playerlist: Vec<&Player> = state.server.get_players().values().collect();
            playerlist.sort_by(|a, b| b.time.cmp(&a.time));

            for player in playerlist {
                let team_ui = match player.team {
                    Team::Invaders => &mut cols[0],
                    Team::Defenders => &mut cols[1],
                    Team::None => {
                        remaining_players.push(player);
                        continue;
                    }
                };

                team_ui.horizontal(|ui| {
                    ui.set_width(width);

                    if let Some(returned_action) = player.render_player(
                        ui,
                        &state.settings.user,
                        true,
                        !state.settings.steamapi_key.is_empty(),
                    ) {
                        action = Some((returned_action, player));
                    }
                });
            }
        });

        // Render players with no team
        if !remaining_players.is_empty() {
            ui.separator();
            for player in remaining_players {
                ui.horizontal(|ui| {
                    if let Some(returned_action) = player.render_player(
                        ui,
                        &state.settings.user,
                        true,
                        !state.settings.steamapi_key.is_empty(),
                    ) {
                        action = Some((returned_action, player));
                    }
                });
            }
        }

        // Do whatever action the user requested from the UI
        if let Some((action, player)) = action {
            match action {
                UserAction::Update(record) => {
                    state.server.update_player_from_record(record.clone());
                    state.player_checker.update_player_record(record);
                }
                UserAction::Kick(reason) => {
                    state
                        .io
                        .send(IORequest::RunCommand(CommandManager::kick_player_command(
                            &player.userid,
                            reason,
                        )));
                }
                UserAction::GetProfile(steamid32) => {
                    state.steamapi_request_sender.send(steamid32).ok();
                }
                UserAction::OpenWindow(window) => {
                    windows.push(window);
                }
            }
        }
    });
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
