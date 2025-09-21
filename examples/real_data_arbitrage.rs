//! Real-time arbitrage with actual market data from Binance and Bybit

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::Exchange,
    data::OrderBook,
    trading::DryRunExecutor,
    Result,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("🚀 REAL-TIME Arbitrage Scanner with Live Market Data");
    println!("===================================================");
    println!("📡 Connecting to ACTUAL exchange WebSocket feeds");
    println!("⚠️  This uses REAL market data from Binance and Bybit");
    println!();

    // Configuration
    let mut config = ArbitrageConfig::default();
    config.strategy.min_spread_bps = 3;
    config.strategy.max_position_size = 0.1; // Small size for testing
    config.execution.enable_fees = true;
    config.execution.maker_fee = -0.00025; // Bybit rebate
    config.execution.taker_fee = 0.0004;   // Binance fee

    // Create dry-run executor for testing
    let mut executor = DryRunExecutor::new(config.clone()).await?;

    println!("📊 Configuration:");
    println!("  Strategy: Real-time arbitrage with live data");
    println!("  Min Spread: {} bps", config.strategy.min_spread_bps);
    println!("  Bybit Maker Fee: {:.3}% (rebate)", config.execution.maker_fee * 100.0);
    println!("  Binance Taker Fee: {:.3}%", config.execution.taker_fee * 100.0);

    // Symbols to monitor
    let symbols = vec!["BTCUSDT", "ETHUSDT", "BNBUSDT"];
    println!("\n📈 Monitoring {} symbols with REAL data:", symbols.len());
    for symbol in &symbols {
        println!("  - {}", symbol);
    }

    // Market data storage
    let mut binance_books: HashMap<String, OrderBook> = HashMap::new();
    let mut bybit_books: HashMap<String, OrderBook> = HashMap::new();
    let mut total_opportunities = 0u64;
    let mut total_executed = 0u64;

    println!("\n🔗 Connecting to live WebSocket feeds...");

    // Connect to Binance futures WebSocket
    let binance_streams: Vec<String> = symbols.iter()
        .map(|s| format!("{}@depth20@100ms", s.to_lowercase()))
        .collect();
    let binance_url = format!("wss://fstream.binance.com/stream?streams={}", binance_streams.join("/"));
    
    println!("📡 Binance URL: {}", binance_url);

    let (mut binance_ws, _) = match timeout(Duration::from_secs(10), connect_async(&binance_url)).await {
        Ok(Ok(connection)) => {
            println!("✅ Connected to Binance futures WebSocket");
            connection
        }
        Ok(Err(e)) => {
            error!("❌ Failed to connect to Binance: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            error!("❌ Binance connection timeout");
            return Err(anyhow::anyhow!("Connection timeout"));
        }
    };

    // Connect to Bybit futures WebSocket
    let bybit_url = "wss://stream.bybit.com/v5/public/linear";
    println!("📡 Bybit URL: {}", bybit_url);

    let (mut bybit_ws, _) = match timeout(Duration::from_secs(10), connect_async(bybit_url)).await {
        Ok(Ok(connection)) => {
            println!("✅ Connected to Bybit futures WebSocket");
            connection
        }
        Ok(Err(e)) => {
            error!("❌ Failed to connect to Bybit: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            error!("❌ Bybit connection timeout");
            return Err(anyhow::anyhow!("Connection timeout"));
        }
    };

    // Subscribe to Bybit data
    let bybit_subscription = create_bybit_subscription(&symbols);
    println!("📤 Sending Bybit subscription...");
    
    if let Err(e) = bybit_ws.send(Message::Text(bybit_subscription)).await {
        error!("Failed to send Bybit subscription: {}", e);
        return Err(e.into());
    }

    println!("✅ Subscriptions sent successfully");
    println!("\n📊 Starting real-time arbitrage scanning...");
    println!("🔄 Processing live market data...\n");

    let start_time = std::time::Instant::now();
    let mut last_report = std::time::Instant::now();

    // Main event loop
    loop {
        tokio::select! {
            // Process Binance messages
            binance_msg = binance_ws.next() => {
                match binance_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Err(e) = process_binance_depth(&data, &mut binance_books).await {
                                warn!("Failed to process Binance data: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        warn!("Binance connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("Binance WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("Binance stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Process Bybit messages
            bybit_msg = bybit_ws.next() => {
                match bybit_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Err(e) = process_bybit_depth(&data, &mut bybit_books).await {
                                warn!("Failed to process Bybit data: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        warn!("Bybit connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("Bybit WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("Bybit stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Periodic arbitrage scanning
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Scan for arbitrage opportunities using real data
                let opportunities = scan_arbitrage_opportunities(&binance_books, &bybit_books, &config).await;
                
                if !opportunities.is_empty() {
                    total_opportunities += opportunities.len() as u64;
                    
                    info!("🎯 REAL-TIME ARBITRAGE OPPORTUNITIES: {}", opportunities.len());
                    
                    for (i, opp) in opportunities.iter().enumerate() {
                        info!("  {}. {} - Spread: {:.2} bps, Profit: ${:.4}",
                              i + 1, opp.0, opp.1, opp.2);
                        
                        // Execute in dry-run mode with real prices
                        if opp.2 > 0.5 { // Profit > $0.50
                            let buy_order = cross_exchange_arbitrage::connectors::LimitOrder {
                                symbol: opp.0.clone(),
                                side: cross_exchange_arbitrage::connectors::OrderSide::Buy,
                                quantity: 0.01,
                                price: opp.3, // Real price from market data
                                time_in_force: cross_exchange_arbitrage::connectors::TimeInForce::GTC,
                                client_order_id: Some(format!("real_arb_{}", chrono::Utc::now().timestamp_millis())),
                            };

                            match executor.execute_order(buy_order).await {
                                Ok(response) => {
                                    total_executed += 1;
                                    info!("     ✅ Executed dry-run order: {}", response.order_id);
                                }
                                Err(e) => {
                                    warn!("     ❌ Failed to execute: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Status reports every 30 seconds
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if last_report.elapsed() >= Duration::from_secs(30) {
                    let elapsed = start_time.elapsed();
                    let results = executor.get_results().await;
                    
                    println!("\n📊 === LIVE DATA STATUS REPORT ===");
                    println!("⏱️  Runtime: {:.1} minutes", elapsed.as_secs_f64() / 60.0);
                    println!("📡 Data Source: REAL WebSocket feeds");
                    println!("🔄 Binance Books: {}", binance_books.len());
                    println!("🔄 Bybit Books: {}", bybit_books.len());
                    println!("💡 Opportunities Found: {}", total_opportunities);
                    println!("🎯 Trades Executed (Dry-run): {}", total_executed);
                    println!("💰 Total PnL (Dry-run): ${:.2}", results.total_pnl);
                    
                    if total_opportunities > 0 {
                        println!("📈 Execution Rate: {:.1}%", 
                                 total_executed as f64 / total_opportunities as f64 * 100.0);
                    }
                    
                    println!("===================================\n");
                    last_report = std::time::Instant::now();
                }
            }

            // Graceful shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Shutdown signal received");
                break;
            }
        }
    }

    // Final report
    let final_results = executor.get_results().await;
    let elapsed = start_time.elapsed();

    println!("\n🎯 === FINAL REAL DATA REPORT ===");
    println!("⏱️  Total Runtime: {:.2} minutes", elapsed.as_secs_f64() / 60.0);
    println!("📡 Data Source: REAL exchange WebSockets");
    println!("💡 Total Opportunities: {}", total_opportunities);
    println!("🎯 Total Executions: {}", total_executed);
    println!("💰 Total PnL: ${:.2}", final_results.total_pnl);
    
    if total_opportunities > 0 {
        println!("📈 Success Rate: {:.1}%", 
                 total_executed as f64 / total_opportunities as f64 * 100.0);
        println!("🔄 Opportunity Rate: {:.1} per minute", 
                 total_opportunities as f64 / (elapsed.as_secs_f64() / 60.0));
    }

    println!("✅ Real-time arbitrage scanning completed!");

    Ok(())
}

/// Create Bybit subscription message
fn create_bybit_subscription(symbols: &[&str]) -> String {
    let topics: Vec<String> = symbols.iter()
        .map(|symbol| format!("orderbook.50.{}", symbol))
        .collect();
    
    serde_json::json!({
        "op": "subscribe",
        "args": topics
    }).to_string()
}

/// Process Binance depth data
async fn process_binance_depth(data: &Value, books: &mut HashMap<String, OrderBook>) -> Result<()> {
    if let Some(stream) = data.get("stream").and_then(|s| s.as_str()) {
        if stream.contains("@depth") {
            if let Some(depth_data) = data.get("data") {
                if let Some(symbol) = depth_data.get("s").and_then(|s| s.as_str()) {
                    let mut orderbook = OrderBook::new(symbol.to_string(), Exchange::Binance);
                    
                    // Process bids
                    if let Some(bids) = depth_data.get("b").and_then(|b| b.as_array()) {
                        for bid in bids.iter().take(5) { // Top 5 levels
                            if let Some(bid_array) = bid.as_array() {
                                if bid_array.len() >= 2 {
                                    if let (Some(price_str), Some(qty_str)) = (bid_array[0].as_str(), bid_array[1].as_str()) {
                                        if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                                            orderbook.update_bid(price, qty);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Process asks
                    if let Some(asks) = depth_data.get("a").and_then(|a| a.as_array()) {
                        for ask in asks.iter().take(5) { // Top 5 levels
                            if let Some(ask_array) = ask.as_array() {
                                if ask_array.len() >= 2 {
                                    if let (Some(price_str), Some(qty_str)) = (ask_array[0].as_str(), ask_array[1].as_str()) {
                                        if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                                            orderbook.update_ask(price, qty);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    orderbook.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                    books.insert(symbol.to_string(), orderbook);
                    
                    info!("📊 Binance {} - Bid: {:?}, Ask: {:?}", 
                          symbol, 
                          books[symbol].best_bid(), 
                          books[symbol].best_ask());
                }
            }
        }
    }
    Ok(())
}

/// Process Bybit depth data
async fn process_bybit_depth(data: &Value, books: &mut HashMap<String, OrderBook>) -> Result<()> {
    if let Some(topic) = data.get("topic").and_then(|t| t.as_str()) {
        if topic.contains("orderbook") {
            if let Some(bybit_data) = data.get("data") {
                if let Some(symbol) = bybit_data.get("s").and_then(|s| s.as_str()) {
                    let mut orderbook = OrderBook::new(symbol.to_string(), Exchange::Bybit);
                    
                    // Process bids
                    if let Some(bids) = bybit_data.get("b").and_then(|b| b.as_array()) {
                        for bid in bids.iter().take(5) { // Top 5 levels
                            if let Some(bid_array) = bid.as_array() {
                                if bid_array.len() >= 2 {
                                    if let (Some(price_str), Some(qty_str)) = (bid_array[0].as_str(), bid_array[1].as_str()) {
                                        if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                                            orderbook.update_bid(price, qty);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Process asks
                    if let Some(asks) = bybit_data.get("a").and_then(|a| a.as_array()) {
                        for ask in asks.iter().take(5) { // Top 5 levels
                            if let Some(ask_array) = ask.as_array() {
                                if ask_array.len() >= 2 {
                                    if let (Some(price_str), Some(qty_str)) = (ask_array[0].as_str(), ask_array[1].as_str()) {
                                        if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                                            orderbook.update_ask(price, qty);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    orderbook.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                    books.insert(symbol.to_string(), orderbook);
                    
                    info!("📊 Bybit {} - Bid: {:?}, Ask: {:?}", 
                          symbol, 
                          books[symbol].best_bid(), 
                          books[symbol].best_ask());
                }
            }
        }
    }
    Ok(())
}

/// Scan for arbitrage opportunities using real market data
async fn scan_arbitrage_opportunities(
    binance_books: &HashMap<String, OrderBook>,
    bybit_books: &HashMap<String, OrderBook>,
    config: &ArbitrageConfig,
) -> Vec<(String, f64, f64, f64)> { // (symbol, spread_bps, profit, price)
    let mut opportunities = Vec::new();
    
    for symbol in ["BTCUSDT", "ETHUSDT", "BNBUSDT"] {
        if let (Some(binance_book), Some(bybit_book)) = (binance_books.get(symbol), bybit_books.get(symbol)) {
            
            // Strategy 1: Bybit Maker Sell + Binance Taker Buy
            if let (Some(bybit_bid), Some(binance_ask)) = (bybit_book.best_bid(), binance_book.best_ask()) {
                if bybit_bid > binance_ask {
                    let spread = bybit_bid - binance_ask;
                    let spread_bps = (spread / binance_ask * 10000.0).round();
                    
                    if spread_bps >= config.strategy.min_spread_bps as f64 {
                        let quantity = 0.01; // Small test size
                        let maker_rebate = bybit_bid * quantity * 0.00025; // Bybit rebate
                        let taker_cost = binance_ask * quantity * 0.0004;  // Binance fee
                        let profit = spread * quantity + maker_rebate - taker_cost;
                        
                        if profit > 0.0 {
                            opportunities.push((symbol.to_string(), spread_bps, profit, bybit_bid));
                            info!("🎯 REAL OPPORTUNITY: {} - Sell Bybit@{:.2} + Buy Binance@{:.2}, Profit: ${:.4}",
                                  symbol, bybit_bid, binance_ask, profit);
                        }
                    }
                }
            }
            
            // Strategy 2: Bybit Maker Buy + Binance Taker Sell
            if let (Some(bybit_ask), Some(binance_bid)) = (bybit_book.best_ask(), binance_book.best_bid()) {
                if binance_bid > bybit_ask {
                    let spread = binance_bid - bybit_ask;
                    let spread_bps = (spread / bybit_ask * 10000.0).round();
                    
                    if spread_bps >= config.strategy.min_spread_bps as f64 {
                        let quantity = 0.01; // Small test size
                        let maker_rebate = bybit_ask * quantity * 0.00025; // Bybit rebate
                        let taker_cost = binance_bid * quantity * 0.0004;  // Binance fee
                        let profit = spread * quantity + maker_rebate - taker_cost;
                        
                        if profit > 0.0 {
                            opportunities.push((symbol.to_string(), spread_bps, profit, bybit_ask));
                            info!("🎯 REAL OPPORTUNITY: {} - Buy Bybit@{:.2} + Sell Binance@{:.2}, Profit: ${:.4}",
                                  symbol, bybit_ask, binance_bid, profit);
                        }
                    }
                }
            }
        }
    }
    
    opportunities
}
