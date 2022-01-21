use glium::glutin::event::VirtualKeyCode;

use std::collections::HashMap;

pub struct Keyboard {
    keys: HashMap<VirtualKeyCode, bool>,
    this_frame: HashMap<VirtualKeyCode, bool>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            keys: HashMap::new(),
            this_frame: HashMap::new(),
        }
    }

    pub fn press(&mut self, key: VirtualKeyCode) {
        self.keys.insert(key, true);
        self.this_frame.insert(key, true);
    }

    pub fn release(&mut self, key: VirtualKeyCode) {
        self.keys.insert(key, false);
        self.this_frame.insert(key, true);
    }

    pub fn pressed_this_frame(&self, key: &VirtualKeyCode) -> bool {
        match self.keys.get(&key) {
            None | Some(false) => false,
            Some(true) => match self.this_frame.get(&key) {
                None | Some(false) => false,
                Some(true) => true,
            },
        }
    }

    pub fn released_this_frame(&self, key: &VirtualKeyCode) -> bool {
        match self.keys.get(&key) {
            Some(true) => false,
            None | Some(false) => match self.this_frame.get(&key) {
                None | Some(false) => false,
                Some(true) => true,
            },
        }
    }

    pub fn is_pressed(&self, key: &VirtualKeyCode) -> bool {
        match self.keys.get(&key) {
            None | Some(false) => false,
            Some(true) => true,
        }
    }

    pub fn next_frame(&mut self) {
        self.this_frame.clear();
    }
}
