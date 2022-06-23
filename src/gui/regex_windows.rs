use egui::Id;
use glium_app::utils::persistent_window::PersistentWindow;
use regex::Regex;

use crate::state::State;

use super::create_dialog_box;

pub fn new_regex_window(mut regex: String) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, windows, gui_ctx, state| {
        let mut open = true;

        let mut exported = false;
        egui::Window::new("New Regex")
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
                                state.player_checker.bots_regx.push(reg);
                                log::info!("{}", state.message);
                                exported = true;
                            }
                            Err(e) => {
                                windows.push(create_dialog_box("Invalid Regex".to_string(), format!("{}", e)));
                            }
                        }
                    }
                });
            });

        open & !exported
    }))
}

pub fn view_regexes_window() -> PersistentWindow<State> {
    enum Action {
        Delete(usize),
        Edit(usize),
    }

    PersistentWindow::new(Box::new(move |id, windows, gui_ctx, state| {
        let mut open = true;

        // Saved Regexes window
        egui::Window::new("Saved Regexes")
        .id(Id::new(id))
        .collapsible(false)
        .open(&mut open)
        .show(gui_ctx, |ui| {

            let mut action: Option<Action> = None;
            ui.vertical_centered(|ui| {
                // Add new regex button
                if ui.button("Add Regex").clicked() {
                    windows.push(new_regex_window(String::new()));
                }
                ui.separator();

                // List of regexes
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, regex) in state.player_checker.bots_regx.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(regex.as_str());

                            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                if ui.button("Delete").clicked() {
                                    action = Some(Action::Delete(i));
                                }
                                if ui.button("Edit").clicked() {
                                    action = Some(Action::Edit(i));
                                }
                            });
                        });
                    }
                });

                // Delete or edit regex
                if let Some(Action::Delete(i)) = action {
                    state.player_checker.bots_regx.remove(i);
                }
                if let Some(Action::Edit(i)) = action {
                    windows.push(edit_regex_window(state.player_checker.bots_regx[i].to_string(), i, state.player_checker.bots_regx.len()));
                }
            });
        });

        open
    }))
}

pub fn edit_regex_window(mut regex: String, i: usize, len: usize) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, windows, gui_ctx, state| {
        let mut open = true;
        let mut saved = false;

        egui::Window::new("Edit regex")
        .id(Id::new(id))
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .show(gui_ctx, |ui| {
            if i >= len || len != state.player_checker.bots_regx.len() {
                saved = true;
                return;
            }

            ui.vertical_centered(|ui| {
                ui.text_edit_singleline(&mut regex);
                // Attempt to Save regex
                if ui.button("Save").clicked() {
                    match Regex::new(&regex) {
                        Ok(mut r) => {
                            saved = true;
                            std::mem::swap(&mut r, &mut state.player_checker.bots_regx[i]);
                        },
                        Err(e) => {
                            windows.push(create_dialog_box("Invalid Regex".to_string(), format!("{}", e)));
                        }
                    }
                }
            });
        });

        open & !saved
    }))
}