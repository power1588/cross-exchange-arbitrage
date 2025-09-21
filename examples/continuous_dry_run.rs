//! Continuous dry-run testing for arbitrage strategy

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    strategy::ArbitrageStrategy,
    trading::DryRunExecutor,
    Result,
};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    println!("🚀 Starting Continuous Dry-Run Arbitrage Testing");
    println!("================================================");

    // Load configuration from file
    let config = ArbitrageConfig::from_file("config/arbitrage.toml")
        .unwrap_or_else(|_| {
            warn!("Could not load config file, using defaults");
            ArbitrageConfig::default()
        });

    // Display configuration
    println!("📊 Configuration:");
    println!("  Symbol: {}", config.strategy.symbol);
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Max Position: {}", config.strategy.max_position_size);
    println!("  Slippage Tolerance: {:.2}%", config.execution.slippage_tolerance * 100.0);
    println!("  Maker Fee: {:.2}%", config.execution.maker_fee * 100.0);
    println!("  Taker Fee: {:.2}%", config.execution.taker_fee * 100.0);

    // Create strategy and executor
    let mut strategy = ArbitrageStrategy::new(config.clone()).await?;
    let mut executor = DryRunExecutor::new(config.clone()).await?;

    println!("\n💰 Initial Portfolio:");
    let initial_portfolio = executor.get_portfolio().await;
    println!("  BTC Position: {:.6}", initial_portfolio.get_position("BTCUSDT"));
    println!("  USDT Balance: ${:.2}", initial_portfolio.get_balance("USDT"));

    // Statistics tracking
    let mut total_opportunities = 0u64;
    let mut total_executed = 0u64;
    let mut iteration_count = 0u64;
    let start_time = std::time::Instant::now();

    // Set up interval for market updates (every 5 seconds)
    let mut market_update_interval = interval(Duration::from_secs(5));
    
    // Set up interval for status reports (every 30 seconds)
    let mut status_report_interval = interval(Duration::from_secs(30));

    println!("\n📈 Starting continuous market simulation...");
    println!("Press Ctrl+C to stop\n");

    loop {
        tokio::select! {
            _ = market_update_interval.tick() => {
                iteration_count += 1;
                
                // Simulate market data update with varying conditions
                let opportunities = simulate_market_opportunities(iteration_count).await;
                total_opportunities += opportunities.len() as u64;

                if !opportunities.is_empty() {
                    info!("💡 Iteration {}: Found {} arbitrage opportunities", iteration_count, opportunities.len());
                    
                    for (i, opportunity) in opportunities.iter().enumerate() {
                        info!("  {}. Buy {} @ ${:.2} on {} → Sell @ ${:.2} on {} (Spread: {:.2} bps, Profit: ${:.2})",
                              i + 1,
                              opportunity.quantity,
                              opportunity.buy_price,
                              opportunity.buy_exchange,
                              opportunity.sell_price,
                              opportunity.sell_exchange,
                              opportunity.spread_bps,
                              opportunity.expected_profit);

                        // Execute profitable opportunities
                        if opportunity.spread_bps >= config.strategy.min_spread_bps as f64 && opportunity.expected_profit > 1.0 {
                            let buy_order = cross_exchange_arbitrage::connectors::LimitOrder {
                                symbol: opportunity.symbol.clone(),
                                side: cross_exchange_arbitrage::connectors::OrderSide::Buy,
                                quantity: opportunity.quantity.min(config.strategy.max_position_size),
                                price: opportunity.buy_price,
                                time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                                client_order_id: Some(format!("continuous_buy_{}_{}", iteration_count, i)),
                            };

                            let sell_order = cross_exchange_arbitrage::connectors::LimitOrder {
                                symbol: opportunity.symbol.clone(),
                                side: cross_exchange_arbitrage::connectors::OrderSide::Sell,
                                quantity: opportunity.quantity.min(config.strategy.max_position_size),
                                price: opportunity.sell_price,
                                time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                                client_order_id: Some(format!("continuous_sell_{}_{}", iteration_count, i)),
                            };

                            match (executor.execute_order(buy_order).await, executor.execute_order(sell_order).await) {
                                (Ok(buy_response), Ok(sell_response)) => {
                                    total_executed += 2;
                                    info!("     ✅ Executed: Buy {} & Sell {}", buy_response.order_id, sell_response.order_id);
                                }
                                (Err(e), _) | (_, Err(e)) => {
                                    error!("     ❌ Failed to execute arbitrage: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    info!("📉 Iteration {}: No arbitrage opportunities", iteration_count);
                }
            }

            _ = status_report_interval.tick() => {
                // Generate status report
                let current_results = executor.get_results().await;
                let current_portfolio = executor.get_portfolio().await;
                let performance_metrics = executor.get_performance_metrics().await;
                let strategy_stats = strategy.get_statistics().await;
                
                let elapsed = start_time.elapsed();
                let uptime_hours = elapsed.as_secs_f64() / 3600.0;

                println!("\n📊 === STATUS REPORT (Uptime: {:.2}h) ===", uptime_hours);
                println!("🔄 Market Updates: {}", iteration_count);
                println!("💡 Opportunities: {} detected, {} executed", total_opportunities, total_executed / 2);
                println!("📈 Trading Stats:");
                println!("   Total Trades: {}", current_results.total_trades);
                println!("   Total PnL: ${:.2}", current_results.total_pnl);
                println!("   Success Rate: {:.1}%", performance_metrics.success_rate);
                println!("   Total Volume: ${:.2}", performance_metrics.total_volume);
                println!("   Total Fees: ${:.2}", performance_metrics.total_fees);
                
                println!("💰 Current Portfolio:");
                println!("   BTC Position: {:.6}", current_portfolio.get_position("BTCUSDT"));
                println!("   USDT Balance: ${:.2}", current_portfolio.get_balance("USDT"));
                
                if total_opportunities > 0 {
                    let execution_rate = (total_executed / 2) as f64 / total_opportunities as f64 * 100.0;
                    println!("📊 Execution Rate: {:.1}%", execution_rate);
                }
                
                println!("⏱️  Average Execution Time: {:.2}ms", performance_metrics.average_execution_time.as_millis());
                println!("==========================================\n");
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Received shutdown signal, stopping...");
                break;
            }
        }
    }

    // Final report
    let final_results = executor.get_results().await;
    let final_portfolio = executor.get_portfolio().await;
    let final_metrics = executor.get_performance_metrics().await;
    let elapsed = start_time.elapsed();

    println!("\n🎯 === FINAL REPORT ===");
    println!("⏱️  Total Runtime: {:.2} hours", elapsed.as_secs_f64() / 3600.0);
    println!("🔄 Market Updates: {}", iteration_count);
    println!("💡 Total Opportunities: {} detected, {} executed", total_opportunities, total_executed / 2);
    println!("📈 Final Trading Stats:");
    println!("   Total Trades: {}", final_results.total_trades);
    println!("   Total PnL: ${:.2}", final_results.total_pnl);
    println!("   Success Rate: {:.1}%", final_metrics.success_rate);
    println!("   Total Volume: ${:.2}", final_metrics.total_volume);
    println!("   Total Fees: ${:.2}", final_metrics.total_fees);
    
    println!("💰 Final Portfolio:");
    println!("   BTC Position: {:.6}", final_portfolio.get_position("BTCUSDT"));
    println!("   USDT Balance: ${:.2}", final_portfolio.get_balance("USDT"));
    
    if total_opportunities > 0 {
        let execution_rate = (total_executed / 2) as f64 / total_opportunities as f64 * 100.0;
        println!("📊 Overall Execution Rate: {:.1}%", execution_rate);
    }

    println!("✅ Continuous dry-run testing completed successfully!");
    
    Ok(())
}

/// Simulate market opportunities with varying conditions
async fn simulate_market_opportunities(iteration: u64) -> Vec<cross_exchange_arbitrage::strategy::ArbitrageOpportunity> {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    let mut opportunities = Vec::new();
    
    // Base BTC price with some volatility
    let base_price = 50000.0 + (iteration as f64 * 5.0) + rng.gen_range(-500.0..500.0);
    
    // Generate opportunities based on different market conditions
    match iteration % 10 {
        // High volatility periods - more opportunities
        0 | 1 | 2 => {
            if rng.gen::<f64>() < 0.7 {  // 70% chance
                let spread_bps = rng.gen_range(5.0..15.0);
                let spread_amount = base_price * spread_bps / 10000.0;
                
                opportunities.push(cross_exchange_arbitrage::strategy::ArbitrageOpportunity {
                    symbol: "BTCUSDT".to_string(),
                    buy_exchange: cross_exchange_arbitrage::connectors::Exchange::Binance,
                    sell_exchange: cross_exchange_arbitrage::connectors::Exchange::Bybit,
                    buy_price: base_price - spread_amount / 2.0,
                    sell_price: base_price + spread_amount / 2.0,
                    quantity: rng.gen_range(0.01..0.1),
                    spread_bps,
                    expected_profit: spread_amount * rng.gen_range(0.01..0.1),
                    timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                });
            }
        }
        
        // Normal market conditions - fewer opportunities
        3 | 4 | 5 | 6 => {
            if rng.gen::<f64>() < 0.3 {  // 30% chance
                let spread_bps = rng.gen_range(3.0..8.0);
                let spread_amount = base_price * spread_bps / 10000.0;
                
                if spread_bps >= 5.0 {  // Only profitable ones
                    opportunities.push(cross_exchange_arbitrage::strategy::ArbitrageOpportunity {
                        symbol: "BTCUSDT".to_string(),
                        buy_exchange: cross_exchange_arbitrage::connectors::Exchange::Bybit,
                        sell_exchange: cross_exchange_arbitrage::connectors::Exchange::Binance,
                        buy_price: base_price - spread_amount / 2.0,
                        sell_price: base_price + spread_amount / 2.0,
                        quantity: rng.gen_range(0.005..0.05),
                        spread_bps,
                        expected_profit: spread_amount * rng.gen_range(0.005..0.05),
                        timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    });
                }
            }
        }
        
        // Low volatility periods - rare opportunities
        _ => {
            if rng.gen::<f64>() < 0.1 {  // 10% chance
                let spread_bps = rng.gen_range(2.0..6.0);
                let spread_amount = base_price * spread_bps / 10000.0;
                
                if spread_bps >= 5.0 {
                    opportunities.push(cross_exchange_arbitrage::strategy::ArbitrageOpportunity {
                        symbol: "BTCUSDT".to_string(),
                        buy_exchange: cross_exchange_arbitrage::connectors::Exchange::Binance,
                        sell_exchange: cross_exchange_arbitrage::connectors::Exchange::Bybit,
                        buy_price: base_price - spread_amount / 2.0,
                        sell_price: base_price + spread_amount / 2.0,
                        quantity: rng.gen_range(0.001..0.02),
                        spread_bps,
                        expected_profit: spread_amount * rng.gen_range(0.001..0.02),
                        timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    });
                }
            }
        }
    }
    
    opportunities
}
