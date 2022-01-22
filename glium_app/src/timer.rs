use std::time::Instant;

pub struct Timer {
    last: Instant,
    fps: u32,
    last_delta: f32,
    frame_min_duration: f32,
    frame_count: u32,
    frame_time: f32,
    frame_update_time: f32,

    abs_time: f32,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            last: Instant::now(),
            fps: 0,
            last_delta: 0.0,
            frame_min_duration: 0.001,
            frame_count: 0,
            frame_time: 0.0,
            frame_update_time: 0.25,

            abs_time: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.last = Instant::now();
    }

    pub fn go(&mut self) -> Option<f32> {
        let now = self.last.elapsed();
        let delta = (now.as_micros() as f32) / 1_000_000.0;

        if delta < self.frame_min_duration {
            return None;
        }

        self.abs_time += self.last_delta;

        self.frame_count += 1;
        self.frame_time += delta;
        if self.frame_time > self.frame_update_time {
            self.fps = (self.frame_count as f32 * (1.0 / self.frame_time)) as u32;
            self.frame_count = 0;
            self.frame_time = 0.0;
        }

        self.last_delta = delta;
        self.last = Instant::now();
        Some(delta)
    }

    pub fn set_frame_min_duration(&mut self, dur: f32) {
        self.frame_min_duration = dur;
    }

    pub fn set_frame_update_time(&mut self, dur: f32) {
        self.frame_update_time = dur;
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    pub fn delta(&self) -> f32 {
        self.last_delta
    }

    pub fn absolute_time(&self) -> f32 {
        self.abs_time
    }
}
