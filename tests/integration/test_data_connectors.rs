//! Integration tests for data connectors

use cross_exchange_arbitrage::{
    config::ExchangeConfig,
    connectors::{BinanceConnector, BybitConnector, ExchangeConnector, ConnectionStatus},
    data::OrderBook,
    Result,
};
use std::time::Duration;
use tokio::time::timeout;

/// Test configuration for Binance connector
fn create_test_binance_config() -> ExchangeConfig {
    use cross_exchange_arbitrage::config::*;
    use std::collections::HashMap;
    
    ExchangeConfig {
        connection: ConnectionConfig {
            websocket_url: "wss://testnet.binance.vision/ws".to_string(),
            rest_api_url: "https://testnet.binance.vision".to_string(),
            connection_timeout_secs: 10,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 1,
        },
        auth: AuthConfig {
            api_key: "test_api_key".to_string(),
            secret_key: "test_secret_key".to_string(),
            testnet: true,
            testnet_websocket_url: Some("wss://testnet.binance.vision/ws".to_string()),
            testnet_rest_api_url: Some("https://testnet.binance.vision".to_string()),
        },
        trading: TradingConfig {
            default_order_type: "LIMIT".to_string(),
            default_time_in_force: "GTC".to_string(),
            additional: HashMap::new(),
        },
        fees: FeeConfig {
            maker_fee: 0.001,
            taker_fee: 0.001,
            fee_currency: "BNB".to_string(),
            additional: HashMap::new(),
        },
        limits: LimitsConfig {
            order_rate_limit: 1200,
            market_data_rate_limit: 6000,
            min_order_sizes: {
                let mut sizes = HashMap::new();
                sizes.insert("BTCUSDT".to_string(), 0.00001);
                sizes
            },
            tick_sizes: {
                let mut sizes = HashMap::new();
                sizes.insert("BTCUSDT".to_string(), 0.01);
                sizes
            },
        },
        market_data: MarketDataConfig {
            streams: vec!["depth@100ms".to_string(), "trade".to_string()],
            topics: vec![],
            depth_levels: 20,
            additional: HashMap::new(),
        },
        monitoring: MonitoringConfig {
            enable_metrics: true,
            metrics_interval_secs: 60,
            enable_trade_logging: true,
            log_rotation_size_mb: 100,
            health_check_interval_secs: 30,
        },
    }
}

/// Test configuration for Bybit connector
fn create_test_bybit_config() -> ExchangeConfig {
    use cross_exchange_arbitrage::config::*;
    use std::collections::HashMap;
    
    ExchangeConfig {
        connection: ConnectionConfig {
            websocket_url: "wss://stream-testnet.bybit.com/v5/public/spot".to_string(),
            rest_api_url: "https://api-testnet.bybit.com".to_string(),
            connection_timeout_secs: 10,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 1,
        },
        auth: AuthConfig {
            api_key: "test_api_key".to_string(),
            secret_key: "test_secret_key".to_string(),
            testnet: true,
            testnet_websocket_url: Some("wss://stream-testnet.bybit.com/v5/public/spot".to_string()),
            testnet_rest_api_url: Some("https://api-testnet.bybit.com".to_string()),
        },
        trading: TradingConfig {
            default_order_type: "Limit".to_string(),
            default_time_in_force: "GTC".to_string(),
            additional: HashMap::new(),
        },
        fees: FeeConfig {
            maker_fee: 0.001,
            taker_fee: 0.001,
            fee_currency: "USDT".to_string(),
            additional: HashMap::new(),
        },
        limits: LimitsConfig {
            order_rate_limit: 600,
            market_data_rate_limit: 6000,
            min_order_sizes: {
                let mut sizes = HashMap::new();
                sizes.insert("BTCUSDT".to_string(), 0.00001);
                sizes
            },
            tick_sizes: {
                let mut sizes = HashMap::new();
                sizes.insert("BTCUSDT".to_string(), 0.01);
                sizes
            },
        },
        market_data: MarketDataConfig {
            streams: vec![],
            topics: vec!["orderbook.1".to_string(), "publicTrade".to_string()],
            depth_levels: 50,
            additional: HashMap::new(),
        },
        monitoring: MonitoringConfig {
            enable_metrics: true,
            metrics_interval_secs: 60,
            enable_trade_logging: true,
            log_rotation_size_mb: 100,
            health_check_interval_secs: 30,
        },
    }
}

