use std::error::Error;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{Color32, Id, Label, RichText, SelectableLabel, Vec2};
use glium_app::utils::persistent_window::PersistentWindow;

use crate::{
    player_checker::PlayerRecord,
    server::player::{PlayerState, PlayerType},
    state::State,
};

use super::{format_time, player_type_combobox, truncate, TRUNC_LEN};

pub fn view_players_window() -> PersistentWindow<State> {
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
                    egui::ScrollArea::vertical().show_rows(ui, ui.text_style_height(&egui::TextStyle::Body), len, |ui, range| {
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
                                        RichText::new(&format!("{:?}", p.player_type))
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
                    });
                });
            });

        if let Some(Action::Delete(steamid)) = action {
            state.player_checker.players.remove(&steamid);
            state.server.players.remove(&steamid);
        } else if let Some(Action::Edit(steamid)) = action {
            windows.push(edit_player_window(
                state.player_checker.players.get(&steamid).unwrap().clone(),
            ));
        }

        open
    }))
}

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
                    if let Some(p) = state.server.players.get_mut(&record.steamid) {
                        p.player_type = record.player_type;
                        p.notes = record.notes.clone();
                    }

                    // Update previous players record
                    for p in state.server.previous_players.inner_mut() {
                        if p.steamid != record.steamid {
                            continue;
                        }

                        p.player_type = record.player_type;
                        p.notes = record.notes.clone();
                    }
                }
            });

        open & !saved
    }))
}

pub fn recent_players_window() -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, windows, gui_ctx, state| {
        let mut open = true;

        egui::Window::new("Recent Players")
            .id(Id::new(id))
            .open(&mut open)
            .collapsible(true)
            .show(gui_ctx, |ui| {
                egui::ScrollArea::vertical().show_rows(ui, ui.text_style_height(&egui::TextStyle::Body), state.server.previous_players.inner().len(), |ui, range| {
                    let width = 500.0;
                    ui.set_width(width);

                    // Render players
                    for i in range {
                        let player = &state.server.previous_players.inner()[state.server.previous_players.inner().len() - i - 1];

                        ui.horizontal(|ui| {
                            ui.set_width(width);

                            let text;
                            if player.steamid == state.settings.user {
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

                            let header = ui.selectable_label(false, text);

                            if header.clicked() {
                                windows.push(edit_player_window(player.get_record()));
                            }

                            // Notes / Stolen name warning
                            if player.stolen_name || !player.notes.is_empty() {
                                header.on_hover_ui(|ui| {
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
                                ui.horizontal(|ui| {
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
                                        ui.add(Label::new(
                                            RichText::new("Joining").color(Color32::YELLOW),
                                        ));
                                    }
                                });
                            });
                        });
                    }
                });
            });
        open
    }))
}
