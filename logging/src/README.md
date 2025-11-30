# My logging util

# Improvements

potentially use somthing akin to:
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::logging_function(
            $crate::LoggingLevel::Info,
            &format!(
                "{}:{} [{}] {}",
                file!(),
                line!(),
                std::thread::current().name().unwrap_or("thread"),
                format!($($arg)*)
            )
        );
    };
}
to allow for new features
