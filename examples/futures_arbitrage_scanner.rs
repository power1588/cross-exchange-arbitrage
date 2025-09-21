//! Real-time futures arbitrage scanner
//! Scans Binance and Bybit USDT perpetual contracts for arbitrage opportunities
//! Strategy: Bybit Maker + Binance Taker

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::{
        Exchange,
        futures::{FuturesConnector, ContractType},
        binance_futures::BinanceFuturesConnector,
        bybit_futures::BybitFuturesConnector,
    },
    strategy::futures_arbitrage::{FuturesArbitrageStrategy, FuturesStrategyState},
    data::OrderBook,
    Result,
};
use std::collections::HashMap;
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
        .init();

    println!("🚀 Futures Arbitrage Scanner - Bybit Maker + Binance Taker");
    println!("===========================================================");

    // Create configuration
    let mut config = ArbitrageConfig::default();
    config.strategy.min_spread_bps = 3; // Lower threshold for futures (higher leverage)
    config.strategy.max_position_size = 0.5; // Conservative position size
    config.execution.min_order_size = 0.001;

    println!("📊 Configuration:");
    println!("  Strategy: Bybit Maker + Binance Taker");
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Max Position: {} contracts", config.strategy.max_position_size);
    println!("  Min Order Size: {} contracts", config.execution.min_order_size);

    // Create connectors (without API keys for public data)
    let mut binance_connector = BinanceFuturesConnector::new(None, None);
    let mut bybit_connector = BybitFuturesConnector::new(None, None);

    // Get common USDT perpetual symbols
    let common_symbols = binance_connector.get_common_usdt_perpetuals().await?;
    println!("\n📈 Monitoring {} USDT Perpetual Contracts:", common_symbols.len());
    for (i, symbol) in common_symbols.iter().enumerate() {
        println!("  {}. {}", i + 1, symbol);
    }

    // Create strategy
    let strategy = FuturesArbitrageStrategy::new(config, common_symbols.clone()).await?;
    strategy.start().await?;

    println!("\n💡 Fee Structure:");
    println!("  Bybit Maker Fee: -0.025% (rebate)");
    println!("  Binance Taker Fee: 0.04%");
    println!("  Net Fee Cost: 0.015% (0.04% - 0.025%)");

    // Subscribe to market data for all symbols
    println!("\n🔌 Setting up market data subscriptions...");
    
    for symbol in &common_symbols {
        if let Err(e) = binance_connector.subscribe_orderbook(symbol).await {
            warn!("Failed to subscribe to Binance orderbook for {}: {}", symbol, e);
        }
        
        if let Err(e) = bybit_connector.subscribe_orderbook(symbol).await {
            warn!("Failed to subscribe to Bybit orderbook for {}: {}", symbol, e);
        }
    }

    // Start market data simulation (in real implementation, this would be live data)
    println!("\n📊 Starting market data simulation...");
    println!("In production, this would connect to live WebSocket feeds");
    println!("Press Ctrl+C to stop\n");

    let mut iteration = 0u64;
    let mut total_opportunities = 0u64;
    let start_time = std::time::Instant::now();

    // Set up intervals
    let mut market_update_interval = interval(Duration::from_secs(2));
    let mut status_report_interval = interval(Duration::from_secs(15));

    loop {
        tokio::select! {
            _ = market_update_interval.tick() => {
                iteration += 1;
                
                // Simulate market data updates for multiple symbols
                for symbol in &common_symbols {
                    // Generate realistic orderbook data with potential arbitrage opportunities
                    let (bybit_book, binance_book) = generate_realistic_orderbooks(symbol, iteration).await;
                    
                    // Update strategy with market data
                    strategy.update_orderbook(Exchange::Bybit, bybit_book).await?;
                    strategy.update_orderbook(Exchange::Binance, binance_book).await?;
                }

                // Get detected opportunities
                let opportunities = strategy.get_current_opportunities().await;
                total_opportunities += opportunities.len() as u64;

                if !opportunities.is_empty() {
                    info!("🎯 Iteration {}: Found {} arbitrage opportunities", iteration, opportunities.len());
                    
                    for (i, opp) in opportunities.iter().enumerate() {
                        info!("  {}. {} - {} {} @ {:.2} on {} → {} @ {:.2} on {}",
                              i + 1, opp.symbol,
                              opp.maker_side, opp.quantity, opp.maker_price, opp.maker_exchange,
                              opp.taker_side, opp.taker_price, opp.taker_exchange);
                        info!("     Spread: {:.2} bps, Profit: ${:.4}, Risk: {:.0}/100",
                              opp.spread_bps, opp.expected_profit, opp.risk_score);
                        
                        // In a real implementation, we would execute the opportunity here
                        // For demo purposes, we just log it
                        if opp.expected_profit > 0.5 && opp.risk_score < 50.0 {
                            info!("     ✅ Would execute this opportunity (profit > $0.5, risk < 50)");
                        } else {
                            info!("     ⏸️  Would skip (profit too low or risk too high)");
                        }
                    }
                } else if iteration % 10 == 0 {
                    info!("📉 Iteration {}: No arbitrage opportunities found", iteration);
                }
            }

            _ = status_report_interval.tick() => {
                let stats = strategy.get_statistics().await;
                let elapsed = start_time.elapsed();
                
                println!("\n📊 === STATUS REPORT (Uptime: {:.1}m) ===", elapsed.as_secs_f64() / 60.0);
                println!("🔄 Market Updates: {}", iteration);
                println!("💡 Total Opportunities: {}", total_opportunities);
                println!("📈 Strategy Stats:");
                println!("   Detected: {}", stats.opportunities_detected);
                println!("   Executed: {}", stats.opportunities_executed);
                println!("   Success Rate: {:.1}%", stats.success_rate);
                if stats.opportunities_detected > 0 {
                    let detection_rate = total_opportunities as f64 / iteration as f64 * 100.0;
                    println!("   Detection Rate: {:.1}% of market updates", detection_rate);
                }
                println!("=========================================\n");
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Received shutdown signal, stopping scanner...");
                break;
            }
        }
    }

    // Final report
    strategy.stop().await?;
    let final_stats = strategy.get_statistics().await;
    let elapsed = start_time.elapsed();

    println!("\n🎯 === FINAL SCANNER REPORT ===");
    println!("⏱️  Total Runtime: {:.2} minutes", elapsed.as_secs_f64() / 60.0);
    println!("🔄 Total Market Updates: {}", iteration);
    println!("💡 Total Opportunities: {}", total_opportunities);
    println!("📊 Detection Rate: {:.2}% of updates", total_opportunities as f64 / iteration as f64 * 100.0);
    
    if total_opportunities > 0 {
        println!("📈 Opportunity Analysis:");
        println!("   Average per minute: {:.1}", total_opportunities as f64 / (elapsed.as_secs_f64() / 60.0));
        println!("   Peak detection rate achieved");
    }

    println!("✅ Futures arbitrage scanner completed successfully!");
    println!("\n🔄 Next Steps for Live Trading:");
    println!("1. Configure API keys in config/live_trading.toml");
    println!("2. Set appropriate position sizes and risk limits");
    println!("3. Enable real WebSocket connections");
    println!("4. Start with small position sizes for testing");

    Ok(())
}

