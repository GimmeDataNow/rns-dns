//! This crate is for implementing all of the logging and
//! info functions / macros used throughout the program.

// logging utils
use chrono;
use colored::Colorize;

/// This is used for changing the behaviour of the logging
/// function.
#[allow(dead_code)]
pub enum LoggingLevel {
    Trace,
    Info,
    Warn,
    Error,
    Fatal,
}

/// This function builds and prints the provided messages
/// in accordance with the provided `LoggingLevel`.
///
/// This function takes in a desired `Logging_level` and
/// the message that should be displayed in the console.
/// It always print in the same format where the only
/// difference is the color and text that is used to
/// identify the message.
///
/// # Usage
/// This function should generally not be called on its own.
/// It should generally only be used inside of the context
/// of any of the wrapper macros.
#[allow(unreachable_patterns)]
pub fn logging_function(lvl: LoggingLevel, str: &str) {
    let now = chrono::Local::now();
    let time = now.format("%Y-%m-%d %H:%M:%S %:z");
    let logging_level = match lvl {
        LoggingLevel::Trace => "TRACE".purple(),
        LoggingLevel::Info => "INFO ".blue(),
        LoggingLevel::Warn => "WARN ".yellow(),
        LoggingLevel::Error => "ERROR".red(),
        LoggingLevel::Fatal => "FATAL".black().on_bright_red(),
        _ => "not yet implemented".white(),
    };
    println!("@[{}] {} | {}", time, logging_level.to_string(), str);
}

#[allow(unreachable_patterns)]
pub fn logging_format(lvl: LoggingLevel, str: &str) -> String {
    let now = chrono::Local::now();
    let time = now.format("%Y-%m-%d %H:%M:%S %:z");
    let logging_level = match lvl {
        LoggingLevel::Trace => "TRACE".purple(),
        LoggingLevel::Info => "INFO ".blue(),
        LoggingLevel::Warn => "WARN ".yellow(),
        LoggingLevel::Error => "ERROR".red(),
        LoggingLevel::Fatal => "FATAL".black().on_bright_red(),
        _ => "not yet implemented".white(),
    };
    format!("@[{}] {} | {}", time, logging_level.to_string(), str)
}

#[macro_export]
macro_rules! trace { ( $($arg:tt)* ) => { $crate::logging_function($crate::LoggingLevel::Trace, &format!($($arg)*)); }; }
#[macro_export]
macro_rules! info  { ( $($arg:tt)* ) => { $crate::logging_function($crate::LoggingLevel::Info,  &format!($($arg)*)); }; }
#[macro_export]
macro_rules! warn  { ( $($arg:tt)* ) => { $crate::logging_function($crate::LoggingLevel::Warn,  &format!($($arg)*)); }; }
#[macro_export]
macro_rules! error { ( $($arg:tt)* ) => { $crate::logging_function($crate::LoggingLevel::Error, &format!($($arg)*)); }; }
#[macro_export]
macro_rules! fatal { ( $($arg:tt)* ) => { $crate::logging_function($crate::LoggingLevel::Fatal, &format!($($arg)*)); }; }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_length() {
        logging_function(LoggingLevel::Trace, "wow");
        logging_function(LoggingLevel::Info, "wow");
        logging_function(LoggingLevel::Warn, "wow");
        logging_function(LoggingLevel::Error, "wow");
        logging_function(LoggingLevel::Fatal, "wow");

        trace!("");
        info!("");
        warn!("");
        error!("");
        fatal!("");
    }
}
