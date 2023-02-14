use egui::{Id, Separator};
use glium_app::utils::persistent_window::PersistentWindow;

use crate::state::State;

pub fn view_chat_window() -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _windows, ctx, state| {
        let mut open = true;

        egui::Window::new("Chat settings")
            .id(Id::new(id))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.heading("Joining");

                ui.horizontal(|ui| {
                    ui.label("Message (bots)")
                        .on_hover_text("when *bots* are joining");
                    ui.text_edit_singleline(&mut state.settings.message_bots);
                });
                ui.horizontal(|ui| {
                    ui.label("Message (cheaters)")
                        .on_hover_text("When *cheaters* are joining");
                    ui.text_edit_singleline(&mut state.settings.message_cheaters);
                });
                ui.horizontal(|ui| {
                    ui.label("Message (both)")
                        .on_hover_text("When *both bots and cheaters* are joining");
                    ui.text_edit_singleline(&mut state.settings.message_both);
                });

                ui.add(Separator::default().spacing(20.0));
                ui.heading("Team");

                ui.horizontal(|ui| {
                    ui.label("Message (same team)")
                        .on_hover_text("When a bot/cheater joins *your* team");
                    ui.text_edit_singleline(&mut state.settings.message_same_team);
                });
                ui.horizontal(|ui| {
                    ui.label("Message (enemy team)").on_hover_text(
                        "When a bot/cheater joins the *enemy* team",
                    );
                    ui.text_edit_singleline(&mut state.settings.message_enemy_team);
                });
                ui.horizontal(|ui| {
                    ui.label("Message (both teams)")
                        .on_hover_text("When bots/cheaters join *both* teams");
                    ui.text_edit_singleline(&mut state.settings.message_both_teams);
                });
                ui.horizontal(|ui| {
                    ui.label("Message (default)")
                        .on_hover_text("When a bot/cheater joins your game (for when your UserID is not provided or the bot does not have a team)");
                    ui.text_edit_singleline(&mut state.settings.message_default);
                });

                ui.add(Separator::default().spacing(20.0));
                ui.heading("Example message:");
                ui.label(format!("{} {} m4gic", state.settings.message_bots.trim(), state.settings.message_same_team.trim()));
            });

        open
    }))
}
