use egui_glium::EguiGlium;
use glium::Display;

use crate::{io::{keyboard::Keyboard, mouse::Mouse}};

pub struct Context {
    pub dis: Display,
    pub gui: EguiGlium,

    pub mouse: Mouse,
    pub keyboard: Keyboard,

}

impl Context {
    pub fn new(dis: Display, gui: EguiGlium) -> Context {

        Context {
            dis,
            gui,

            mouse: Mouse::new(),
            keyboard: Keyboard::new(),
        }
    }
}