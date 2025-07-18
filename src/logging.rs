//! This crate is for implementing all of the logging and
//! info functions / macros used throughout the program.

// logging utils
use colored::Colorize;
use chrono;

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
    let time = chrono::offset::Local::now().to_string();
    let logging_level = match lvl {
        LoggingLevel::Trace => "TRACE".purple(),
        LoggingLevel::Info => "INFO ".blue(),
        LoggingLevel::Warn => "WARN ".yellow(),
        LoggingLevel::Error => "ERROR".red(),
        LoggingLevel::Fatal => "FATAL".black().on_bright_red(),
        _ => "not yet implemented".white()
    };
    println!( "@ [{}] {} | {}", time, logging_level.to_string(), str);
}

#[macro_export]
macro_rules! trace { ( $($arg:tt)* ) => { logging_function(LoggingLevel::Trace, &format!($($arg)*)); }; }
#[macro_export]
macro_rules! info  { ( $($arg:tt)* ) => { logging_function(LoggingLevel::Info,  &format!($($arg)*)); }; }
#[macro_export]
macro_rules! warn  { ( $($arg:tt)* ) => { logging_function(LoggingLevel::Warn,  &format!($($arg)*)); }; }
#[macro_export]
macro_rules! error { ( $($arg:tt)* ) => { logging_function(LoggingLevel::Error, &format!($($arg)*)); }; }
#[macro_export]
macro_rules! fatal { ( $($arg:tt)* ) => { logging_function(LoggingLevel::Fatal, &format!($($arg)*)); }; }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_length() {
        logging_function(LoggingLevel::Trace, "wow");
        logging_function(LoggingLevel::Info , "wow");
        logging_function(LoggingLevel::Warn , "wow");
        logging_function(LoggingLevel::Error, "wow");
        logging_function(LoggingLevel::Fatal, "wow");

        trace!("");
        info!("");
        warn!("");
        error!("");
        fatal!("");        
    }
}
