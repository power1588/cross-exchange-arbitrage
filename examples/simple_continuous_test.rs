//! Simple continuous dry-run test

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    trading::DryRunExecutor,
    Result,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Simple Continuous Dry-Run Test");
    println!("=================================");

    // Create configuration
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 5;
    config.execution.enable_fees = true;
    config.execution.maker_fee = 0.001; // 0.1%
    config.execution.taker_fee = 0.001; // 0.1%
    config.execution.slippage_tolerance = 0.001; // 0.1%

    println!("📊 Configuration:");
    println!("  Symbol: {}", config.strategy.symbol);
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Slippage: {:.2}%", config.execution.slippage_tolerance * 100.0);
    println!("  Maker Fee: {:.2}%", config.execution.maker_fee * 100.0);
    println!("  Taker Fee: {:.2}%", config.execution.taker_fee * 100.0);

    let mut executor = DryRunExecutor::new(config.clone()).await?;

    println!("\n💰 Initial Portfolio:");
    let initial_portfolio = executor.get_portfolio().await;
    println!("  USDT Balance: ${:.2}", initial_portfolio.get_balance("USDT"));

    println!("\n📈 Running 20 iterations (every 2 seconds)...");
    println!("Press Ctrl+C to stop early\n");

    let mut total_opportunities = 0;
    let mut total_executed = 0;

    for iteration in 1..=20 {
        println!("--- Iteration {} ---", iteration);

        // Simulate some trading opportunities
        if iteration % 3 == 0 {
            // Create a profitable opportunity
            let buy_order = cross_exchange_arbitrage::connectors::LimitOrder {
                symbol: "BTCUSDT".to_string(),
                side: cross_exchange_arbitrage::connectors::OrderSide::Buy,
                quantity: 0.01,
                price: 50000.0,
                time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                client_order_id: Some(format!("test_buy_{}", iteration)),
            };

            let sell_order = cross_exchange_arbitrage::connectors::LimitOrder {
                symbol: "BTCUSDT".to_string(),
                side: cross_exchange_arbitrage::connectors::OrderSide::Sell,
                quantity: 0.01,
                price: 50035.0,
                time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                client_order_id: Some(format!("test_sell_{}", iteration)),
            };

            total_opportunities += 1;
            println!("💡 Found arbitrage opportunity: Buy @$50000, Sell @$50035 (7 bps spread)");

            match (executor.execute_order(buy_order).await, executor.execute_order(sell_order).await) {
                (Ok(buy_response), Ok(sell_response)) => {
                    total_executed += 1;
                    println!("✅ Executed: Buy {} & Sell {}", 
                            buy_response.order_id, sell_response.order_id);
                }
                _ => {
                    println!("❌ Failed to execute arbitrage");
                }
            }
        } else {
            println!("📉 No arbitrage opportunities");
        }

        // Show current status every 5 iterations
        if iteration % 5 == 0 {
            let results = executor.get_results().await;
            let portfolio = executor.get_portfolio().await;
            let metrics = executor.get_performance_metrics().await;

            println!("\n📊 Status Update:");
            println!("  Opportunities: {} found, {} executed", total_opportunities, total_executed);
            println!("  Total Trades: {}", results.total_trades);
            println!("  Total PnL: ${:.2}", results.total_pnl);
            println!("  USDT Balance: ${:.2}", portfolio.get_balance("USDT"));
            println!("  Success Rate: {:.1}%", metrics.success_rate);
            println!("  Total Fees: ${:.2}", metrics.total_fees);
            println!();
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Final report
    let final_results = executor.get_results().await;
    let final_portfolio = executor.get_portfolio().await;
    let final_metrics = executor.get_performance_metrics().await;

    println!("\n🎯 Final Report:");
    println!("================");
    println!("📊 Trading Statistics:");
    println!("  Opportunities: {} found, {} executed", total_opportunities, total_executed);
    println!("  Total Trades: {}", final_results.total_trades);
    println!("  Total PnL: ${:.2}", final_results.total_pnl);
    println!("  Success Rate: {:.1}%", final_metrics.success_rate);
    println!("  Total Volume: ${:.2}", final_metrics.total_volume);
    println!("  Total Fees: ${:.2}", final_metrics.total_fees);

    println!("\n💰 Final Portfolio:");
    println!("  BTC Position: {:.6}", final_portfolio.get_position("BTCUSDT"));
    println!("  USDT Balance: ${:.2}", final_portfolio.get_balance("USDT"));

    let profit_loss = final_portfolio.get_balance("USDT") - 100000.0;
    println!("  Net P&L: ${:.2}", profit_loss);

    if profit_loss > 0.0 {
        println!("✅ Profitable session!");
    } else {
        println!("📉 Loss session (including fees and slippage)");
    }

    println!("\n✅ Continuous dry-run test completed!");

    Ok(())
}
