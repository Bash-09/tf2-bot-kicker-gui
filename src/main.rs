#![feature(hash_drain_filter)]

extern crate chrono;
extern crate env_logger;
extern crate rfd;
extern crate serde;
extern crate steam_api;

pub mod gui;
pub mod io;
pub mod player_checker;
pub mod ringbuffer;
pub mod server;
pub mod settings;
pub mod state;
pub mod steamapi;
pub mod timer;
pub mod version;

use chrono::{DateTime, Local};
use crossbeam_channel::TryRecvError;
use egui::{Align2, Vec2};
use egui_dock::{DockArea, Tree};
use egui_winit::{
    egui,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        window::{Icon, WindowBuilder},
    },
};
use gui::GuiTab;
use image::{EncodableLayout, ImageFormat};

use player_checker::{PLAYER_LIST, REGEX_LIST};
use server::{player::PlayerType, *};
use state::State;
use std::{io::Cursor, time::SystemTime};
use version::VersionResponse;
use wgpu_app::utils::persistent_window::{PersistentWindow, PersistentWindowManager};

fn main() {
    env_logger::init();

    let app = TF2BotKicker::new();

    let inner_size = PhysicalSize::new(
        app.state.settings.window.width,
        app.state.settings.window.height,
    );
    let outer_pos = PhysicalPosition::new(app.state.settings.window.x, app.state.settings.window.y);

    let mut logo = image::io::Reader::new(Cursor::new(include_bytes!("../images/logo.png")));
    logo.set_format(ImageFormat::Png);

    let wb = WindowBuilder::new()
        .with_window_icon(Some(
            Icon::from_rgba(
                logo.decode().unwrap().into_rgba8().as_bytes().to_vec(),
                512,
                512,
            )
            .unwrap(),
        ))
        .with_title("TF2 Bot Kicker by Bash09")
        .with_resizable(true)
        .with_inner_size(inner_size)
        .with_position(outer_pos);
    wgpu_app::run(app, wb);
}

pub struct TF2BotKicker {
    state: State,
    windows: PersistentWindowManager<State>,
    gui_tree: Tree<GuiTab>,
}

impl Default for TF2BotKicker {
    fn default() -> Self {
        Self::new()
    }
}

impl TF2BotKicker {
    // Create the application
    pub fn new() -> TF2BotKicker {
        let state = State::new();
        let gui_tree = state.settings.saved_dock.clone();

        Self {
            state,
            windows: PersistentWindowManager::new(),
            gui_tree,
        }
    }
}

impl wgpu_app::Application for TF2BotKicker {
    fn init(&mut self, _ctx: &mut wgpu_app::context::Context) {
        self.state.refresh_timer.reset();
        self.state.kick_timer.reset();
        self.state.alert_timer.reset();

        self.state.latest_version = Some(VersionResponse::request_latest_version());
        if !self.state.settings.ignore_no_api_key && self.state.settings.steamapi_key.is_empty() {
            self.windows
                .push(steamapi::create_set_api_key_window(String::new()));
        }

        // Try to run TF2 if set to
        if self.state.settings.launch_tf2 {
            if let Err(e) = std::process::Command::new("steam")
                .arg("steam://rungameid/440")
                .spawn()
            {
                self.windows
                    .push(PersistentWindow::new(Box::new(move |id, _, ctx, _| {
                        let mut open = true;
                        egui::Window::new("Failed to launch TF2")
                            .id(egui::Id::new(id))
                            .open(&mut open)
                            .show(ctx, |ui| {
                                ui.label(&format!("{:?}", e));
                            });
                        open
                    })));
            }
        }
    }

