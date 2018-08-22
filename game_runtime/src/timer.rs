use std::time::Instant;

/// Simple timer for measuring time durations in seconds. It internally uses
/// [`std::time::Instant`] for its measurements.
///
/// # Examples
/// ```
/// let timer_program_start = Timer::new();
///
/// let timer_task = Timer::new();
/// // task_a();
/// let task_a_duration = timer_task.elapsed_time();
/// 
/// timer_task.reset();
/// // task_b();
/// let task_b_duration = timer_task.elapsed_time();
/// 
/// let task_a_b_duration = timer_program_start.elapsed_time();
/// // Now task_a_b_duration is very close to task_a_duration + task_b_duration
/// ```
pub struct Timer {
    start_time: Instant,
}

impl Timer {
    /// Creates a new timer 
    pub fn new() -> Timer {
        Timer {
            start_time: Instant::now(),
        }
    }

    /// Returns the number of seconds elapsed since either the creation of the timer,
    /// or the last time this timer was [`reset`].
    pub fn elapsed_time(&self) -> f64 {
        let now = Instant::now();
        let time_since_start = now.duration_since(self.start_time);
        let secs = time_since_start.as_secs() as f64;
        let nanos = (f64::from(time_since_start.subsec_nanos())) / 1_000_000_000.0;
        secs + nanos
    }

    /// Resets the timer such that the next call to [`elapsed_time`] will return the time
    /// since this reset call.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }
}
