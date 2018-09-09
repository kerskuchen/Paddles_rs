use math;

/// A macro for debugging which returns a string representation of an expression and its value
///
/// It uses the `stringify` macro internally and requires the input to be an expression.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate game_lib;
/// # fn main() {
///
/// let name = 5;
/// assert_eq!(dformat!(1 + 2), "1 + 2 = 3");
/// assert_eq!(dformat!(1 + name), "1 + name = 6");
/// assert_eq!(dformat!(name), "name = 5");
///
/// # }
/// ```
#[macro_export]
macro_rules! dformat {
    ($x:expr) => {
        format!("{} = {:?}", stringify!($x), $x)
    };
}

/// A macro used for debugging which prints a string containing the name and value of a given
/// variable.
///
/// It uses the `dformat` macro internally and requires the input to be an expression.
/// For more information see the `dformat` macro
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate game_lib;
/// # fn main() {
///
/// dprintln!(1 + 2);
/// // prints: "1 + 2 = 3"
///
/// let name = 5;
/// dprintln!(name);
/// // prints: "name = 5"
///
/// dprintln!(1 + name);
/// // prints: "1 + name = 6"
///
/// # }
/// ```
#[macro_export]
macro_rules! dprintln {
    ($x:expr) => {
        println!("{}", dformat!($x));
    };
}

//==================================================================================================
// CountdownTimer
//==================================================================================================
//

#[derive(Debug)]
pub struct CountdownTimer {
    cur_time: f32,
    end_time: f32,
}

impl Default for CountdownTimer {
    fn default() -> CountdownTimer {
        CountdownTimer::with_one_second_end_time()
    }
}

impl CountdownTimer {
    pub fn with_one_second_end_time() -> CountdownTimer {
        CountdownTimer {
            cur_time: 0.0,
            end_time: 1.0,
        }
    }

    pub fn with_given_end_time(end_time: f32) -> CountdownTimer {
        debug_assert!(end_time > math::EPSILON);

        CountdownTimer {
            cur_time: 0.0,
            end_time,
        }
    }

    pub fn increment(&mut self, delta_time: f32) {
        self.cur_time = f32::min(self.cur_time + delta_time, self.end_time);
    }

    pub fn is_finished(&self) -> bool {
        (self.end_time - self.cur_time) < math::EPSILON
    }

    pub fn is_running(&self) -> bool {
        !self.is_finished()
    }

    pub fn completion_ratio(&self) -> f32 {
        self.cur_time / self.end_time
    }

    pub fn restart(&mut self) {
        self.cur_time = 0.0;
    }
}