    fn update(
        &mut self,
        _t: &wgpu_app::Timer,
        ctx: &mut wgpu_app::context::Context,
    ) -> Result<(), wgpu::SurfaceError> {
        let TF2BotKicker {
            state,
            windows,
            gui_tree,
        } = self;

        // Check latest version
        if let Some(latest) = &mut state.latest_version {
            match latest.try_recv() {
                Ok(Ok(latest)) => {
                    log::debug!(
                        "Got latest version of application, current: {}, latest: {}",
                        version::VERSION,
                        latest.version
                    );

                    if latest.version != version::VERSION
                        && (latest.version != state.settings.ignore_version
                            || state.force_latest_version)
                    {
                        windows.push(latest.to_persistent_window());
                        state.force_latest_version = false;
                    } else if state.force_latest_version {
                        windows.push(PersistentWindow::new(Box::new(|_, _, ctx, _| {
                            let mut open = true;
                            egui::Window::new("No updates available")
                                .collapsible(false)
                                .resizable(false)
                                .open(&mut open)
                                .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                                .show(ctx, |ui| {
                                    ui.label("You already have the latest version.");
                                });
                            open
                        })));
                    }

                    state.latest_version = None;
                }
                Ok(Err(e)) => {
                    log::error!("Error getting latest version: {:?}", e);
                    state.latest_version = None;
                }
                Err(TryRecvError::Disconnected) => {
                    log::error!("Error getting latest version, other thread did not respond");
                    state.latest_version = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        // Handle incoming messages from IO thread
        state.handle_messages();

        // Send steamid requests if an API key is set
        if state.settings.steamapi_key.is_empty() {
            state.server.pending_lookup.clear();
        }
        while let Some(steamid64) = state.server.pending_lookup.pop() {
            state.steamapi_request_sender.send(steamid64).ok();
        }

        // Handle finished steamid requests
        while let Ok((info, img, steamid)) = state.steamapi_request_receiver.try_recv() {
            if let Some(p) = state
                .server
                .get_player_mut(&player::steamid_64_to_32(&steamid).unwrap_or_default())
            {
                p.account_info = info;
                p.profile_image = img;
            }
        }

        let refresh = state.refresh_timer.go(state.settings.refresh_period);

        if refresh.is_none() {
            return Ok(());
        }

        state.kick_timer.go(state.settings.kick_period);
        state.alert_timer.go(state.settings.alert_period);

        // Refresh server
        if state.refresh_timer.update() {
            state.refresh();

            // Close if TF2 has been closed and we want to close now
            if state.has_connected()
                && !state.is_connected().is_ok()
                && state.settings.close_on_disconnect
            {
                log::debug!("Lost connection from TF2, closing program.");
                self.close(ctx);
                std::process::exit(0);
            }

            let system_time = SystemTime::now();
            let datetime: DateTime<Local> = system_time.into();
            log::debug!("{}", format!("Refreshed ({})", datetime.format("%T")));
        }

        // Kick Bots and Cheaters
        if !state.settings.paused {
            if state.kick_timer.update() {
                if state.settings.kick_bots {
                    state.server.kick_players_of_type(
                        &state.settings,
                        &mut state.io,
                        PlayerType::Bot,
                    );
                }

                if state.settings.kick_cheaters {
                    state.server.kick_players_of_type(
                        &state.settings,
                        &mut state.io,
                        PlayerType::Cheater,
                    );
                }
            }

            if state.alert_timer.update() {
                state
                    .server
                    .send_chat_messages(&state.settings, &mut state.io);
            }
        }

        // Render *****************88
        let output = ctx.wgpu_state.surface.get_current_texture()?;
        ctx.egui.render(&mut ctx.wgpu_state, &output, |gui_ctx| {
            gui::render_top_panel(gui_ctx, state, gui_tree);
            DockArea::new(gui_tree).show(gui_ctx, state);

            // Get new persistent windows
            if !state.new_persistent_windows.is_empty() {
                let mut new_windows = Vec::new();
                std::mem::swap(&mut new_windows, &mut state.new_persistent_windows);
                for w in new_windows {
                    windows.push(w);
                }
            }
            windows.render(state, gui_ctx);
        });
        output.present();

        Ok(())
    }

    fn close(&mut self, ctx: &wgpu_app::context::Context) {
        if let Err(e) = self.state.player_checker.save_players(PLAYER_LIST) {
            log::error!("Failed to save players: {:?}", e);
        }
        if let Err(e) = self.state.player_checker.save_regex(REGEX_LIST) {
            log::error!("Failed to save regexes: {:?}", e);
        }

        let size = ctx.wgpu_state.window.inner_size();
        let position = ctx.wgpu_state.window.outer_position();

        let settings = &mut self.state.settings;
        settings.window.width = size.width;
        settings.window.height = size.height;
        if let Ok(pos) = position {
            settings.window.x = pos.x;
            settings.window.y = pos.y;
        }
        settings.saved_dock = self.gui_tree.clone();

        if let Err(e) = settings.export() {
            log::error!("Failed to save settings: {:?}", e);
        }
    }

    fn handle_event(
        &mut self,
        _: &mut wgpu_app::context::Context,
        _: &egui_winit::winit::event::Event<()>,
    ) {
    }
}
