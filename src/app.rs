use std::{time::SystemTime, fs::read_dir};

use chrono::{DateTime, Utc, Local};
use clipboard::{ClipboardProvider, ClipboardContext};
use eframe::{egui::{self, Ui}, epi};

pub mod timer;
use timer::*;

pub mod settings;
use settings::*;

pub mod log_watcher;
use log_watcher::*;

pub struct TemplateApp {

    timer: Timer,
    settings: Settings,

    message: String,

    console: Option<LogWatcher>,

    paused: bool,

}

impl Default for TemplateApp {
    fn default() -> Self {

        let settings: Settings;

        let set = Settings::import("settings.json");
        if set.is_err() {
            settings = Settings::new();
        } else {
            settings = set.unwrap();
        }

        let console = use_directory(&settings.directory);

        Self {
            timer: Timer::new(),
            settings,

            message: String::from("Loaded..."),

            console,

            paused: true,
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "TF2 Bot Kicker by Bash09/Googe14"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {



    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        // Ensures update is called again as soon as this one is finished.
        ctx.request_repaint();

        // Skip the update if it hasn't been very long
        let t = self.timer.go(self.settings.period);
        if t.is_none() {return;}

        if self.timer.update() && !self.paused {
            let system_time = SystemTime::now();
            let datetime: DateTime<Local> = system_time.into();
            self.message = format!("Refreshing... ({})", datetime.format("%T"));


        }

        // Tracks if the settings need to be saved
        let mut settings_changed = false;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {

                    if ui.button("Set TF2 Directory").clicked() {
                        match rfd::FileDialog::new().pick_folder() {
                            Some(pb) => {
                                let dir = pb.to_string_lossy().to_string();
                                self.settings.directory = dir;
                                self.console = use_directory(&self.settings.directory);
                            },
                            None => {}
                        }
                    }

                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });

                if ui.button(match self.paused {true => "Start", false => "Pause"}).clicked() {
                    self.paused = !self.paused;
                    self.message = String::from(match self.paused {true => "Paused", false => "Started"});
                }
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Settings");

            ui.horizontal(|ui| {
                ui.label("User: ");
                settings_changed |= ui.text_edit_singleline(&mut self.settings.user).changed();
            });

            settings_changed |= ui.checkbox(&mut self.settings.kick, "Kick Bots").changed();
            settings_changed |= ui.checkbox(&mut self.settings.join_alert, "Join Alerts").changed();
            settings_changed |= ui.checkbox(&mut self.settings.chat_reminders, "Chat Reminders").changed();

            ui.horizontal(|ui| {
                ui.label("Period: ");
                settings_changed |= ui.add(egui::Slider::new(&mut self.settings.period, 1.0..=60.0)).changed();
            });


            // Credits at the bottom left
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                });

                // Display a little bit of information
                ui.label(&self.message);
            });
        });


        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            match &self.console {
                // Text for if there's no TF2 directory set yet
                None=> {

                    ui.label("No valid TF2 directory set. (It should be the one inside \"common\")\n\n");

                    ui.label("Instructions:");

                    ui.horizontal(|ui| {
                        ui.label("1. Add");
                        copy_label(&mut self.message, "-condebug -conclearlog", ui);
                        ui.label("to your TF2 launch options and start the game.");
                    });

                    ui.horizontal(|ui| {
                        ui.label("2. Add");
                        copy_label(&mut self.message, "bind F8 \"exec command.cfg\"", ui);
                        ui.label("to your autoexec.cfg file.");
                    });

                    ui.horizontal(|ui| {
                        ui.label("3. Click");
                        if ui.button("Set your TF2 directory").clicked() {

                            match rfd::FileDialog::new().pick_folder() {
                                Some(pb) => {
                                    
                                    let dir = pb.to_string_lossy().to_string();
                                    self.settings.directory = dir;
                                    self.console = use_directory(&self.settings.directory);

                                },
                                None => {}
                            }
                        }
                        ui.label("and navigate to your Team Fortress 2 folder");
                    });
                    ui.label("4. Start the program and enjoy the game!\n\n");
                    ui.label("Note: If you have set your TF2 directory but are still seeing this message, ensure you have added the launch options and launched the game before trying again.");

                },

                // When there is a TF2 directory present
                Some(log) => {



                }
            }

        });



            // egui::Window::new("Window").show(ctx, |ui| {
            //     ui.label("Windows can be moved by dragging them.");
            //     ui.label("They are automatically sized based on contents.");
            //     ui.label("You can turn on resizing and scrolling if you like.");
            //     ui.label("You would normally chose either panels OR windows.");
            // });

        if settings_changed {
            match self.settings.export("settings.json") {
                Ok(_) => {},
                Err(e) => {
                    println!("Failed to export settings");
                    self.message = e.to_string();
                }
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
                },
                Err(e) => {
                    log.clear();
                    log.push_str("Couldn't copy text to clipboard");
                }
            }
        }
        lab.on_hover_text("Copy");
}

// Try to open this TF2 directory
fn use_directory(dir: &str) -> Option<LogWatcher> {

    if read_dir(format!("{}/tf/cfg", dir)).is_ok() {
        


    }

    None
}

