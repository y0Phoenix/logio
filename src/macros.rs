#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::utils::with_logger(format!($($arg)*), $crate::utils::LogType::Info);
    }};
}

#[macro_export]
macro_rules! err{
    ($($arg:tt)*) => {{
        $crate::utils::with_logger(format!($($arg)*), $crate::utils::LogType::Err);
    }};
}

#[macro_export]
macro_rules! warn{
    ($($arg:tt)*) => {{
        $crate::utils::with_logger(format!($($arg)*), $crate::utils::LogType::Warn);
    }};
}
