//! Configuration management module

pub mod settings;

pub use settings::*;

use crate::{ArbitrageError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure for the arbitrage system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Strategy configuration
    pub strategy: StrategyConfig,
    /// Risk management configuration
    pub risk: RiskConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Exchange configuration
    pub exchanges: ExchangeListConfig,
}

/// Strategy-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Trading symbol
    pub symbol: String,
    /// Minimum spread in basis points
    pub min_spread_bps: u32,
    /// Maximum position size per exchange
    pub max_position_size: f64,
    /// Rebalance threshold
    pub rebalance_threshold: f64,
    /// Minimum profit threshold in USD
    pub min_profit_usd: f64,
    /// Maximum concurrent positions
    pub max_concurrent_positions: u32,
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Stop loss in basis points
    pub stop_loss_bps: u32,
    /// Position limit per exchange
    pub position_limit: f64,
    /// Daily loss limit in USD
    pub daily_loss_limit: f64,
    /// Volatility threshold
    pub volatility_threshold: f64,
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Order timeout in milliseconds
    pub order_timeout_ms: u64,
    /// Slippage tolerance
    pub slippage_tolerance: f64,
    /// Minimum order size
    pub min_order_size: f64,
    /// Order size fraction
    pub order_size_fraction: f64,
    /// Allow partial fills
    pub allow_partial_fills: bool,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics update interval in seconds
    pub metrics_interval_secs: u64,
    /// Enable trade logging
    pub enable_trade_logging: bool,
    /// Log rotation size in MB
    pub log_rotation_size_mb: u64,
    /// Health check interval in seconds
    pub health_check_interval_secs: u64,
}

/// Exchange list configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeListConfig {
    /// Enabled exchanges
    pub enabled: Vec<String>,
    /// Primary exchange for reference pricing
    pub primary_exchange: String,
}

/// Individual exchange configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Authentication settings
    pub auth: AuthConfig,
    /// Trading settings
    pub trading: TradingConfig,
    /// Fee settings
    pub fees: FeeConfig,
    /// Rate limits and constraints
    pub limits: LimitsConfig,
    /// Market data settings
    pub market_data: MarketDataConfig,
    /// Monitoring settings
    pub monitoring: MonitoringConfig,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// WebSocket URL
    pub websocket_url: String,
    /// REST API URL
    pub rest_api_url: String,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay_secs: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// API key
    pub api_key: String,
    /// Secret key
    pub secret_key: String,
    /// Enable testnet
    pub testnet: bool,
    /// Testnet WebSocket URL
    pub testnet_websocket_url: Option<String>,
    /// Testnet REST API URL
    pub testnet_rest_api_url: Option<String>,
}

/// Trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Default order type
    pub default_order_type: String,
    /// Default time in force
    pub default_time_in_force: String,
    /// Additional trading settings (exchange-specific)
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Fee configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    /// Maker fee
    pub maker_fee: f64,
    /// Taker fee
    pub taker_fee: f64,
    /// Fee currency
    pub fee_currency: String,
    /// Additional fee settings
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Limits and constraints configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Order rate limit (requests per minute)
    pub order_rate_limit: u32,
    /// Market data rate limit (requests per minute)
    pub market_data_rate_limit: u32,
    /// Minimum order sizes by symbol
    pub min_order_sizes: std::collections::HashMap<String, f64>,
    /// Tick sizes by symbol
    pub tick_sizes: std::collections::HashMap<String, f64>,
}

/// Market data configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataConfig {
    /// Subscription streams/topics
    #[serde(default)]
    pub streams: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    /// Order book depth levels
    pub depth_levels: u32,
    /// Additional market data settings
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

impl ArbitrageConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ArbitrageError::Config(format!("Failed to read config file: {}", e)))?;
        
        let mut config: ArbitrageConfig = toml::from_str(&content)
            .map_err(|e| ArbitrageError::Config(format!("Failed to parse config: {}", e)))?;
        
        // Expand environment variables
        config.expand_env_vars()?;
        
        Ok(config)
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate strategy config
        if self.strategy.symbol.is_empty() {
            return Err(ArbitrageError::Config("Symbol cannot be empty".to_string()).into());
        }
        
        if self.strategy.min_spread_bps == 0 {
            return Err(ArbitrageError::Config("Minimum spread must be greater than 0".to_string()).into());
        }
        
        if self.strategy.max_position_size <= 0.0 {
            return Err(ArbitrageError::Config("Maximum position size must be positive".to_string()).into());
        }
        
        // Validate risk config
        if self.risk.max_drawdown <= 0.0 || self.risk.max_drawdown >= 1.0 {
            return Err(ArbitrageError::Config("Max drawdown must be between 0 and 1".to_string()).into());
        }
        
        // Validate execution config
        if self.execution.order_timeout_ms == 0 {
            return Err(ArbitrageError::Config("Order timeout must be greater than 0".to_string()).into());
        }
        
        // Validate exchanges
        if self.exchanges.enabled.is_empty() {
            return Err(ArbitrageError::Config("At least one exchange must be enabled".to_string()).into());
        }
        
        if self.exchanges.enabled.len() < 2 {
            return Err(ArbitrageError::Config("At least two exchanges required for arbitrage".to_string()).into());
        }
        
        Ok(())
    }
    
    /// Expand environment variables in configuration
    fn expand_env_vars(&mut self) -> Result<()> {
        // This is a simplified implementation
        // In a real implementation, you would recursively traverse all string fields
        // and expand ${VAR_NAME} patterns
        Ok(())
    }
}

impl ExchangeConfig {
    /// Load exchange configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ArbitrageError::Config(format!("Failed to read exchange config file: {}", e)))?;
        
        let config: ExchangeConfig = toml::from_str(&content)
            .map_err(|e| ArbitrageError::Config(format!("Failed to parse exchange config: {}", e)))?;
        
        Ok(config)
    }
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            strategy: StrategyConfig {
                symbol: "BTCUSDT".to_string(),
                min_spread_bps: 10,
                max_position_size: 1.0,
                rebalance_threshold: 0.1,
                min_profit_usd: 5.0,
                max_concurrent_positions: 3,
            },
            risk: RiskConfig {
                max_drawdown: 0.05,
                stop_loss_bps: 50,
                position_limit: 10.0,
                daily_loss_limit: 1000.0,
                volatility_threshold: 0.1,
            },
            execution: ExecutionConfig {
                order_timeout_ms: 5000,
                slippage_tolerance: 0.001,
                min_order_size: 0.001,
                order_size_fraction: 0.1,
                allow_partial_fills: true,
                max_retry_attempts: 3,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_interval_secs: 60,
                enable_trade_logging: true,
                log_rotation_size_mb: 100,
                health_check_interval_secs: 30,
            },
            exchanges: ExchangeListConfig {
                enabled: vec!["binance".to_string(), "bybit".to_string()],
                primary_exchange: "binance".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_default_config_validation() {
        let config = ArbitrageConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = ArbitrageConfig::default();
        config.strategy.min_spread_bps = 0;
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = ArbitrageConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(!toml_str.is_empty());
        
        let parsed_config: ArbitrageConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.strategy.symbol, parsed_config.strategy.symbol);
    }

    #[test]
    fn test_config_from_file() {
        let config = ArbitrageConfig::default();
        let toml_content = toml::to_string(&config).unwrap();
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        
        let loaded_config = ArbitrageConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.strategy.symbol, loaded_config.strategy.symbol);
    }
}
