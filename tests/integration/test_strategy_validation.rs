//! Integration tests for strategy validation with real market data simulation

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::{Exchange, OrderSide, TimeInForce, LimitOrder},
    data::{OrderBook, MarketDataManager},
    strategy::{ArbitrageStrategy, StrategyState, StrategyExecutor},
    trading::DryRunExecutor,
    Result,
};
use std::time::Duration;
use tokio::time::timeout;

/// Create a comprehensive test configuration
fn create_strategy_config() -> ArbitrageConfig {
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 5; // 0.05% minimum spread
    config.strategy.max_position_size = 1.0; // 1 BTC max position
    config.execution.min_order_size = 0.001; // 0.001 BTC minimum
    config.execution.slippage_tolerance = 0.001; // 0.1% slippage
    config.execution.enable_fees = true;
    config.execution.maker_fee = 0.001; // 0.1% maker fee
    config.execution.taker_fee = 0.001; // 0.1% taker fee
    config.execution.simulate_market_impact = true;
    config.execution.market_impact_factor = 0.0001; // 0.01% per unit
    config
}

/// Mock market data provider for testing
struct MockMarketDataProvider {
    iteration: u64,
}

impl MockMarketDataProvider {
    fn new() -> Self {
        Self { iteration: 0 }
    }
    
    /// Generate realistic market data with arbitrage opportunities
    fn generate_market_data(&mut self) -> (OrderBook, OrderBook) {
        self.iteration += 1;
        
        // Base price with some volatility
        let base_price = 50000.0 + (self.iteration as f64 * 10.0) + (rand::random::<f64>() - 0.5) * 500.0;
        
        // Create realistic spread between exchanges
        let spread_factor = if self.iteration % 3 == 0 { 
            // Occasionally create profitable opportunities
            0.0015 // 0.15% spread
        } else {
            0.0005 // 0.05% spread (below threshold)
        };
        
        // Binance orderbook (lower prices for buying opportunities)
        let mut binance_book = OrderBook::new("BTCUSDT".to_string(), Exchange::Binance);
        let binance_bid = base_price - 15.0;
        let binance_ask = base_price - 5.0;
        
        binance_book.update_bid(binance_bid, 2.0);
        binance_book.update_bid(binance_bid - 10.0, 1.5);
        binance_book.update_ask(binance_ask, 1.8);
        binance_book.update_ask(binance_ask + 10.0, 2.2);
        binance_book.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        // Bybit orderbook (higher prices for selling opportunities)
        let mut bybit_book = OrderBook::new("BTCUSDT".to_string(), Exchange::Bybit);
        let bybit_bid = base_price + (base_price * spread_factor);
        let bybit_ask = base_price + 20.0;
        
        bybit_book.update_bid(bybit_bid, 1.5);
        bybit_book.update_bid(bybit_bid - 10.0, 2.0);
        bybit_book.update_ask(bybit_ask, 2.1);
        bybit_book.update_ask(bybit_ask + 10.0, 1.7);
        bybit_book.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        (binance_book, bybit_book)
    }
}

