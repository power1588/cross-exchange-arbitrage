//! Exchange connector implementations

pub mod traits;
pub mod binance;
pub mod bybit;

pub use traits::*;
pub use binance::BinanceConnector;
pub use bybit::BybitConnector;

use crate::{ArbitrageError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported exchanges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    /// Binance exchange
    Binance,
    /// Bybit exchange
    Bybit,
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exchange::Binance => write!(f, "binance"),
            Exchange::Bybit => write!(f, "bybit"),
        }
    }
}

impl std::str::FromStr for Exchange {
    type Err = ArbitrageError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binance" => Ok(Exchange::Binance),
            "bybit" => Ok(Exchange::Bybit),
            _ => Err(ArbitrageError::Config(format!("Unknown exchange: {}", s))),
        }
    }
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Disconnected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Reconnecting
    Reconnecting,
    /// Error state
    Error,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "disconnected"),
            ConnectionStatus::Connecting => write!(f, "connecting"),
            ConnectionStatus::Connected => write!(f, "connected"),
            ConnectionStatus::Reconnecting => write!(f, "reconnecting"),
            ConnectionStatus::Error => write!(f, "error"),
        }
    }
}

/// Connector factory for creating exchange connectors
pub struct ConnectorFactory;

impl ConnectorFactory {
    /// Create a connector for the specified exchange
    pub async fn create_connector(
        exchange: Exchange,
        config: crate::config::ExchangeConfig,
    ) -> Result<Box<dyn ExchangeConnector + Send + Sync>> {
        match exchange {
            Exchange::Binance => {
                let connector = BinanceConnector::new(config).await?;
                Ok(Box::new(connector))
            }
            Exchange::Bybit => {
                let connector = BybitConnector::new(config).await?;
                Ok(Box::new(connector))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_from_str() {
        assert_eq!("binance".parse::<Exchange>().unwrap(), Exchange::Binance);
        assert_eq!("bybit".parse::<Exchange>().unwrap(), Exchange::Bybit);
        assert_eq!("BINANCE".parse::<Exchange>().unwrap(), Exchange::Binance);
        assert!("unknown".parse::<Exchange>().is_err());
    }

    #[test]
    fn test_exchange_display() {
        assert_eq!(Exchange::Binance.to_string(), "binance");
        assert_eq!(Exchange::Bybit.to_string(), "bybit");
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "disconnected");
    }
}
