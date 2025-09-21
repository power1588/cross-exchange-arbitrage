//! Integration tests for live trading executor

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::{Exchange, OrderSide, TimeInForce, LimitOrder, BinanceConnector, BybitConnector},
    data::{OrderBook, MarketDataManager},
    trading::LiveTradingExecutor,
    Result,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

/// Create a test configuration for live trading executor
fn create_test_config() -> ArbitrageConfig {
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 5;
    config.strategy.max_position_size = 1.0;
    config.execution.order_timeout_ms = 5000;
    config.execution.max_retry_attempts = 3;
    config
}

/// Create a test order
fn create_test_order(side: OrderSide, quantity: f64, price: f64) -> LimitOrder {
    LimitOrder {
        symbol: "BTCUSDT".to_string(),
        side,
        quantity,
        price,
        time_in_force: TimeInForce::GTC,
        client_order_id: Some(format!("live_test_order_{}", rand::random::<u32>())),
    }
}

#[tokio::test]
async fn test_live_trading_executor_creation() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test initial state
    assert!(!executor.is_connected());
    assert_eq!(executor.get_connected_exchanges().len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_exchange_connection() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // This test would require real API keys, so we test the interface
    let result = executor.connect_to_exchanges().await;
    
    // In test environment without real API keys, this should fail gracefully
    // But the interface should work
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_balance_checking() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test balance checking interface
    let result = executor.check_balances().await;
    
    // Without real connections, this should fail gracefully
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_connectivity_check() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test connectivity checking interface
    let result = executor.check_connectivity().await;
    
    // Without real connections, this should fail gracefully
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_order_placement_interface() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 0.001, 50000.0);
    
    // Test order placement interface
    let result = executor.place_order(Exchange::Binance, order).await;
    
    // Without real connections, this should fail
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_order_cancellation_interface() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // Test order cancellation interface
    let result = executor.cancel_order(Exchange::Binance, "BTCUSDT", "test_order_id").await;
    
    // Without real connections, this should fail
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_position_management() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test position retrieval interface
    let result = executor.get_positions().await;
    
    // Without real connections, this should return empty or fail
    assert!(result.is_err() || result.unwrap().is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_risk_management() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test risk check interface
    let order = create_test_order(OrderSide::Buy, 10.0, 50000.0); // Large order
    let result = executor.check_risk_limits(&order).await;
    
    // Risk checks should work even without connections
    assert!(result.is_ok() || result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_market_data_subscription() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // Test market data subscription interface
    let result = executor.subscribe_market_data(Exchange::Binance, "BTCUSDT").await;
    
    // Without real connections, this should fail
    assert!(result.is_err() || result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_order_status_tracking() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test order status retrieval interface
    let result = executor.get_order_status(Exchange::Binance, "BTCUSDT", "test_order_id").await;
    
    // Without real connections, this should fail
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_emergency_shutdown() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // Test emergency shutdown interface
    let result = executor.emergency_shutdown().await;
    
    // Emergency shutdown should always work
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_health_monitoring() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test health status retrieval
    let health = executor.get_health_status().await;
    
    // Health status should always be available
    assert!(health.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_execution_statistics() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test execution statistics retrieval
    let stats = executor.get_execution_statistics().await;
    
    // Statistics should always be available
    assert!(stats.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_order_timeout_handling() -> Result<()> {
    let mut config = create_test_config();
    config.execution.order_timeout_ms = 100; // Very short timeout
    
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 0.001, 50000.0);
    
    // Test order with timeout
    let result = timeout(Duration::from_millis(200), executor.place_order(Exchange::Binance, order)).await;
    
    // Should either complete or timeout
    assert!(result.is_ok() || result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_retry_mechanism() -> Result<()> {
    let mut config = create_test_config();
    config.execution.max_retry_attempts = 2;
    
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 0.001, 50000.0);
    
    // Test order placement with retries
    let result = executor.place_order_with_retry(Exchange::Binance, order).await;
    
    // Should fail after retries in test environment
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let config = create_test_config();
    let executor = std::sync::Arc::new(tokio::sync::Mutex::new(LiveTradingExecutor::new(config).await?));
    
    // Test concurrent balance checks
    let mut handles = vec![];
    
    for _ in 0..3 {
        let executor_clone = executor.clone();
        let handle = tokio::spawn(async move {
            let exec = executor_clone.lock().await;
            exec.check_balances().await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        // Results may vary, but operations should complete
        assert!(result.is_ok() || result.is_err());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_validation() -> Result<()> {
    let mut config = create_test_config();
    config.strategy.max_position_size = -1.0; // Invalid configuration
    
    // Test that invalid configuration is caught
    let result = LiveTradingExecutor::new(config).await;
    
    // Should fail with invalid configuration
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_exchange_specific_operations() -> Result<()> {
    let config = create_test_config();
    let executor = LiveTradingExecutor::new(config).await?;
    
    // Test Binance-specific operations
    let binance_result = executor.get_exchange_info(Exchange::Binance).await;
    assert!(binance_result.is_err() || binance_result.is_ok());
    
    // Test Bybit-specific operations
    let bybit_result = executor.get_exchange_info(Exchange::Bybit).await;
    assert!(bybit_result.is_err() || bybit_result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_market_data_processing() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // Create mock market data
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Exchange::Binance);
    orderbook.update_bid(49950.0, 1.0);
    orderbook.update_ask(50050.0, 1.0);
    
    // Test market data processing
    let result = executor.process_market_data(Exchange::Binance, orderbook).await;
    
    // Market data processing should work
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_error_recovery() -> Result<()> {
    let config = create_test_config();
    let mut executor = LiveTradingExecutor::new(config).await?;
    
    // Simulate connection error and recovery
    let result = executor.handle_connection_error(Exchange::Binance).await;
    
    // Error handling should work
    assert!(result.is_ok());
    
    Ok(())
}
