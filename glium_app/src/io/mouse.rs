pub struct Mouse {
    this_frame: [bool; 10],
    pressed: [bool; 10],
    pos: (i32, i32),
    delta: (i32, i32),
    wheel: (f32, f32),
}

impl Mouse {
    pub fn new() -> Mouse {
        Mouse {
            this_frame: [false; 10],
            pressed: [false; 10],
            pos: (0, 0),
            delta: (0, 0),
            wheel: (0.0, 0.0),
        }
    }

    pub fn press_button(&mut self, button: usize) {
        self.this_frame[button] = true;
        self.pressed[button] = true;
    }

    pub fn release_button(&mut self, button: usize) {
        self.this_frame[button] = true;
        self.pressed[button] = false;
    }

    pub fn translate(&mut self, delta: (i32, i32)) {
        self.delta.0 += delta.0;
        self.delta.1 += delta.1;
        self.pos.0 += delta.0;
        self.pos.1 += delta.1;
    }

    pub fn update_pos(&mut self, pos: (i32, i32)) {
        self.delta.0 += pos.0 - self.pos.0;
        self.delta.1 += pos.1 - self.pos.1;
        self.pos = pos;
    }

    pub fn scroll(&mut self, wheel: (f32, f32)) {
        self.wheel.0 += wheel.0;
        self.wheel.1 += wheel.1;
    }

    pub fn next_frame(&mut self) {
        self.delta = (0, 0);
        self.wheel = (0.0, 0.0);
        self.this_frame = [false; 10];
    }

    pub fn get_pos(&self) -> (i32, i32) {
        self.pos
    }

    pub fn get_delta(&self) -> (i32, i32) {
        self.delta
    }

    pub fn get_scroll(&self) -> (f32, f32) {
        self.wheel
    }

    pub fn is_pressed(&self, button: usize) -> bool {
        self.pressed[button]
    }

    pub fn pressed_this_frame(&self, button: usize) -> bool {
        self.pressed[button] && self.this_frame[button]
    }

    pub fn released_this_frame(&self, button: usize) -> bool {
        !self.pressed[button] && self.this_frame[button]
    }
}
