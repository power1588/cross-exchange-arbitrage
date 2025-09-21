//! Cross-Exchange Arbitrage Strategy
//! 
//! A high-performance arbitrage trading system built on top of the HFTBacktest framework.
//! Supports real-time arbitrage opportunities detection and execution across multiple exchanges.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod connectors;
pub mod strategy;
pub mod data;
pub mod trading;
pub mod utils;

// Re-export commonly used types
pub use config::ArbitrageConfig;
pub use strategy::ArbitrageStrategy;
pub use connectors::{ExchangeConnector, Exchange};
pub use data::OrderBook;
pub use trading::{DryRunExecutor, LiveTradingExecutor};

/// Result type used throughout the application
pub type Result<T> = anyhow::Result<T>;

/// Common error types for the arbitrage system
#[derive(thiserror::Error, Debug)]
pub enum ArbitrageError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// Data parsing error
    #[error("Data parsing error: {0}")]
    DataParsing(String),
    
    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
    
    /// Not implemented error
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    /// Trading error
    #[error("Trading error: {0}")]
    Trading(String),
    
    /// Risk management error
    #[error("Risk management error: {0}")]
    RiskManagement(String),
    
    /// Timeout error
    #[error("Timeout error: {0}")]
    Timeout(String),
}

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert!(!APP_NAME.is_empty());
    }
}
