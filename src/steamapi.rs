use std::thread;

use crossbeam_channel::{Sender, Receiver, unbounded};
use egui::Id;
use egui_extras::RetainedImage;
use glium_app::utils::persistent_window::PersistentWindow;
use steam_api::structs::{summaries, friends, bans};

use crate::state::State;

pub type AccountInfoReceiver = Receiver<(summaries::User, bans::User, Vec<friends::User>, Option<RetainedImage>)>;
pub type AccountInfoSender = Sender<(summaries::User, bans::User, Vec<friends::User>, Option<RetainedImage>)>;

pub fn create_api_thread(key: String) -> (Sender<String>, AccountInfoReceiver) {

    let (request_s, request_r): (Sender<String>, Receiver<String>) = unbounded();
    let (response_s, response_r): (AccountInfoSender, AccountInfoReceiver) = unbounded();

    // Spawn thread to watch requests
    thread::spawn(move || {
        let key = key;

        thread::scope(|s| {
            loop {
                match request_r.recv() {
                    Err(_) => {
                        log::warn!("Disconnected from main thread, killing api thread.");
                        break;
                    },
                    Ok(steamid) => {

                        // On receiving a request, dispatch it on a new thread.
                        s.spawn(|| {
                            let steamid = steamid;
                            
                            let mut summary = match steam_api::get_player_summaries(&steamid, &key) {
                                Ok(summary) => summary,
                                Err(e) => {
                                    log::error!("Failed to get account summary: {}", e);
                                    Vec::new()
                                }
                            };
                            if summary.is_empty() {
                                return;
                            }

                            let mut bans = match steam_api::get_player_bans(&steamid, &key) {
                                Ok(summary) => summary,
                                Err(e) => {
                                    log::error!("Failed to get account bans: {}", e);
                                    Vec::new()
                                }
                            };
                            if bans.is_empty() {
                                return;
                            }

                            let friends = if summary[0].communityvisibilitystate != 3 {
                                match steam_api::get_friends_list(&steamid, &key) {
                                    Ok(friends) => friends,
                                    Err(e) => {
                                        log::warn!("Failed to get friends list: {}", e);
                                        Vec::new()
                                    }
                                }
                            } else {
                                Vec::new()
                            };

                            let img = if let Ok(img_response) = reqwest::blocking::get(&summary[0].avatarmedium) {
                                if let Ok(img) = RetainedImage::from_image_bytes(&summary[0].steamid, &img_response.bytes().unwrap_or_default()) {
                                    Some(img)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            response_s.send((summary.remove(0), bans.remove(0), friends, img)).unwrap();
                        });
                    },
                }
            }
        });
    });

    (request_s, response_r)
}

pub fn create_set_api_key_window(mut key: String) -> PersistentWindow<State> {
    PersistentWindow::new(Box::new(move |id, _, gui_ctx, state| {
        let mut open = true;
        let mut saved = false;

        egui::Window::new("Steam Web API key")
            .id(Id::new(id))
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(gui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Get your own Steam Web API key");
                ui.hyperlink_to("here", "https://steamcommunity.com/dev/apikey");
            });

            ui.text_edit_singleline(&mut key);
    
            if ui.button("Apply").clicked() {
                saved = true;

                state.settings.steamapi_key = key.clone();
                (state.steamapi_request_sender, state.steamapi_request_receiver) = create_api_thread(key.clone());

                for p in state.server.players.values() {
                    state.steamapi_request_sender.send(p.steamid64.clone()).ok();
                }
            }
        });

        open && !saved
    }))
}
