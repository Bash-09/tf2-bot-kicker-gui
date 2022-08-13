use std::{sync::mpsc, error::Error};

use egui::{Id, Vec2, Align2};
use glium_app::utils::persistent_window::PersistentWindow;
use serde_json::Value;

use crate::state::State;

pub const VERSION: &str = "v1.2.3";

pub struct VersionResponse {
    pub version: String,
    pub downloads: Vec<String>,
}

impl VersionResponse {

    pub fn request_latest_version() -> std::sync::mpsc::Receiver<Result<VersionResponse, Box<dyn Error + Send>>> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .unwrap();

            runtime.block_on(async {
                tx.send(VersionResponse::get_latest_version().await).unwrap();
            });

        });

        rx
    }

    async fn get_latest_version() -> Result<VersionResponse, Box<dyn Error + Send>> {
        let release = match reqwest::Client::new().get("https://api.github.com/repos/Bash-09/tf2-bot-kicker-gui/releases/latest")
                    .header("User-Agent", "tf2-bot-kicker-gui")
                    .send().await {
            Ok(it) => it,
            Err(err) => return Err(Box::new(err)),
        };

        let text = match release.text().await {
            Ok(it) => it,
            Err(err) => return Err(Box::new(err)),
        };
        let json: Value = match serde_json::from_str(&text) {
            Ok(it) => it,
            Err(err) => return Err(Box::new(err)),
        };

        
        let version;
        if let Some(Value::String(v)) = json.get("tag_name") {
            version = v.to_string();
        } else {
            version = "".to_string();
        }

        let mut response = VersionResponse {
            version,
            downloads: Vec::new(),
        };

        if let Some(Value::Array(assets)) = json.get("assets") {
            for a in assets {
                if let Some(Value::String(url)) = a.get("browser_download_url") {
                    response.downloads.push(url.to_string());
                }
            }
        }

        Ok(response)
    }


    pub fn to_persistent_window(self) -> PersistentWindow<State> {
        let file_names: Vec<String> = self.downloads.iter().map(|link| {
            link.split('/').last().unwrap().to_string()
        }).collect();

        PersistentWindow::new(Box::new(move |id, _, ctx, state| {
            let mut open = true;

            egui::Window::new("New version available")
                .id(Id::new(id))
                .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {

                    ui.heading(&format!("Current version: {}", VERSION));
                    ui.heading(&format!("Latest version:  {}", &self.version));

                    let ignored = state.settings.ignore_version == self.version;
                    let mut ignore = ignored;
                    ui.checkbox(&mut ignore, "Don't remind me for this version");
                    if ignore && !ignored {
                        state.settings.ignore_version = self.version.clone();
                    } else if !ignore && ignored {
                        state.settings.ignore_version = String::new();
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Find the latest version on");
                        ui.add(egui::Hyperlink::from_label_and_url("github", "https://github.com/Bash-09/tf2-bot-kicker-gui/releases/latest"));
                    });
                    ui.add_space(10.0);

                    ui.label("Or download it directly:");
                    for (i, file) in file_names.iter().enumerate() {
                        ui.add(egui::Hyperlink::from_label_and_url(file, &self.downloads[i]));
                    }

                });
            open
        }))
    }

}

