use std::error::Error;

use clipboard::{ClipboardContext, ClipboardProvider};
use egui::{Id, RichText, Vec2, SelectableLabel};
use egui_winit::clipboard::Clipboard;
use glium_app::utils::persistent_window::PersistentWindow;

use crate::{state::State, server::player::PlayerType, player_checker::PlayerRecord, gui::copy_label};

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
                    windows.push(edit_player_window(PlayerRecord { steamid: String::new(), player_type: PlayerType::Player, notes: String::new() }));
                }
                ui.separator();

                // Filter
                ui.horizontal(|ui| {
                    let text = match filter {
                        Some(filter) => {
                            RichText::new(format!("{:?}", filter)).color(filter.color(ui))
                        },
                        None => RichText::new("None")
                    };

                    ui.label("Filter");
                    egui::ComboBox::new("Saved Players", "")
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut filter, None, "None");
                        ui.selectable_value(&mut filter, Some(PlayerType::Player), RichText::new("Player").color(PlayerType::Player.color(ui)));
                        ui.selectable_value(&mut filter, Some(PlayerType::Bot), RichText::new("Bot").color(PlayerType::Bot.color(ui)));
                        ui.selectable_value(&mut filter, Some(PlayerType::Cheater), RichText::new("Cheater").color(PlayerType::Cheater.color(ui)));
                    });

                    // Search
                    ui.add_space(20.0);
                    ui.label("Search");
                    ui.text_edit_singleline(&mut search);
                });
                ui.separator();

                // Actual player area
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for p in state.player_checker.players.values_mut() {
                        if let Some(filter) = filter {
                            if p.player_type != filter {
                                continue;
                            }
                        }

                        if !p.steamid.contains(&search) && !p.notes.contains(&search) {
                            continue;
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                action = Some(Action::Delete(p.steamid.clone()));
                            }
                            if ui.button("Edit").clicked() {
                                action = Some(Action::Edit(p.steamid.clone()));
                            }

                            egui::ComboBox::new(&p.steamid, "")
                            .selected_text(RichText::new(format!("{:?}", p.player_type)).color(p.player_type.color(ui)))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut p.player_type, PlayerType::Player, RichText::new("Player").color(PlayerType::Player.color(ui)));
                                ui.selectable_value(&mut p.player_type, PlayerType::Bot, RichText::new("Bot").color(PlayerType::Bot.color(ui)));
                                ui.selectable_value(&mut p.player_type, PlayerType::Cheater, RichText::new("Cheater").color(PlayerType::Cheater.color(ui)));
                            });

                            let steamid_response = ui.add_sized(Vec2::new(100.0, 20.0), SelectableLabel::new(false, &p.steamid));
                            if steamid_response.clicked() {
                                let ctx: Result<ClipboardContext, Box<dyn Error>> = ClipboardContext::new();
                                if let Ok(mut ctx) = ctx {
                                    ctx.set_contents(p.steamid.clone()).ok();
                                }
                            }
                            steamid_response.on_hover_text("Click to copy");
                            ui.label(&p.notes);
                        });
                    }
                });
            });
        });

        if let Some(Action::Delete(steamid)) = action {
            state.player_checker.players.remove(&steamid);
            state.server.players.remove(&steamid);
        } else if let Some(Action::Edit(steamid)) = action {
            windows.push(edit_player_window(state.player_checker.players.get(&steamid).unwrap().clone()));
        }

        open
    }))
}

pub fn edit_player_window(mut record: PlayerRecord) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _, gui_ctx, state| {
        let mut open = true;
        let mut saved = false;

        egui::Window::new(format!("Editing {}", record.steamid))
        .id(Id::new(id))
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .show(gui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("SteamID3").on_hover_text("SteamID3 has the format U:1:xxxxxxx");
                ui.text_edit_singleline(&mut record.steamid);
            });
            ui.horizontal(|ui| {
                ui.label("Player Type");
                egui::ComboBox::new("Editing player", "")
                .selected_text(RichText::new(format!("{:?}", record.player_type)).color(record.player_type.color(ui)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut record.player_type, PlayerType::Player, RichText::new("Player").color(PlayerType::Player.color(ui)));
                    ui.selectable_value(&mut record.player_type, PlayerType::Bot, RichText::new("Bot").color(PlayerType::Bot.color(ui)));
                    ui.selectable_value(&mut record.player_type, PlayerType::Cheater, RichText::new("Cheater").color(PlayerType::Cheater.color(ui)));
                });
            });
            ui.text_edit_multiline(&mut record.notes);
            if ui.button("Save").clicked() {
                saved = true;
                state.player_checker.update_player_record(record.clone());
                if let Some(p) = state.server.players.get_mut(&record.steamid) {
                    p.player_type = record.player_type.clone();
                    p.notes = record.notes.clone();
                }
            }
        });

        open & !saved
    }))
}