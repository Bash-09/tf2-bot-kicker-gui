extern crate chrono;
extern crate clipboard;
extern crate inputbot;
extern crate rfd;
extern crate serde;

mod app;
use app::*;
use tokio::runtime::Runtime;

// #[tokio::main]
fn main() {
    let rt = Runtime::new().unwrap();
    // let app = Box::new(TF2BotKicker::new().await);

    let mut app = None;

    rt.block_on(async {
        app = Some(TF2BotKicker::new().await);
    });

    glium_app::run(app.unwrap(), rt);
}
