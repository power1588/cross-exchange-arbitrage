//! Logging utilities

use crate::Result;
use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Registry,
};

/// Initialize logging system
pub fn init<P: AsRef<Path>>(log_level: &str, log_file: P) -> Result<()> {
    // Create log directory if it doesn't exist
    if let Some(parent) = log_file.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Create file appender with daily rotation
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_file.as_ref().parent().unwrap_or(Path::new(".")),
        log_file.as_ref().file_name().unwrap_or(std::ffi::OsStr::new("app.log"))
    );
    
    // Create console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);
    
    // Create file layer
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_appender);
    
    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));
    
    // Initialize subscriber
    Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();
    
    Ok(())
}

/// Structured logging macros
#[macro_export]
macro_rules! log_trade {
    ($level:ident, $exchange:expr, $symbol:expr, $side:expr, $quantity:expr, $price:expr, $($field:tt)*) => {
        tracing::$level!(
            exchange = %$exchange,
            symbol = %$symbol,
            side = %$side,
            quantity = %$quantity,
            price = %$price,
            $($field)*
        );
    };
}

/// Log spread information with structured fields
#[macro_export]
macro_rules! log_spread {
    ($level:ident, $symbol:expr, $exchange1:expr, $exchange2:expr, $spread_bps:expr, $($field:tt)*) => {
        tracing::$level!(
            symbol = %$symbol,
            exchange1 = %$exchange1,
            exchange2 = %$exchange2,
            spread_bps = %$spread_bps,
            $($field)*
        );
    };
}

/// Log position information with structured fields
#[macro_export]
macro_rules! log_position {
    ($level:ident, $exchange:expr, $symbol:expr, $position:expr, $($field:tt)*) => {
        tracing::$level!(
            exchange = %$exchange,
            symbol = %$symbol,
            position = %$position,
            $($field)*
        );
    };
}

/// Log risk information with structured fields
#[macro_export]
macro_rules! log_risk {
    ($level:ident, $risk_type:expr, $value:expr, $threshold:expr, $($field:tt)*) => {
        tracing::$level!(
            risk_type = %$risk_type,
            value = %$value,
            threshold = %$threshold,
            $($field)*
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_logger_init() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");
        
        let result = init("info", &log_file);
        assert!(result.is_ok());
        
        // Test that we can log something
        tracing::info!("Test log message");
        
        // The log file should be created
        assert!(log_file.exists());
    }
}
