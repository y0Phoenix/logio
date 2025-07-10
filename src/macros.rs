#[macro_export]
macro_rules! logio {
    ($($arg:tt)*) => {{
        $crate::with_logger(|logger| {
            logger.log(&format!($($arg)*));
        });
    }};
}