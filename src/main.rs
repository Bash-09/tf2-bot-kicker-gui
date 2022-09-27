#![feature(hash_drain_filter)]

extern crate chrono;
extern crate env_logger;
extern crate rfd;
extern crate serde;

pub mod command_manager;
pub mod gui;
pub mod logwatcher;
pub mod player_checker;
pub mod ringbuffer;
pub mod server;
pub mod settings;
pub mod state;
pub mod timer;
pub mod version;

use chrono::{DateTime, Local};
use command_manager::CommandManager;
use egui::{Align2, Vec2, Color32, Style, Visuals};
use egui_winit::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::{Icon, WindowBuilder},
};
use glium::{
    glutin::{self, ContextBuilder},
    Display,
};
use glium_app::{
    context::Context, run_with_context, utils::persistent_window::{PersistentWindowManager, PersistentWindow},
    Application,
};
use image::{EncodableLayout, ImageFormat};
use player_checker::{PLAYER_LIST, REGEX_LIST};
use server::{player::PlayerType, *};
use state::State;
use version::VersionResponse;
use std::{io::Cursor, time::SystemTime, sync::mpsc::TryRecvError};
mod regexes;

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

    let event_loop = glutin::event_loop::EventLoop::new();
    let cb = ContextBuilder::new().with_vsync(true);
    let display = Display::new(wb, cb, &event_loop).expect("Failed to open Display!");
    let egui_glium = egui_glium::EguiGlium::new(&display);
    let context: Context = Context::new(display, egui_glium);

    run_with_context(app, context, event_loop);
}

pub struct TF2BotKicker {
    state: State,
    cmd: CommandManager,

    windows: PersistentWindowManager<State>,
}

impl TF2BotKicker {
    // Create the application
    pub fn new() -> TF2BotKicker {
        let state = State::new();

        let cmd = CommandManager::new(&state.settings.rcon_password);

        Self {
            state,
            cmd,
            windows: PersistentWindowManager::new(),
        }
    }
}

impl Application for TF2BotKicker {
    fn init(&mut self, _ctx: &mut glium_app::context::Context) {
        self.state.refresh_timer.reset();
        self.state.kick_timer.reset();
        self.state.alert_timer.reset();

        self.state.latest_version = Some(VersionResponse::request_latest_version());
    }

    fn update(&mut self, _t: &glium_app::Timer, ctx: &mut Context) {
        let TF2BotKicker {
            state,
            cmd: _,
            windows,
        } = self;

        if let Some(latest) = &mut state.latest_version {
            match latest.try_recv() {
                Ok(Ok(latest)) => {
                    log::debug!("Got latest version of application, current: {}, latest: {}", version::VERSION, latest.version);

                    if latest.version != version::VERSION && (latest.version != state.settings.ignore_version || state.force_latest_version) {
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
                                .show(ctx, |ui|{
                                    ui.label("You already have the latest version.");
                                });
                            open
                        })));
                    }

                    state.latest_version = None;
                },
                Ok(Err(e)) => { 
                    log::error!("Error getting latest version: {:?}", e);
                    state.latest_version = None;
                },
                Err(TryRecvError::Disconnected) =>  {
                    log::error!("Error getting latest version, other thread did not respond");
                    state.latest_version = None;
                },
                Err(TryRecvError::Empty) => {},
            }
        }

        let refresh = state.refresh_timer.go(state.settings.refresh_period);

        if refresh.is_none() {
            return;
        }

        state.kick_timer.go(state.settings.kick_period);
        state.alert_timer.go(state.settings.alert_period);

        // Refresh server
        if state.refresh_timer.update() {
            state.refresh(&mut self.cmd);

            let system_time = SystemTime::now();
            let datetime: DateTime<Local> = system_time.into();
            log::debug!("{}", format!("Refreshed ({})", datetime.format("%T")));
        }

        // Parse output from `status` and other console output
        match &mut state.log {
            Some(lw) => {
                // If there is a loaded dir, process any new console lines
                while let Some(line) = lw.next_line() {
                    if let Some(c) = state.regx_status.r.captures(&line) {
                        (state.regx_status.f)(
                            &mut state.server,
                            &line,
                            c,
                            &state.settings,
                            &mut state.player_checker,
                            &mut self.cmd,
                        );
                        continue;
                    }
                }
            }
            None => {}
        }

        // Kick Bots and Cheaters
        if !state.settings.paused {
            if state.kick_timer.update() {
                if state.settings.kick_bots {
                    state
                        .server
                        .kick_players_of_type(&state.settings, &mut self.cmd, PlayerType::Bot);
                }

                if state.settings.kick_cheaters {
                    state
                        .server
                        .kick_players_of_type(&state.settings, &mut self.cmd, PlayerType::Cheater);
                }
            }

            if state.alert_timer.update() {
                state
                    .server
                    .send_chat_messages(&state.settings, &mut self.cmd);
            }
        }

        let mut target = ctx.dis.draw();

        let _ = ctx.gui.run(&ctx.dis, |gui_ctx| {
            gui::render(gui_ctx, windows, state, &mut self.cmd);
            windows.render(state, gui_ctx);
        });

        ctx.gui.paint(&ctx.dis, &mut target);
        target.finish().unwrap();
    }

    fn close(&mut self, ctx: &Context) {
        if let Err(e) = self.state.player_checker.save_players(PLAYER_LIST) {
            log::error!("Failed to save players: {:?}", e);
        }
        if let Err(e) = self.state.player_checker.save_regex(REGEX_LIST) {
            log::error!("Failed to save regexes: {:?}", e);
        }

        let gl_window = ctx.dis.gl_window();
        let window = gl_window.window();

        let size = window.inner_size();
        let position = window.outer_position();

        let settings = &mut self.state.settings;
        settings.window.width = size.width;
        settings.window.height = size.height;
        if let Ok(pos) = position {
            settings.window.x = pos.x;
            settings.window.y = pos.y;
        }

        if let Err(e) = settings.export() {
            log::error!("Failed to save settings: {:?}", e);
        }
    }

    fn handle_event(&mut self, _: &mut Context, _: &egui_winit::winit::event::Event<()>) {}
}
