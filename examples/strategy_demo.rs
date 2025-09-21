//! Demonstration of the arbitrage strategy with dry-run executor

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    strategy::ArbitrageStrategy,
    trading::DryRunExecutor,
    Result,
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Skip logger initialization for simplicity
    
    println!("🚀 Cross-Exchange Arbitrage Strategy Demo");
    println!("==========================================");
    
    // Create configuration
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 5; // 0.05% minimum spread
    config.strategy.max_position_size = 1.0; // 1 BTC max
    config.execution.min_order_size = 0.001; // 0.001 BTC minimum
    config.execution.slippage_tolerance = 0.001; // 0.1% slippage
    config.execution.enable_fees = true;
    config.execution.maker_fee = 0.001; // 0.1% maker fee
    config.execution.taker_fee = 0.001; // 0.1% taker fee
    
    println!("📊 Strategy Configuration:");
    println!("  Symbol: {}", config.strategy.symbol);
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Max Position: {} BTC", config.strategy.max_position_size);
    println!("  Min Order Size: {} BTC", config.execution.min_order_size);
    println!("  Slippage Tolerance: {:.1}%", config.execution.slippage_tolerance * 100.0);
    println!("  Fees Enabled: {}", config.execution.enable_fees);
    
    // Create strategy and executor
    let mut strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config).await?;
    
    println!("\n💰 Initial Portfolio:");
    let initial_portfolio = executor.get_portfolio().await;
    println!("  BTC Position: {:.6}", initial_portfolio.get_position("BTCUSDT"));
    println!("  USDT Balance: ${:.2}", initial_portfolio.get_balance("USDT"));
    
    // Simulate market data updates and opportunity detection
    println!("\n📈 Starting Market Data Simulation...");
    
    let mut total_opportunities = 0;
    let mut executed_trades = 0;
    
    for iteration in 1..=10 {
        println!("\n--- Market Update {} ---", iteration);
        
        // For demonstration, we'll create a simple simulation
        // In a real implementation, market data would come from live feeds
        info!("Processing market data for iteration {}", iteration);
        
        // Simulate some opportunities (this is just for demo purposes)
        let opportunities = if iteration % 3 == 0 {
            // Create a mock opportunity every 3rd iteration
            vec![cross_exchange_arbitrage::strategy::ArbitrageOpportunity {
                symbol: "BTCUSDT".to_string(),
                buy_exchange: cross_exchange_arbitrage::connectors::Exchange::Binance,
                sell_exchange: cross_exchange_arbitrage::connectors::Exchange::Bybit,
                buy_price: 50000.0,
                sell_price: 50030.0,
                quantity: 0.05,
                spread_bps: 6.0,
                expected_profit: 1.5,
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            }]
        } else {
            vec![]
        };
        total_opportunities += opportunities.len();
        
        if !opportunities.is_empty() {
            println!("💡 Found {} arbitrage opportunities:", opportunities.len());
            
            for (i, opportunity) in opportunities.iter().enumerate() {
                println!("  {}. Buy {} @ ${:.2} on {} → Sell @ ${:.2} on {}",
                        i + 1,
                        opportunity.quantity,
                        opportunity.buy_price,
                        opportunity.buy_exchange,
                        opportunity.sell_price,
                        opportunity.sell_exchange);
                println!("     Spread: {:.2} bps, Expected Profit: ${:.2}",
                        opportunity.spread_bps,
                        opportunity.expected_profit);
                
                // Execute profitable opportunities
                if opportunity.spread_bps >= 5.0 && opportunity.expected_profit > 1.0 {
                    // Create buy order
                    let buy_order = cross_exchange_arbitrage::connectors::LimitOrder {
                        symbol: opportunity.symbol.clone(),
                        side: cross_exchange_arbitrage::connectors::OrderSide::Buy,
                        quantity: opportunity.quantity.min(0.1), // Limit size for demo
                        price: opportunity.buy_price,
                        time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                        client_order_id: Some(format!("demo_buy_{}_{}", iteration, i)),
                    };
                    
                    // Create sell order
                    let sell_order = cross_exchange_arbitrage::connectors::LimitOrder {
                        symbol: opportunity.symbol.clone(),
                        side: cross_exchange_arbitrage::connectors::OrderSide::Sell,
                        quantity: opportunity.quantity.min(0.1),
                        price: opportunity.sell_price,
                        time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                        client_order_id: Some(format!("demo_sell_{}_{}", iteration, i)),
                    };
                    
                    // Execute orders
                    match (executor.execute_order(buy_order).await, executor.execute_order(sell_order).await) {
                        (Ok(buy_response), Ok(sell_response)) => {
                            executed_trades += 2;
                            println!("     ✅ Executed: Buy Order {} & Sell Order {}", 
                                    buy_response.order_id, sell_response.order_id);
                        }
                        _ => {
                            warn!("Failed to execute arbitrage opportunity");
                        }
                    }
                }
            }
        } else {
            println!("📉 No arbitrage opportunities found");
        }
        
        // Small delay between iterations
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    
    // Final results
    println!("\n🎯 Demo Results Summary");
    println!("=======================");
    
    let final_results = executor.get_results().await;
    println!("📊 Execution Statistics:");
    println!("  Total Opportunities Detected: {}", total_opportunities);
    println!("  Total Trades Executed: {}", final_results.total_trades);
    println!("  Total PnL: ${:.2}", final_results.total_pnl);
    
    let final_portfolio = executor.get_portfolio().await;
    println!("\n💰 Final Portfolio:");
    println!("  BTC Position: {:.6}", final_portfolio.get_position("BTCUSDT"));
    println!("  USDT Balance: ${:.2}", final_portfolio.get_balance("USDT"));
    
    let performance_metrics = executor.get_performance_metrics().await;
    println!("\n📈 Performance Metrics:");
    println!("  Success Rate: {:.1}%", performance_metrics.success_rate);
    println!("  Total Volume: ${:.2}", performance_metrics.total_volume);
    println!("  Total Fees: ${:.2}", performance_metrics.total_fees);
    println!("  Average Execution Time: {:.2}ms", performance_metrics.average_execution_time.as_millis());
    
    let strategy_stats = strategy.get_statistics().await;
    println!("\n🔍 Strategy Statistics:");
    println!("  Opportunities Detected: {}", strategy_stats.opportunities_detected);
    println!("  Opportunities Executed: {}", strategy_stats.opportunities_executed);
    println!("  Strategy Uptime: {}s", strategy_stats.uptime_seconds);
    if strategy_stats.opportunities_executed > 0 {
        println!("  Average Spread Captured: {:.2} bps", strategy_stats.avg_spread_bps);
        println!("  Success Rate: {:.1}%", strategy_stats.success_rate);
    }
    
    println!("\n✅ Demo completed successfully!");
    println!("This demonstrates the dry-run arbitrage strategy with simulated market data.");
    println!("In a real implementation, market data would come from live WebSocket feeds.");
    
    Ok(())
}