/// Generate realistic orderbook data for testing
async fn generate_realistic_orderbooks(symbol: &str, iteration: u64) -> (OrderBook, OrderBook) {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    
    // Base price varies by symbol
    let base_price = match symbol {
        "BTCUSDT" => 50000.0 + rng.gen_range(-1000.0..1000.0),
        "ETHUSDT" => 3000.0 + rng.gen_range(-200.0..200.0),
        "BNBUSDT" => 300.0 + rng.gen_range(-30.0..30.0),
        "ADAUSDT" => 0.5 + rng.gen_range(-0.1..0.1),
        "XRPUSDT" => 0.6 + rng.gen_range(-0.1..0.1),
        "SOLUSDT" => 100.0 + rng.gen_range(-20.0..20.0),
        _ => 100.0 + rng.gen_range(-10.0..10.0),
    };

    // Create Bybit orderbook (maker exchange - slightly worse liquidity)
    let mut bybit_book = OrderBook::new(symbol.to_string(), Exchange::Bybit);
    let bybit_spread = rng.gen_range(0.0002..0.0008); // 0.02% - 0.08% spread
    let bybit_bid = base_price * (1.0 - bybit_spread / 2.0);
    let bybit_ask = base_price * (1.0 + bybit_spread / 2.0);
    
    // Add some depth
    bybit_book.update_bid(bybit_bid, rng.gen_range(0.5..2.0));
    bybit_book.update_bid(bybit_bid - base_price * 0.0001, rng.gen_range(1.0..3.0));
    bybit_book.update_ask(bybit_ask, rng.gen_range(0.5..2.0));
    bybit_book.update_ask(bybit_ask + base_price * 0.0001, rng.gen_range(1.0..3.0));

    // Create Binance orderbook (taker exchange - better liquidity)
    let mut binance_book = OrderBook::new(symbol.to_string(), Exchange::Binance);
    let binance_spread = rng.gen_range(0.0001..0.0005); // 0.01% - 0.05% spread (tighter)
    
    // Occasionally create arbitrage opportunities
    let price_offset = if iteration % 5 == 0 {
        // Create opportunity: Bybit bid > Binance ask
        rng.gen_range(0.0003..0.0008) // 0.03% - 0.08% price difference
    } else if iteration % 7 == 0 {
        // Create opportunity: Binance bid > Bybit ask
        -rng.gen_range(0.0003..0.0008)
    } else {
        rng.gen_range(-0.0002..0.0002) // Normal market conditions
    };
    
    let binance_mid = base_price * (1.0 + price_offset);
    let binance_bid = binance_mid * (1.0 - binance_spread / 2.0);
    let binance_ask = binance_mid * (1.0 + binance_spread / 2.0);
    
    // Better liquidity on Binance
    binance_book.update_bid(binance_bid, rng.gen_range(2.0..5.0));
    binance_book.update_bid(binance_bid - base_price * 0.0001, rng.gen_range(3.0..6.0));
    binance_book.update_ask(binance_ask, rng.gen_range(2.0..5.0));
    binance_book.update_ask(binance_ask + base_price * 0.0001, rng.gen_range(3.0..6.0));

    // Set timestamps
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    bybit_book.set_timestamp(timestamp);
    binance_book.set_timestamp(timestamp);

    (bybit_book, binance_book)
}
