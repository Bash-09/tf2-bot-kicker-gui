extern crate serde;
extern crate chrono;
extern crate rfd;
extern crate clipboard;
extern crate inputbot;


mod app;
use app::*;

#[tokio::main]
async fn main() {
    let app = Box::new(TF2BotKicker::new().await);
    
    glium_app::run(app).await;
}