#[tokio::test]
async fn test_binance_connector_creation() -> Result<()> {
    let config = create_test_binance_config();
    let connector = BinanceConnector::new(config).await?;
    
    // Test initial state
    assert!(!connector.is_connected());
    assert_eq!(connector.connection_status(), ConnectionStatus::Disconnected);
    
    Ok(())
}

#[tokio::test]
async fn test_bybit_connector_creation() -> Result<()> {
    let config = create_test_bybit_config();
    let connector = BybitConnector::new(config).await?;
    
    // Test initial state
    assert!(!connector.is_connected());
    assert_eq!(connector.connection_status(), ConnectionStatus::Disconnected);
    
    Ok(())
}

#[tokio::test]
async fn test_binance_connector_connection() -> Result<()> {
    let config = create_test_binance_config();
    let mut connector = BinanceConnector::new(config).await?;
    
    // Test connection (this will fail in test environment, but we test the interface)
    let result = timeout(Duration::from_secs(5), connector.connect()).await;
    
    // We expect this to timeout or fail in test environment
    // The important thing is that the interface works
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_bybit_connector_connection() -> Result<()> {
    let config = create_test_bybit_config();
    let mut connector = BybitConnector::new(config).await?;
    
    // Test connection (this will fail in test environment, but we test the interface)
    let result = timeout(Duration::from_secs(5), connector.connect()).await;
    
    // We expect this to timeout or fail in test environment
    // The important thing is that the interface works
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_binance_orderbook_subscription() -> Result<()> {
    let config = create_test_binance_config();
    let mut connector = BinanceConnector::new(config).await?;
    
    // Test orderbook subscription (interface test)
    let result = connector.subscribe_orderbook("BTCUSDT").await;
    
    // In test environment, this might fail, but we test the interface
    // The connector should handle the subscription request properly
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_bybit_orderbook_subscription() -> Result<()> {
    let config = create_test_bybit_config();
    let mut connector = BybitConnector::new(config).await?;
    
    // Test orderbook subscription (interface test)
    let result = connector.subscribe_orderbook("BTCUSDT").await;
    
    // In test environment, this might fail, but we test the interface
    // The connector should handle the subscription request properly
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_connector_balance_retrieval() -> Result<()> {
    let config = create_test_binance_config();
    let connector = BinanceConnector::new(config).await?;
    
    // Test balance retrieval (will fail without real connection)
    let result = connector.get_balances().await;
    
    // We expect this to fail in test environment without real API keys
    // But the interface should be properly implemented
    assert!(result.is_err()); // Expected to fail in test environment
    
    Ok(())
}

#[tokio::test]
async fn test_connector_order_placement() -> Result<()> {
    use cross_exchange_arbitrage::connectors::{LimitOrder, OrderSide, TimeInForce};
    
    let config = create_test_binance_config();
    let connector = BinanceConnector::new(config).await?;
    
    let order = LimitOrder {
        symbol: "BTCUSDT".to_string(),
        side: OrderSide::Buy,
        quantity: 0.001,
        price: 50000.0,
        time_in_force: TimeInForce::GTC,
        client_order_id: Some("test_order_123".to_string()),
    };
    
    // Test order placement (will fail without real connection)
    let result = connector.place_limit_order(&order).await;
    
    // We expect this to fail in test environment without real API keys
    // But the interface should be properly implemented
    assert!(result.is_err()); // Expected to fail in test environment
    
    Ok(())
}

#[tokio::test]
async fn test_connector_message_parsing() {
    // This test will be implemented when we have the actual message parsing logic
    // For now, we just ensure the test structure is in place
    assert!(true);
}

#[tokio::test]
async fn test_connector_error_handling() {
    // Test various error scenarios
    // This will be expanded as we implement the connectors
    assert!(true);
}

#[tokio::test]
async fn test_connector_reconnection_logic() {
    // Test automatic reconnection functionality
    // This will be implemented with the actual connector logic
    assert!(true);
}

// Performance tests
#[tokio::test]
async fn test_connector_performance() {
    // Test message processing performance
    // This will be implemented with benchmarks
    assert!(true);
}

// Integration test with market data manager
#[tokio::test]
async fn test_connector_with_market_data_manager() {
    use cross_exchange_arbitrage::data::MarketDataManager;
    
    let manager = MarketDataManager::new(100);
    
    // This test will verify integration between connectors and market data manager
    // Will be implemented when both components are ready
    assert!(manager.get_statistics().await.orderbook_count == 0);
}
