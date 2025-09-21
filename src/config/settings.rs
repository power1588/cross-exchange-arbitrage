//! Settings management utilities

use crate::{ArbitrageError, Result};
use std::env;
use std::collections::HashMap;

/// Environment variable expansion utility
pub struct EnvExpander;

impl EnvExpander {
    /// Expand environment variables in a string
    /// Supports ${VAR_NAME} and $VAR_NAME patterns
    pub fn expand(input: &str) -> Result<String> {
        let mut result = input.to_string();
        
        // Handle ${VAR_NAME} pattern
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = env::var(var_name)
                    .map_err(|_| ArbitrageError::Config(
                        format!("Environment variable '{}' not found", var_name)
                    ))?;
                
                result.replace_range(start..start + end + 1, &var_value);
            } else {
                return Err(ArbitrageError::Config(
                    "Unclosed environment variable reference".to_string()
                ).into());
            }
        }
        
        Ok(result)
    }
    
    /// Expand environment variables in a HashMap of string values
    pub fn expand_map(map: &mut HashMap<String, String>) -> Result<()> {
        for (_, value) in map.iter_mut() {
            *value = Self::expand(value)?;
        }
        Ok(())
    }
}

/// Configuration validation utilities
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a symbol format
    pub fn validate_symbol(symbol: &str) -> Result<()> {
        if symbol.is_empty() {
            return Err(ArbitrageError::Config("Symbol cannot be empty".to_string()).into());
        }
        
        if !symbol.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ArbitrageError::Config(
                "Symbol must contain only alphanumeric characters".to_string()
            ).into());
        }
        
        Ok(())
    }
    
    /// Validate a percentage value (0.0 to 1.0)
    pub fn validate_percentage(value: f64, name: &str) -> Result<()> {
        if !(0.0..=1.0).contains(&value) {
            return Err(ArbitrageError::Config(
                format!("{} must be between 0.0 and 1.0", name)
            ).into());
        }
        Ok(())
    }
    
    /// Validate a positive value
    pub fn validate_positive(value: f64, name: &str) -> Result<()> {
        if value <= 0.0 {
            return Err(ArbitrageError::Config(
                format!("{} must be positive", name)
            ).into());
        }
        Ok(())
    }
    
    /// Validate a URL format
    pub fn validate_url(url: &str, name: &str) -> Result<()> {
        if url.is_empty() {
            return Err(ArbitrageError::Config(
                format!("{} cannot be empty", name)
            ).into());
        }
        
        if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("wss://") && !url.starts_with("ws://") {
            return Err(ArbitrageError::Config(
                format!("{} must be a valid URL", name)
            ).into());
        }
        
        Ok(())
    }
}

/// Configuration defaults
pub struct ConfigDefaults;

impl ConfigDefaults {
    /// Default connection timeout in seconds
    pub const CONNECTION_TIMEOUT_SECS: u64 = 10;
    
    /// Default reconnection attempts
    pub const MAX_RECONNECT_ATTEMPTS: u32 = 5;
    
    /// Default reconnection delay in seconds
    pub const RECONNECT_DELAY_SECS: u64 = 5;
    
    /// Default order timeout in milliseconds
    pub const ORDER_TIMEOUT_MS: u64 = 5000;
    
    /// Default slippage tolerance
    pub const SLIPPAGE_TOLERANCE: f64 = 0.001;
    
    /// Default minimum spread in basis points
    pub const MIN_SPREAD_BPS: u32 = 10;
    
    /// Default maximum drawdown
    pub const MAX_DRAWDOWN: f64 = 0.05;
    
    /// Default stop loss in basis points
    pub const STOP_LOSS_BPS: u32 = 50;
    
    /// Default maker fee
    pub const MAKER_FEE: f64 = 0.001;
    
    /// Default taker fee
    pub const TAKER_FEE: f64 = 0.001;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_expansion() {
        env::set_var("TEST_VAR", "test_value");
        
        let input = "prefix_${TEST_VAR}_suffix";
        let result = EnvExpander::expand(input).unwrap();
        assert_eq!(result, "prefix_test_value_suffix");
        
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_env_expansion_missing_var() {
        let input = "prefix_${MISSING_VAR}_suffix";
        let result = EnvExpander::expand(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_symbol_validation() {
        assert!(ConfigValidator::validate_symbol("BTCUSDT").is_ok());
        assert!(ConfigValidator::validate_symbol("").is_err());
        assert!(ConfigValidator::validate_symbol("BTC-USDT").is_err());
    }

    #[test]
    fn test_percentage_validation() {
        assert!(ConfigValidator::validate_percentage(0.5, "test").is_ok());
        assert!(ConfigValidator::validate_percentage(0.0, "test").is_ok());
        assert!(ConfigValidator::validate_percentage(1.0, "test").is_ok());
        assert!(ConfigValidator::validate_percentage(-0.1, "test").is_err());
        assert!(ConfigValidator::validate_percentage(1.1, "test").is_err());
    }

    #[test]
    fn test_positive_validation() {
        assert!(ConfigValidator::validate_positive(1.0, "test").is_ok());
        assert!(ConfigValidator::validate_positive(0.1, "test").is_ok());
        assert!(ConfigValidator::validate_positive(0.0, "test").is_err());
        assert!(ConfigValidator::validate_positive(-1.0, "test").is_err());
    }

    #[test]
    fn test_url_validation() {
        assert!(ConfigValidator::validate_url("https://api.example.com", "test").is_ok());
        assert!(ConfigValidator::validate_url("wss://stream.example.com", "test").is_ok());
        assert!(ConfigValidator::validate_url("", "test").is_err());
        assert!(ConfigValidator::validate_url("invalid-url", "test").is_err());
    }
}
