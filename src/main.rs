extern crate serde;
extern crate chrono;
extern crate rfd;
extern crate clipboard;
extern crate inputbot;


mod app;
use app::*;

fn main() {
    use eframe::egui::Vec2;

    let app = TF2BotKicker::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2::new(800.0, 350.0));
    eframe::run_native(Box::new(app), native_options);
}