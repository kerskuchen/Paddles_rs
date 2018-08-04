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
