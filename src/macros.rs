#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::with_logger(format!($($arg)*), $crate::LogType::Info);
    }};
}

#[macro_export]
macro_rules! err{
    ($($arg:tt)*) => {{
        $crate::with_logger(format!($($arg)*), $crate::LogType::Err);
    }};
}

#[macro_export]
macro_rules! warn{
    ($($arg:tt)*) => {{
        $crate::with_logger(format!($($arg)*), $crate::LogType::Warn);
    }};
}