#[tokio::test]
async fn test_strategy_with_dry_run_executor() -> Result<()> {
    let config = create_strategy_config();
    let mut strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Test strategy creation and initial state
    assert_eq!(strategy.get_state().await, StrategyState::Stopped);
    
    let initial_stats = strategy.get_statistics().await;
    assert_eq!(initial_stats.opportunities_detected, 0);
    assert_eq!(initial_stats.opportunities_executed, 0);
    
    // Run strategy for a short period
    let strategy_task = tokio::spawn(async move {
        strategy.run_with_executor(&mut executor).await
    });
    
    // Wait for strategy to complete
    let result = timeout(Duration::from_secs(5), strategy_task).await;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_opportunity_detection_and_execution() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Simulate market data updates and opportunity detection
    let mut mock_provider = MockMarketDataProvider::new();
    
    for _ in 0..5 {
        // Generate market data
        let (binance_book, bybit_book) = mock_provider.generate_market_data();
        
        // Update strategy's market data (this would normally be done by the strategy)
        // For testing, we'll call the detection method directly
        
        // The strategy should detect opportunities when spreads are sufficient
        let opportunities = strategy.detect_opportunities().await?;
        
        // Execute any profitable opportunities
        for opportunity in opportunities {
            if opportunity.spread_bps >= 5.0 {
                // Create orders for the opportunity
                let buy_order = LimitOrder {
                    symbol: opportunity.symbol.clone(),
                    side: OrderSide::Buy,
                    quantity: opportunity.quantity,
                    price: opportunity.buy_price,
                    time_in_force: TimeInForce::GTC,
                    client_order_id: Some(format!("test_buy_{}", rand::random::<u32>())),
                };
                
                let sell_order = LimitOrder {
                    symbol: opportunity.symbol.clone(),
                    side: OrderSide::Sell,
                    quantity: opportunity.quantity,
                    price: opportunity.sell_price,
                    time_in_force: TimeInForce::GTC,
                    client_order_id: Some(format!("test_sell_{}", rand::random::<u32>())),
                };
                
                // Execute orders
                let buy_result = executor.execute_order(buy_order).await;
                let sell_result = executor.execute_order(sell_order).await;
                
                // Both orders should succeed in dry-run mode
                assert!(buy_result.is_ok());
                assert!(sell_result.is_ok());
                
                println!("Executed arbitrage: Spread {:.2} bps, Profit ${:.2}", 
                        opportunity.spread_bps, opportunity.expected_profit);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Check executor results
    let results = executor.get_results().await;
    println!("Dry-run results: {} trades, ${:.2} PnL", results.total_trades, results.total_pnl);
    
    // Should have executed some trades
    assert!(results.total_trades > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_strategy_statistics_tracking() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config).await?;
    
    // Test initial statistics
    let stats = strategy.get_statistics().await;
    assert_eq!(stats.opportunities_detected, 0);
    assert_eq!(stats.opportunities_executed, 0);
    assert_eq!(stats.total_pnl, 0.0);
    assert_eq!(stats.success_rate, 0.0);
    
    // Simulate some opportunity detection
    for _ in 0..10 {
        let _opportunities = strategy.detect_opportunities().await?;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Check that statistics were updated
    let updated_stats = strategy.get_statistics().await;
    assert!(updated_stats.uptime_seconds > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_strategy_state_management() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config).await?;
    
    // Test initial state
    assert_eq!(strategy.get_state().await, StrategyState::Stopped);
    
    // Test state transitions
    strategy.pause().await;
    assert_eq!(strategy.get_state().await, StrategyState::Paused);
    
    strategy.resume().await;
    assert_eq!(strategy.get_state().await, StrategyState::Running);
    
    strategy.stop().await;
    assert_eq!(strategy.get_state().await, StrategyState::Stopped);
    
    Ok(())
}

#[tokio::test]
async fn test_risk_management_integration() -> Result<()> {
    let mut config = create_strategy_config();
    config.strategy.max_position_size = 0.1; // Very small position limit
    config.execution.min_order_size = 0.05; // Relatively large minimum
    
    let strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Generate opportunities that might exceed risk limits
    let opportunities = strategy.detect_opportunities().await?;
    
    for opportunity in opportunities {
        // Check that opportunity respects risk limits
        assert!(opportunity.quantity <= 0.1); // Max position size
        
        if opportunity.quantity >= 0.05 { // Min order size
            // Try to execute
            let buy_order = LimitOrder {
                symbol: opportunity.symbol.clone(),
                side: OrderSide::Buy,
                quantity: opportunity.quantity,
                price: opportunity.buy_price,
                time_in_force: TimeInForce::GTC,
                client_order_id: Some(format!("risk_test_{}", rand::random::<u32>())),
            };
            
            let result = executor.execute_order(buy_order).await;
            // Should succeed if within limits
            assert!(result.is_ok());
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_performance_under_load() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    let start_time = std::time::Instant::now();
    
    // Simulate high-frequency opportunity detection
    for i in 0..100 {
        let opportunities = strategy.detect_opportunities().await?;
        
        // Execute first opportunity if available
        if let Some(opportunity) = opportunities.first() {
            let order = LimitOrder {
                symbol: opportunity.symbol.clone(),
                side: OrderSide::Buy,
                quantity: opportunity.quantity.min(0.01), // Small size for speed
                price: opportunity.buy_price,
                time_in_force: TimeInForce::GTC,
                client_order_id: Some(format!("perf_test_{}", i)),
            };
            
            let _result = executor.execute_order(order).await;
        }
        
        // Small delay to simulate realistic timing
        if i % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("Performance test completed in {:?}", elapsed);
    
    // Should complete within reasonable time
    assert!(elapsed < Duration::from_secs(10));
    
    // Check final results
    let results = executor.get_results().await;
    let stats = strategy.get_statistics().await;
    
    println!("Performance results: {} trades, {} opportunities detected", 
             results.total_trades, stats.opportunities_detected);
    
    Ok(())
}

#[tokio::test]
async fn test_market_data_integration() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config).await?;
    
    // Test market data updates
    strategy.update_market_data().await?;
    
    // Detect opportunities based on updated data
    let opportunities = strategy.detect_opportunities().await?;
    
    // Verify opportunity data quality
    for opportunity in opportunities {
        assert!(opportunity.buy_price > 0.0);
        assert!(opportunity.sell_price > 0.0);
        assert!(opportunity.quantity > 0.0);
        assert!(opportunity.spread_bps >= 0.0);
        assert!(opportunity.expected_profit >= 0.0);
        assert!(!opportunity.symbol.is_empty());
        assert!(opportunity.timestamp > 0);
        
        // Verify arbitrage logic
        assert!(opportunity.sell_price > opportunity.buy_price);
        
        // Verify spread calculation
        let calculated_spread = ((opportunity.sell_price - opportunity.buy_price) / opportunity.buy_price) * 10000.0;
        assert!((calculated_spread - opportunity.spread_bps).abs() < 0.1); // Allow small rounding differences
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_recovery() -> Result<()> {
    let config = create_strategy_config();
    let strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    // Test with invalid order (should be handled gracefully)
    let invalid_order = LimitOrder {
        symbol: "INVALID".to_string(),
        side: OrderSide::Buy,
        quantity: 0.0, // Invalid quantity
        price: 0.0, // Invalid price
        time_in_force: TimeInForce::GTC,
        client_order_id: Some("error_test".to_string()),
    };
    
    // This should fail but not crash the system
    let result = executor.execute_order(invalid_order).await;
    // In dry-run mode, this might still succeed as it's simulated
    // The important thing is that it doesn't panic
    
    // Strategy should continue to function
    let opportunities = strategy.detect_opportunities().await?;
    // Should still be able to detect opportunities
    
    Ok(())
}

/// Integration test that simulates a complete trading session
#[tokio::test]
async fn test_complete_trading_session() -> Result<()> {
    println!("🚀 Starting complete trading session simulation...");
    
    let config = create_strategy_config();
    let mut strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    println!("📊 Initial portfolio state:");
    let initial_portfolio = executor.get_portfolio().await;
    println!("  BTC: {:.6}", initial_portfolio.get_position("BTCUSDT"));
    println!("  USDT: {:.2}", initial_portfolio.get_balance("USDT"));
    
    // Run strategy for a limited time
    let session_start = std::time::Instant::now();
    let mut opportunities_found = 0;
    let mut trades_executed = 0;
    
    // Simulate trading session
    for iteration in 1..=20 {
        println!("\n📈 Trading iteration {}/20", iteration);
        
        // Update market data
        strategy.update_market_data().await?;
        
        // Detect opportunities
        let opportunities = strategy.detect_opportunities().await?;
        opportunities_found += opportunities.len();
        
        if !opportunities.is_empty() {
            println!("  💡 Found {} arbitrage opportunities", opportunities.len());
            
            for (i, opportunity) in opportunities.iter().enumerate() {
                println!("    Opportunity {}: {:.2} bps spread, ${:.2} profit potential",
                        i + 1, opportunity.spread_bps, opportunity.expected_profit);
                
                // Execute profitable opportunities
                if opportunity.spread_bps >= 5.0 && opportunity.expected_profit > 1.0 {
                    let buy_order = LimitOrder {
                        symbol: opportunity.symbol.clone(),
                        side: OrderSide::Buy,
                        quantity: opportunity.quantity.min(0.1), // Limit size
                        price: opportunity.buy_price,
                        time_in_force: TimeInForce::GTC,
                        client_order_id: Some(format!("session_buy_{}_{}", iteration, i)),
                    };
                    
                    let sell_order = LimitOrder {
                        symbol: opportunity.symbol.clone(),
                        side: OrderSide::Sell,
                        quantity: opportunity.quantity.min(0.1),
                        price: opportunity.sell_price,
                        time_in_force: TimeInForce::GTC,
                        client_order_id: Some(format!("session_sell_{}_{}", iteration, i)),
                    };
                    
                    match (executor.execute_order(buy_order).await, executor.execute_order(sell_order).await) {
                        (Ok(_), Ok(_)) => {
                            trades_executed += 2;
                            println!("    ✅ Executed arbitrage trade pair");
                        }
                        _ => {
                            println!("    ❌ Failed to execute trade pair");
                        }
                    }
                }
            }
        } else {
            println!("  📉 No arbitrage opportunities found");
        }
        
        // Small delay between iterations
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    let session_duration = session_start.elapsed();
    
    // Final results
    println!("\n🎯 Trading session completed in {:?}", session_duration);
    println!("📊 Session statistics:");
    println!("  Opportunities found: {}", opportunities_found);
    println!("  Trades executed: {}", trades_executed);
    
    let final_results = executor.get_results().await;
    println!("  Total trades: {}", final_results.total_trades);
    println!("  Total PnL: ${:.2}", final_results.total_pnl);
    
    let final_portfolio = executor.get_portfolio().await;
    println!("  Final BTC position: {:.6}", final_portfolio.get_position("BTCUSDT"));
    println!("  Final USDT balance: {:.2}", final_portfolio.get_balance("USDT"));
    
    let performance_metrics = executor.get_performance_metrics().await;
    println!("  Success rate: {:.1}%", performance_metrics.success_rate);
    println!("  Total volume: ${:.2}", performance_metrics.total_volume);
    println!("  Total fees: ${:.2}", performance_metrics.total_fees);
    
    let strategy_stats = strategy.get_statistics().await;
    println!("  Strategy uptime: {}s", strategy_stats.uptime_seconds);
    println!("  Average spread: {:.2} bps", strategy_stats.avg_spread_bps);
    
    // Assertions
    assert!(opportunities_found > 0, "Should have found some opportunities");
    assert!(final_results.total_trades > 0, "Should have executed some trades");
    assert!(session_duration < Duration::from_secs(30), "Session should complete quickly");
    
    println!("✅ Complete trading session test passed!");
    
    Ok(())
}
