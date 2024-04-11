use std::error::Error;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{ComboBox, Id, Label, RichText, SelectableLabel, Ui, Vec2};
use wgpu_app::utils::persistent_window::PersistentWindow;

use crate::{
    player_checker::PlayerRecord,
    server::player::{Player, PlayerType, UserAction},
    state::State,
};

/// Window that shows all steam accounts currently saved in the playerlist.json file
pub fn saved_players_window() -> PersistentWindow<State> {
    enum Action {
        Delete(String),
        Edit(String),
    }

    let mut filter: Option<PlayerType> = None;
    let mut search = String::new();

    PersistentWindow::new(Box::new(move |id, windows, ctx, state| {
        let mut open = true;
        let mut action: Option<Action> = None;

        egui::Window::new("Saved Players")
            .id(Id::new(id))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("Add Player").clicked() {
                        windows.push(edit_player_window(PlayerRecord {
                            steamid: String::new(),
                            player_type: PlayerType::Player,
                            notes: String::new(),
                        }));
                    }
                    ui.separator();

                    // Filter
                    ui.horizontal(|ui| {
                        let text = match filter {
                            Some(filter) => {
                                RichText::new(format!("{:?}", filter)).color(filter.color(ui))
                            }
                            None => RichText::new("None"),
                        };

                        ui.label("Filter");

                        egui::ComboBox::new("Saved Players", "")
                            .selected_text(text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut filter, None, "None");
                                ui.selectable_value(
                                    &mut filter,
                                    Some(PlayerType::Player),
                                    PlayerType::Player.rich_text(),
                                );
                                ui.selectable_value(
                                    &mut filter,
                                    Some(PlayerType::Bot),
                                    PlayerType::Bot.rich_text(),
                                );
                                ui.selectable_value(
                                    &mut filter,
                                    Some(PlayerType::Cheater),
                                    PlayerType::Cheater.rich_text(),
                                );
                                ui.selectable_value(
                                    &mut filter,
                                    Some(PlayerType::Suspicious),
                                    PlayerType::Suspicious.rich_text(),
                                );
                            });

                        // Search
                        ui.add_space(20.0);
                        ui.label("Search");
                        ui.text_edit_singleline(&mut search);
                    });
                    ui.separator();

                    // Actual player area
                    let mut players: Vec<&mut PlayerRecord> =
                        state.player_checker.players.values_mut().collect();
                    players.retain(|p| {
                        if let Some(filter) = filter {
                            if p.player_type != filter {
                                return false;
                            }
                        }
                        if !p.steamid.contains(&search) && !p.notes.contains(&search) {
                            return false;
                        }
                        true
                    });

                    let len = players.len();
                    egui::ScrollArea::vertical().show_rows(
                        ui,
                        ui.text_style_height(&egui::TextStyle::Body),
                        len,
                        |ui, range| {
                            for i in range {
                                let p: &mut PlayerRecord = players[len - i - 1];

                                ui.horizontal(|ui| {
                                    if ui.button("Delete").clicked() {
                                        action = Some(Action::Delete(p.steamid.clone()));
                                    }
                                    if ui.button("Edit").clicked() {
                                        action = Some(Action::Edit(p.steamid.clone()));
                                    }

                                    ui.add_sized(
                                        Vec2::new(50.0, 20.0),
                                        Label::new(
                                            RichText::new(format!("{:?}", p.player_type))
                                                .color(p.player_type.color(ui)),
                                        ),
                                    );

                                    let steamid_response = ui.add_sized(
                                        Vec2::new(100.0, 20.0),
                                        SelectableLabel::new(false, &p.steamid),
                                    );
                                    if steamid_response.clicked() {
                                        let ctx: Result<ClipboardContext, Box<dyn Error>> =
                                            ClipboardContext::new();
                                        if let Ok(mut ctx) = ctx {
                                            ctx.set_contents(p.steamid.clone()).ok();
                                        }
                                    }
                                    steamid_response.on_hover_text("Click to copy");
                                    ui.label(&p.notes);
                                    ui.add_space(ui.available_width());
                                });
                            }
                        },
                    );
                });
            });

        if let Some(Action::Delete(steamid)) = action {
            state.player_checker.players.remove(&steamid);
            state.server.remove_player(&steamid);
        } else if let Some(Action::Edit(steamid)) = action {
            windows.push(edit_player_window(
                state.player_checker.players.get(&steamid).unwrap().clone(),
            ));
        }

        open
    }))
}

/// Edit a player record
pub fn edit_player_window(mut record: PlayerRecord) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _, gui_ctx, state| {
        let mut open = true;
        let mut saved = false;

        egui::Window::new(format!("Editing Player {}", record.steamid))
            .id(Id::new(id))
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .show(gui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("SteamID3")
                        .on_hover_text("SteamID3 has the format U:1:xxxxxxx");
                    ui.text_edit_singleline(&mut record.steamid);
                });

                ui.horizontal(|ui| {
                    ui.label("Player Type");
                    player_type_combobox("Editing Player", &mut record.player_type, ui);
                });

                ui.text_edit_multiline(&mut record.notes);
                if ui.button("Save").clicked() {
                    saved = true;
                    state.player_checker.update_player_record(record.clone());

                    // Update current server record
                    state.server.update_player_from_record(record.clone());
                }

                // Render player info if they are in the server
                if let Some(player) = state.server.get_players().get(&record.steamid) {
                    player.render_account_info(ui, None);
                }
            });

        open & !saved
    }))
}

/// Show a list of players that were recently connected to the server
pub fn recent_players_window() -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, windows, gui_ctx, state| {
        let mut open = true;

        egui::Window::new("Recent Players")
            .id(Id::new(id))
            .open(&mut open)
            .collapsible(true)
            .show(gui_ctx, |ui| {
                egui::ScrollArea::vertical().show_rows(
                    ui,
                    ui.text_style_height(&egui::TextStyle::Body),
                    state.server.get_previous_players().inner().len(),
                    |ui, range| {
                        let width = 500.0;
                        ui.set_width(width);

                        // Render players
                        let mut action: Option<(UserAction, &Player)> = None;
                        for i in range {
                            let player = &state.server.get_previous_players().inner()
                                [state.server.get_previous_players().inner().len() - i - 1];

                            ui.horizontal(|ui| {
                                ui.set_width(width);

                                if let Some(returned_action) = player.render_player(
                                    ui,
                                    &state.settings.user,
                                    false,
                                    !state.settings.steamapi_key.is_empty(),
                                    None
                                ) {
                                    action = Some((returned_action, player));
                                }
                            });
                        }

                        // Do whatever action the user requested from the UI
                        if let Some((action, _)) = action {
                            match action {
                                UserAction::Update(record) => {
                                    state.server.update_player_from_record(record.clone());
                                    state.player_checker.update_player_record(record);
                                }
                                UserAction::Kick(_) => {
                                    log::error!(
                                        "Was able to kick from the recent players window??"
                                    );
                                }
                                UserAction::GetProfile(steamid32) => {
                                    state.steamapi_request_sender.send(steamid32).ok();
                                }
                                UserAction::OpenWindow(window) => {
                                    windows.push(window);
                                }
                            }
                        }
                    },
                );
            });
        open
    }))
}

/// Creates a dropdown combobox to select a player type, returns true if the value was changed
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

/// Creates a dialog window where the user can edit and save the notes for a specific account
pub fn create_edit_notes_window(mut record: PlayerRecord) -> PersistentWindow<State> {
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
                            state.server.update_player_from_record(record.clone());
                            state.player_checker.update_player_record(record.clone());
                        }
                    });
                });
            });
        open & !saved
    }))
}
