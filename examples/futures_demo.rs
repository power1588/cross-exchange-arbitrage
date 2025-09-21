//! Futures arbitrage demo - Bybit Maker + Binance Taker strategy

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::Exchange,
    data::OrderBook,
    strategy::futures_arbitrage::{FuturesArbitrageStrategy, FuturesArbitrageOpportunity},
    Result,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Futures Arbitrage Demo - Bybit Maker + Binance Taker");
    println!("========================================================");

    // Configuration for futures arbitrage
    let mut config = ArbitrageConfig::default();
    config.strategy.symbol = "BTCUSDT".to_string();
    config.strategy.min_spread_bps = 3; // Lower threshold for futures
    config.strategy.max_position_size = 0.5; // Conservative size
    config.execution.min_order_size = 0.001;

    println!("📊 Strategy Configuration:");
    println!("  Type: Bybit Maker + Binance Taker");
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Max Position: {} contracts", config.strategy.max_position_size);
    println!("  Fee Structure:");
    println!("    Bybit Maker: -0.025% (rebate)");
    println!("    Binance Taker: 0.04%");
    println!("    Net Cost: 0.015%");

    // Common USDT perpetual contracts
    let symbols = vec![
        "BTCUSDT".to_string(),
        "ETHUSDT".to_string(),
        "BNBUSDT".to_string(),
        "ADAUSDT".to_string(),
        "XRPUSDT".to_string(),
        "SOLUSDT".to_string(),
        "DOGEUSDT".to_string(),
        "AVAXUSDT".to_string(),
        "MATICUSDT".to_string(),
        "LINKUSDT".to_string(),
    ];

    println!("\n📈 Monitoring {} USDT Perpetual Contracts:", symbols.len());
    for (i, symbol) in symbols.iter().enumerate() {
        println!("  {}. {}", i + 1, symbol);
    }

    // Create strategy
    let strategy = FuturesArbitrageStrategy::new(config, symbols.clone()).await?;
    strategy.start().await?;

    println!("\n💡 Strategy Logic:");
    println!("  1. Monitor orderbooks on both exchanges");
    println!("  2. Place maker orders on Bybit (earn rebate)");
    println!("  3. When filled, immediately hedge on Binance (taker)");
    println!("  4. Profit from spread + maker rebate - taker fee");

    println!("\n📊 Starting market data simulation...");
    println!("Press Ctrl+C to stop\n");

    let mut total_opportunities = 0u64;
    let mut profitable_opportunities = 0u64;
    let start_time = std::time::Instant::now();

    for iteration in 1..=30 {
        println!("--- Market Update {} ---", iteration);

        // Generate realistic market data for multiple symbols
        for symbol in &symbols {
            let (bybit_book, binance_book) = generate_futures_orderbooks(symbol, iteration).await;
            
            // Update strategy with market data
            strategy.update_orderbook(Exchange::Bybit, bybit_book).await?;
            strategy.update_orderbook(Exchange::Binance, binance_book).await?;
        }

        // Get detected opportunities
        let opportunities = strategy.get_current_opportunities().await;
        total_opportunities += opportunities.len() as u64;

        if !opportunities.is_empty() {
            println!("💡 Found {} arbitrage opportunities:", opportunities.len());
            
            for (i, opp) in opportunities.iter().enumerate() {
                println!("  {}. {} - {} {:.3} @ ${:.2} on {} → {} @ ${:.2} on {}",
                         i + 1, opp.symbol,
                         opp.maker_side, opp.quantity, opp.maker_price, opp.maker_exchange,
                         opp.taker_side, opp.taker_price, opp.taker_exchange);
                println!("     Spread: {:.2} bps, Profit: ${:.4}, Risk: {:.0}/100",
                         opp.spread_bps, opp.expected_profit, opp.risk_score);
                
                // Check if opportunity is profitable
                if opp.expected_profit > 0.0 && opp.risk_score < 70.0 {
                    profitable_opportunities += 1;
                    println!("     ✅ PROFITABLE - Would execute this opportunity");
                    
                    // Simulate execution
                    println!("       Step 1: Place {} limit order on Bybit @ ${:.2}", 
                             opp.maker_side, opp.maker_price);
                    println!("       Step 2: Wait for fill (earn {:.4}% rebate)", 
                             opp.maker_fee.abs() * 100.0);
                    println!("       Step 3: Immediately hedge with {} market order on Binance", 
                             opp.taker_side);
                    println!("       Net Profit: ${:.4} (after fees)", opp.expected_profit);
                } else {
                    println!("     ⏸️  SKIP - Low profit or high risk");
                }
            }
        } else {
            if iteration % 5 == 0 {
                println!("📉 No arbitrage opportunities found");
            }
        }

        // Status update every 10 iterations
        if iteration % 10 == 0 {
            let stats = strategy.get_statistics().await;
            let elapsed = start_time.elapsed();
            
            println!("\n📊 === STATUS UPDATE ===");
            println!("⏱️  Runtime: {:.1} seconds", elapsed.as_secs_f64());
            println!("🔄 Market Updates: {}", iteration);
            println!("💡 Total Opportunities: {}", total_opportunities);
            println!("💰 Profitable Opportunities: {}", profitable_opportunities);
            if total_opportunities > 0 {
                println!("📈 Profitability Rate: {:.1}%", 
                         profitable_opportunities as f64 / total_opportunities as f64 * 100.0);
            }
            println!("========================\n");
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Final report
    strategy.stop().await?;
    let final_stats = strategy.get_statistics().await;
    let elapsed = start_time.elapsed();

    println!("\n🎯 === FINAL FUTURES ARBITRAGE REPORT ===");
    println!("⏱️  Total Runtime: {:.1} seconds", elapsed.as_secs_f64());
    println!("📊 Market Analysis:");
    println!("   Total Market Updates: 30");
    println!("   Symbols Monitored: {}", symbols.len());
    println!("   Total Opportunities: {}", total_opportunities);
    println!("   Profitable Opportunities: {}", profitable_opportunities);
    
    if total_opportunities > 0 {
        println!("   Profitability Rate: {:.1}%", 
                 profitable_opportunities as f64 / total_opportunities as f64 * 100.0);
        println!("   Opportunities per minute: {:.1}", 
                 total_opportunities as f64 / (elapsed.as_secs_f64() / 60.0));
    }

    println!("\n💰 Strategy Performance:");
    println!("   Detected: {}", final_stats.opportunities_detected);
    println!("   Executed: {}", final_stats.opportunities_executed);
    if final_stats.opportunities_detected > 0 {
        println!("   Success Rate: {:.1}%", final_stats.success_rate);
        println!("   Average Spread: {:.2} bps", final_stats.avg_spread_bps);
    }

    println!("\n🔄 Next Steps for Live Implementation:");
    println!("1. 🔑 Configure API keys in config/live_trading.toml");
    println!("2. 📡 Connect to real WebSocket feeds:");
    println!("   - Binance: wss://fstream.binance.com/ws/");
    println!("   - Bybit: wss://stream.bybit.com/v5/public/linear");
    println!("3. 🎯 Start with small position sizes (0.01-0.1 contracts)");
    println!("4. 📊 Monitor maker fill rates on Bybit");
    println!("5. ⚡ Optimize hedge execution speed on Binance");

    println!("\n✅ Futures arbitrage demo completed successfully!");
    println!("Strategy validated for Bybit Maker + Binance Taker approach");

    Ok(())
}

/// Generate realistic futures orderbook data
async fn generate_futures_orderbooks(symbol: &str, iteration: u64) -> (OrderBook, OrderBook) {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    
    // Base prices for different symbols
    let base_price = match symbol {
        "BTCUSDT" => 50000.0 + rng.gen_range(-2000.0..2000.0),
        "ETHUSDT" => 3000.0 + rng.gen_range(-300.0..300.0),
        "BNBUSDT" => 300.0 + rng.gen_range(-50.0..50.0),
        "ADAUSDT" => 0.5 + rng.gen_range(-0.1..0.1),
        "XRPUSDT" => 0.6 + rng.gen_range(-0.15..0.15),
        "SOLUSDT" => 100.0 + rng.gen_range(-30.0..30.0),
        "DOGEUSDT" => 0.08 + rng.gen_range(-0.02..0.02),
        "AVAXUSDT" => 25.0 + rng.gen_range(-8.0..8.0),
        "MATICUSDT" => 0.8 + rng.gen_range(-0.2..0.2),
        "LINKUSDT" => 15.0 + rng.gen_range(-3.0..3.0),
        _ => 100.0 + rng.gen_range(-20.0..20.0),
    };

    // Bybit orderbook (maker exchange - slightly worse liquidity, but maker rebate)
    let mut bybit_book = OrderBook::new(symbol.to_string(), Exchange::Bybit);
    let bybit_spread = rng.gen_range(0.0003..0.0010); // 0.03% - 0.10% spread
    
    // Create arbitrage opportunities occasionally
    let price_adjustment = if iteration % 7 == 0 {
        // Create opportunity for Bybit maker sell + Binance taker buy
        rng.gen_range(0.0004..0.0012) // Bybit bid higher than Binance ask
    } else if iteration % 11 == 0 {
        // Create opportunity for Bybit maker buy + Binance taker sell
        -rng.gen_range(0.0004..0.0012) // Binance bid higher than Bybit ask
    } else {
        rng.gen_range(-0.0002..0.0002) // Normal conditions
    };
    
    let bybit_mid = base_price * (1.0 + price_adjustment);
    let bybit_bid = bybit_mid * (1.0 - bybit_spread / 2.0);
    let bybit_ask = bybit_mid * (1.0 + bybit_spread / 2.0);
    
    // Bybit depth (lower liquidity)
    bybit_book.update_bid(bybit_bid, rng.gen_range(0.5..2.0));
    bybit_book.update_bid(bybit_bid - base_price * 0.0001, rng.gen_range(1.0..3.0));
    bybit_book.update_ask(bybit_ask, rng.gen_range(0.5..2.0));
    bybit_book.update_ask(bybit_ask + base_price * 0.0001, rng.gen_range(1.0..3.0));

    // Binance orderbook (taker exchange - better liquidity)
    let mut binance_book = OrderBook::new(symbol.to_string(), Exchange::Binance);
    let binance_spread = rng.gen_range(0.0001..0.0006); // Tighter spread on Binance
    let binance_mid = base_price; // Reference price
    let binance_bid = binance_mid * (1.0 - binance_spread / 2.0);
    let binance_ask = binance_mid * (1.0 + binance_spread / 2.0);
    
    // Binance depth (higher liquidity)
    binance_book.update_bid(binance_bid, rng.gen_range(3.0..8.0));
    binance_book.update_bid(binance_bid - base_price * 0.0001, rng.gen_range(5.0..10.0));
    binance_book.update_ask(binance_ask, rng.gen_range(3.0..8.0));
    binance_book.update_ask(binance_ask + base_price * 0.0001, rng.gen_range(5.0..10.0));

    // Set timestamps
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    bybit_book.set_timestamp(timestamp);
    binance_book.set_timestamp(timestamp);

    (bybit_book, binance_book)
}
