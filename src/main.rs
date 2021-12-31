extern crate serde;
extern crate chrono;
extern crate rfd;
extern crate clipboard;
extern crate inputbot;


mod app;
use app::*;
// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::egui::Vec2;

    let app = TemplateApp::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2::new(800.0, 300.0));
    eframe::run_native(Box::new(app), native_options);
}