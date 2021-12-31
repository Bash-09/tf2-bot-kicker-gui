use std::time::Instant;



pub struct Timer {
    
    last: Instant,

    duration: f32,
    update: bool,

    last_delta: f32,

}

impl Timer {

    pub fn new() -> Timer {
        Timer {
            last: Instant::now(),

            duration: 0.0,
            update: false,

            last_delta: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.last = Instant::now();
        self.duration = 0.0;
    }


    pub fn go(&mut self, duration: f32) -> Option<f32> {
        let now = self.last.elapsed();
        let delta = (now.as_micros() as f32) / 1_000_000.0;

        if delta < 0.001 {
            return None;
        }

        self.update = false;
        self.duration += delta;
        if self.duration > duration {
            self.duration = 0.0;
            self.update = true;
        }

        self.last_delta = delta;
        self.last = Instant::now();
        Some(delta)
    }

    pub fn delta(&self) -> f32 {
        self.last_delta
    }

    pub fn update(&self) -> bool {
        self.update
    }



}