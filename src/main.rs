extern crate chrono;
extern crate clipboard;
extern crate env_logger;
extern crate rfd;
extern crate serde;

pub mod command_manager;
pub mod gui;
pub mod logwatcher;
pub mod player_checker;
pub mod server;
pub mod settings;
pub mod state;
pub mod timer;

use chrono::{DateTime, Local};
use egui_winit::winit::{dpi::{PhysicalSize, PhysicalPosition}, window::WindowBuilder};
use glium_app::{
    context::Context, run, utils::persistent_window::PersistentWindowManager, Application,
};
use server::*;
use state::State;
use std::{fs::OpenOptions, io::Write, time::SystemTime};
use tokio::runtime::Runtime;
mod regexes;

fn main() {
    env_logger::init();

    let app = TF2BotKicker::new();

    let inner_size = PhysicalSize::new(app.state.settings.window.width, app.state.settings.window.height);
    let outer_pos = PhysicalPosition::new(app.state.settings.window.x, app.state.settings.window.y);

    let wb = WindowBuilder::new()
        .with_title("TF2 Bot Kicker by Bash09")
        .with_resizable(true)
        .with_inner_size(inner_size)
        .with_position(outer_pos);


    run(app, wb);
}

pub struct TF2BotKicker {
    state: State,

    runtime: Runtime,
    windows: PersistentWindowManager<State>,
}

impl TF2BotKicker {
    // Create the application
    pub fn new() -> TF2BotKicker {
        let runtime = Runtime::new().expect("Failed to create async runtime");
        let state = State::new(&runtime);

        Self {
            state,

            runtime,
            windows: PersistentWindowManager::new(),
        }
    }
}

impl Application for TF2BotKicker {
    fn init(&mut self, _ctx: &mut glium_app::context::Context) {
        self.state.refresh_timer.reset();
        self.state.kick_timer.reset();
        self.state.alert_timer.reset();
    }

    fn update(&mut self, _t: &glium_app::Timer, ctx: &mut Context) {
        let TF2BotKicker {
            state,

            runtime,
            windows,
        } = self;

        let refresh = state.refresh_timer.go(state.settings.refresh_period);

        if refresh.is_none() {
            return;
        }

        state.kick_timer.go(state.settings.kick_period);
        state.alert_timer.go(state.settings.alert_period);

        runtime.block_on(async {
            // Refresh server
            if state.refresh_timer.update() {
                state.refresh().await;

                let system_time = SystemTime::now();
                let datetime: DateTime<Local> = system_time.into();
                state.message = format!("Refreshed ({})", datetime.format("%T"));
                log::debug!("{}", state.message);
            }
        });

        match &mut state.log {
            Some(lw) => {
                // If there is a loaded dir, process any new console lines
                loop {
                    if let Some(line) = lw.next_line() {
                        if let Some(c) = state.regx_disconnect.r.captures(&line) {
                            (state.regx_disconnect.f)(
                                &mut state.server,
                                &line,
                                c,
                                &state.settings,
                                &mut state.player_checker,
                            );
                            continue;
                        }

                        if let Some(c) = state.regx_status.r.captures(&line) {
                            (state.regx_status.f)(
                                &mut state.server,
                                &line,
                                c,
                                &state.settings,
                                &mut state.player_checker,
                            );
                            continue;
                        }
                    } else {
                        break;
                    }
                }
            }
            None => {}
        }

        runtime.block_on(async {
            // Kick Bots
            if state.kick_timer.update() {
                if state.rcon_connected().await {
                    state
                        .server
                        .kick_bots(&state.settings, state.rcon.as_mut().unwrap())
                        .await;
                }
            }

            // Send chat alerts
            // if state.alert_timer.update() {
            //     if state.rcon_connected().await {
            //         state
            //             .server
            //             .announce_bots(&state.settings, state.rcon.as_mut().unwrap())
            //             .await;
            //     }
            // }
        });

        let mut target = ctx.dis.draw();

        let _ = ctx.gui.run(&ctx.dis, |gui_ctx| {
            gui::render(gui_ctx, windows, state);
            windows.render(state, gui_ctx);
        });

        ctx.gui.paint(&mut ctx.dis, &mut target);
        target.finish().unwrap();
    }

    fn close(&mut self, ctx: &Context) {
        if let Err(e) = self.state.player_checker.save_players("cfg/players.json") {
            log::error!("Failed to save players: {:?}", e);
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

pub fn append_line(data: &str, target: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(target)
        .expect(&format!("Failed to Open or Write to {}", target));

    if let Err(_) = write!(file, "\n{}", data) {
        log::error!("Failed to open or write to {}", target);
    }
}
