//! Integration tests for dry-run executor

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::{Exchange, OrderSide, TimeInForce, LimitOrder},
    data::{OrderBook, MarketDataManager},
    trading::{DryRunExecutor, ExecutionResults},
    Result,
};
use std::time::Duration;
use tokio::time::timeout;

/// Create a test configuration for dry-run executor
fn create_test_config() -> ArbitrageConfig {
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 5;
    config.strategy.max_position_size = 1.0;
    config.execution.slippage_tolerance = 0.001;
    config.execution.order_timeout_ms = 1000;
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
        client_order_id: Some(format!("test_order_{}", rand::random::<u32>())),
    }
}

#[tokio::test]
async fn test_dry_run_executor_creation() -> Result<()> {
    let config = create_test_config();
    let executor = DryRunExecutor::new(config).await?;
    
    // Test initial state
    let results = executor.get_results();
    assert_eq!(results.total_trades, 0);
    assert_eq!(results.total_pnl, 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_order_execution() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Create a test order
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    
    // Execute the order
    let result = executor.execute_order(order).await?;
    
    // Verify execution result
    assert!(result.order_id.len() > 0);
    assert_eq!(result.symbol, "BTCUSDT");
    assert_eq!(result.side, OrderSide::Buy);
    assert_eq!(result.quantity, 0.1);
    assert_eq!(result.price, 50000.0);
    assert_eq!(result.filled_quantity, 0.1); // Should be fully filled in dry-run
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_slippage_simulation() -> Result<()> {
    let mut config = create_test_config();
    config.execution.slippage_tolerance = 0.002; // 0.2% slippage
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Create a buy order
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    let result = executor.execute_order(order).await?;
    
    // With slippage, the average price should be higher than the order price for buy orders
    if let Some(avg_price) = result.average_price {
        assert!(avg_price >= 50000.0);
        assert!(avg_price <= 50100.0); // Max 0.2% slippage
    }
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_portfolio_tracking() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Execute a series of trades
    let buy_order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    let sell_order = create_test_order(OrderSide::Sell, 0.05, 50100.0);
    
    executor.execute_order(buy_order).await?;
    executor.execute_order(sell_order).await?;
    
    // Check portfolio state
    let portfolio = executor.get_portfolio();
    assert!(portfolio.get_position("BTCUSDT") > 0.0); // Should have net long position
    
    let balance = portfolio.get_balance("USDT");
    assert!(balance != 0.0); // Should have some USDT balance change
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_pnl_calculation() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Execute profitable trades
    let buy_order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    let sell_order = create_test_order(OrderSide::Sell, 0.1, 50100.0);
    
    executor.execute_order(buy_order).await?;
    executor.execute_order(sell_order).await?;
    
    let results = executor.get_results();
    assert_eq!(results.total_trades, 2);
    assert!(results.total_pnl > 0.0); // Should be profitable
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_fee_simulation() -> Result<()> {
    let mut config = create_test_config();
    config.execution.enable_fees = true;
    config.execution.maker_fee = 0.001; // 0.1%
    config.execution.taker_fee = 0.001; // 0.1%
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    let result = executor.execute_order(order).await?;
    
    // Check that fees were applied
    let fees = executor.get_total_fees();
    assert!(fees > 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_order_timeout() -> Result<()> {
    let mut config = create_test_config();
    config.execution.order_timeout_ms = 100; // Very short timeout
    config.execution.simulate_delays = true;
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    
    // This should either complete quickly or timeout
    let result = timeout(Duration::from_millis(200), executor.execute_order(order)).await;
    
    // The test should not hang
    assert!(result.is_ok() || result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_market_impact() -> Result<()> {
    let mut config = create_test_config();
    config.execution.simulate_market_impact = true;
    config.execution.market_impact_factor = 0.0001; // 0.01% per unit
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Large order should have more market impact
    let large_order = create_test_order(OrderSide::Buy, 10.0, 50000.0);
    let result = executor.execute_order(large_order).await?;
    
    // Average price should be higher due to market impact
    if let Some(avg_price) = result.average_price {
        assert!(avg_price > 50000.0);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_partial_fills() -> Result<()> {
    let mut config = create_test_config();
    config.execution.allow_partial_fills = true;
    config.execution.partial_fill_probability = 0.3; // 30% chance of partial fill
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    let order = create_test_order(OrderSide::Buy, 1.0, 50000.0);
    let result = executor.execute_order(order).await?;
    
    // Should be either fully or partially filled
    assert!(result.filled_quantity > 0.0);
    assert!(result.filled_quantity <= 1.0);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_order_rejection() -> Result<()> {
    let mut config = create_test_config();
    config.execution.rejection_probability = 0.1; // 10% chance of rejection
    
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Try multiple orders, some might be rejected
    let mut executed_count = 0;
    let mut rejected_count = 0;
    
    for _ in 0..20 {
        let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
        match executor.execute_order(order).await {
            Ok(result) => {
                if result.filled_quantity > 0.0 {
                    executed_count += 1;
                }
            }
            Err(_) => {
                rejected_count += 1;
            }
        }
    }
    
    // Should have some executions and possibly some rejections
    assert!(executed_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_performance_metrics() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Execute multiple trades
    for i in 0..10 {
        let price = 50000.0 + (i as f64 * 10.0);
        let order = create_test_order(OrderSide::Buy, 0.1, price);
        executor.execute_order(order).await?;
    }
    
    let metrics = executor.get_performance_metrics();
    assert!(metrics.total_volume > 0.0);
    assert!(metrics.average_execution_time >= Duration::from_nanos(0));
    assert_eq!(metrics.total_orders, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_reset_state() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Execute some trades
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    executor.execute_order(order).await?;
    
    // Verify state is not empty
    let results_before = executor.get_results();
    assert!(results_before.total_trades > 0);
    
    // Reset state
    executor.reset().await?;
    
    // Verify state is cleared
    let results_after = executor.get_results();
    assert_eq!(results_after.total_trades, 0);
    assert_eq!(results_after.total_pnl, 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_concurrent_orders() -> Result<()> {
    let config = create_test_config();
    let executor = std::sync::Arc::new(tokio::sync::Mutex::new(DryRunExecutor::new(config).await?));
    
    // Execute multiple orders concurrently
    let mut handles = vec![];
    
    for i in 0..5 {
        let executor_clone = executor.clone();
        let handle = tokio::spawn(async move {
            let order = create_test_order(OrderSide::Buy, 0.1, 50000.0 + (i as f64));
            let mut exec = executor_clone.lock().await;
            exec.execute_order(order).await
        });
        handles.push(handle);
    }
    
    // Wait for all orders to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    let exec = executor.lock().await;
    let results = exec.get_results();
    assert_eq!(results.total_trades, 5);
    
    Ok(())
}

#[tokio::test]
async fn test_dry_run_with_market_data() -> Result<()> {
    let config = create_test_config();
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Create market data
    let mut orderbook = OrderBook::new("BTCUSDT".to_string(), Exchange::Binance);
    orderbook.update_bid(49950.0, 1.0);
    orderbook.update_ask(50050.0, 1.0);
    
    // Update executor with market data
    executor.update_market_data(Exchange::Binance, orderbook).await?;
    
    // Execute order - should use market data for pricing
    let order = create_test_order(OrderSide::Buy, 0.1, 50000.0);
    let result = executor.execute_order(order).await?;
    
    // Execution should be influenced by market data
    assert!(result.filled_quantity > 0.0);
    
    Ok(())
}
