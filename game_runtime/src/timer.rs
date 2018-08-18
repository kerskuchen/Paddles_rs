use std::time::Instant;

pub struct Timer {
    start_time: Instant,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            start_time: Instant::now(),
        }
    }

    pub fn elapsed_time(&self) -> f64 {
        let now = Instant::now();
        let time_since_start = now.duration_since(self.start_time);
        let secs = time_since_start.as_secs() as f64;
        let nanos = (f64::from(time_since_start.subsec_nanos())) / 1_000_000_000.0;
        secs + nanos
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }
}
