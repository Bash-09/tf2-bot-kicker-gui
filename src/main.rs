extern crate chrono;
extern crate clipboard;
extern crate env_logger;
extern crate rfd;
extern crate serde;

use std::{fs::OpenOptions, io::Write, time::SystemTime};

use chrono::{DateTime, Local};

pub mod timer;
use egui_winit::winit::{dpi::PhysicalSize, window::WindowBuilder};
use glium_app::{
    context::Context, run, utils::persistent_window::PersistentWindowManager, Application,
};
use state::State;

pub mod settings;

pub mod server;
use server::*;

pub mod gui;

use tokio::runtime::Runtime;

mod regexes;

pub mod bot_checker;

pub mod logwatcher;
pub mod state;

fn main() {
    env_logger::init();

    let wb = WindowBuilder::new()
        .with_title("TF2 Bot Kicker by Bash09/Googe14")
        .with_resizable(true)
        .with_inner_size(PhysicalSize::new(800, 400));
    let app = TF2BotKicker::new();
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
                log::info!("{}", state.message);
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
                                &mut state.bot_checker,
                            );
                            continue;
                        }

                        if let Some(c) = state.regx_status.r.captures(&line) {
                            (state.regx_status.f)(
                                &mut state.server,
                                &line,
                                c,
                                &state.settings,
                                &mut state.bot_checker,
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
            if state.alert_timer.update() {
                if state.rcon_connected().await {
                    state
                        .server
                        .announce_bots(&state.settings, state.rcon.as_mut().unwrap())
                        .await;
                }
            }
        });

        let mut target = ctx.dis.draw();

        let _ = ctx.gui.run(&ctx.dis, |gui_ctx| {
            gui::render(gui_ctx, windows, state);
            windows.render(state, gui_ctx);
        });

        ctx.gui.paint(&mut ctx.dis, &mut target);
        target.finish().unwrap();
    }

    fn close(&mut self) {}

    fn handle_event(&mut self, _: &mut Context, _: &egui_winit::winit::event::Event<()>) {}
}

pub fn append_line(data: &str, target: &str) {
    // Add suspected bot steamid and name to file
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
