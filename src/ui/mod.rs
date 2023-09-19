pub mod components;
pub mod widgets;

#[macro_export]
macro_rules! title {
    ($fmt:expr) => {
        concat!("˧ ", $fmt, " ꜔")
    };
    ($fmt:expr, $($arg:tt)*) => {
        format!(concat!("˧ ", $fmt, " ꜔"), $($arg)*)
    };
}
