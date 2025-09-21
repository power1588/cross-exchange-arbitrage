//! Futures arbitrage strategy implementation
//! Strategy: Bybit Maker + Binance Taker for cross-exchange arbitrage

use crate::{
    config::ArbitrageConfig,
    connectors::{
        Exchange, OrderSide,
        futures::{FuturesConnector, FuturesOrder, FuturesOrderType, FuturesTimeInForce, PositionSide, MarkPrice}
    },
    data::OrderBook,
    Result, ArbitrageError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Futures arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesArbitrageOpportunity {
    /// Symbol
    pub symbol: String,
    /// Maker exchange (Bybit - lower liquidity)
    pub maker_exchange: Exchange,
    /// Taker exchange (Binance - higher liquidity)
    pub taker_exchange: Exchange,
    /// Maker side (what we do on Bybit)
    pub maker_side: OrderSide,
    /// Taker side (what we do on Binance)
    pub taker_side: OrderSide,
    /// Maker price (our limit order on Bybit)
    pub maker_price: f64,
    /// Taker price (market order on Binance)
    pub taker_price: f64,
    /// Available quantity
    pub quantity: f64,
    /// Expected spread in basis points
    pub spread_bps: f64,
    /// Expected profit (including fees)
    pub expected_profit: f64,
    /// Maker fee (Bybit rebate)
    pub maker_fee: f64,
    /// Taker fee (Binance)
    pub taker_fee: f64,
    /// Risk score (0-100)
    pub risk_score: f64,
    /// Timestamp when opportunity was detected
    pub timestamp: i64,
}

/// Strategy state for futures arbitrage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuturesStrategyState {
    /// Strategy is stopped
    Stopped,
    /// Strategy is running
    Running,
    /// Strategy is paused
    Paused,
    /// Error state
    Error,
}

/// Futures arbitrage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesArbitrageStats {
    /// Total opportunities detected
    pub opportunities_detected: u64,
    /// Total opportunities executed
    pub opportunities_executed: u64,
    /// Total profit/loss
    pub total_pnl: f64,
    /// Total maker rebates earned
    pub total_maker_rebates: f64,
    /// Total taker fees paid
    pub total_taker_fees: f64,
    /// Success rate
    pub success_rate: f64,
    /// Average spread captured
    pub avg_spread_bps: f64,
    /// Total volume traded
    pub total_volume: f64,
    /// Strategy uptime
    pub uptime_seconds: u64,
    /// Last execution timestamp
    pub last_execution: Option<i64>,
}

impl Default for FuturesArbitrageStats {
    fn default() -> Self {
        Self {
            opportunities_detected: 0,
            opportunities_executed: 0,
            total_pnl: 0.0,
            total_maker_rebates: 0.0,
            total_taker_fees: 0.0,
            success_rate: 0.0,
            avg_spread_bps: 0.0,
            total_volume: 0.0,
            uptime_seconds: 0,
            last_execution: None,
        }
    }
}

/// Futures arbitrage strategy
pub struct FuturesArbitrageStrategy {
    /// Configuration
    config: ArbitrageConfig,
    /// Strategy state
    state: Arc<RwLock<FuturesStrategyState>>,
    /// Statistics
    statistics: Arc<RwLock<FuturesArbitrageStats>>,
    /// Current opportunities
    opportunities: Arc<RwLock<Vec<FuturesArbitrageOpportunity>>>,
    /// Market data cache (exchange -> symbol -> orderbook)
    market_data: Arc<RwLock<HashMap<Exchange, HashMap<String, OrderBook>>>>,
    /// Mark prices cache (exchange -> symbol -> mark_price)
    mark_prices: Arc<RwLock<HashMap<Exchange, HashMap<String, MarkPrice>>>>,
    /// Active symbols for monitoring
    active_symbols: Vec<String>,
    /// Start time
    start_time: std::time::Instant,
}

impl FuturesArbitrageStrategy {
    /// Create a new futures arbitrage strategy
    pub async fn new(config: ArbitrageConfig, symbols: Vec<String>) -> Result<Self> {
        info!("Creating futures arbitrage strategy for {} symbols", symbols.len());
        
        Ok(Self {
            config,
            state: Arc::new(RwLock::new(FuturesStrategyState::Stopped)),
            statistics: Arc::new(RwLock::new(FuturesArbitrageStats::default())),
            opportunities: Arc::new(RwLock::new(Vec::new())),
            market_data: Arc::new(RwLock::new(HashMap::new())),
            mark_prices: Arc::new(RwLock::new(HashMap::new())),
            active_symbols: symbols,
            start_time: std::time::Instant::now(),
        })
    }

    /// Start the strategy
    pub async fn start(&self) -> Result<()> {
        info!("Starting futures arbitrage strategy");
        let mut state = self.state.write().await;
        *state = FuturesStrategyState::Running;
        Ok(())
    }

    /// Stop the strategy
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping futures arbitrage strategy");
        let mut state = self.state.write().await;
        *state = FuturesStrategyState::Stopped;
        Ok(())
    }

    /// Update market data
    pub async fn update_orderbook(&self, exchange: Exchange, orderbook: OrderBook) -> Result<()> {
        let symbol = orderbook.symbol.clone();
        
        {
            let mut market_data = self.market_data.write().await;
            let exchange_data = market_data.entry(exchange).or_insert_with(HashMap::new);
            exchange_data.insert(symbol.clone(), orderbook);
        }
        
        // Trigger opportunity detection after market data update
        if self.is_running().await {
            self.detect_opportunities().await?;
        }
        
        Ok(())
    }

    /// Update mark price
    pub async fn update_mark_price(&self, exchange: Exchange, mark_price: MarkPrice) -> Result<()> {
        let symbol = mark_price.symbol.clone();
        
        {
            let mut mark_prices = self.mark_prices.write().await;
            let exchange_data = mark_prices.entry(exchange).or_insert_with(HashMap::new);
            exchange_data.insert(symbol, mark_price);
        }
        
        Ok(())
    }

    /// Detect arbitrage opportunities
    pub async fn detect_opportunities(&self) -> Result<Vec<FuturesArbitrageOpportunity>> {
        let market_data = self.market_data.read().await;
        let mut opportunities = Vec::new();

        // Get orderbooks from both exchanges
        let binance_data = market_data.get(&Exchange::Binance);
        let bybit_data = market_data.get(&Exchange::Bybit);

        if let (Some(binance_books), Some(bybit_books)) = (binance_data, bybit_data) {
            for symbol in &self.active_symbols {
                if let (Some(binance_book), Some(bybit_book)) = 
                    (binance_books.get(symbol), bybit_books.get(symbol)) {
                    
                    // Strategy: Bybit Maker + Binance Taker
                    // Look for opportunities where we can place maker orders on Bybit
                    // and immediately hedge with taker orders on Binance
                    
                    self.analyze_maker_taker_opportunity(
                        symbol,
                        bybit_book,   // Maker exchange (lower liquidity)
                        binance_book, // Taker exchange (higher liquidity)
                        &mut opportunities
                    ).await?;
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.opportunities_detected += opportunities.len() as u64;
        }

        // Store current opportunities
        {
            let mut current_opportunities = self.opportunities.write().await;
            *current_opportunities = opportunities.clone();
        }

        if !opportunities.is_empty() {
            info!("Detected {} futures arbitrage opportunities", opportunities.len());
            for opp in &opportunities {
                info!("Opportunity {}: {} on {} @ {:.2} -> {} on {} @ {:.2}, Spread: {:.2} bps, Profit: ${:.2}",
                      opp.symbol, opp.maker_side, opp.maker_exchange, opp.maker_price,
                      opp.taker_side, opp.taker_exchange, opp.taker_price,
                      opp.spread_bps, opp.expected_profit);
            }
        }

        Ok(opportunities)
    }

    /// Analyze maker-taker arbitrage opportunity
    async fn analyze_maker_taker_opportunity(
        &self,
        symbol: &str,
        maker_book: &OrderBook,  // Bybit (maker)
        taker_book: &OrderBook,  // Binance (taker)
        opportunities: &mut Vec<FuturesArbitrageOpportunity>
    ) -> Result<()> {
        
        // Scenario 1: Sell on Bybit (maker), Buy on Binance (taker)
        // When Bybit bid > Binance ask
        if let (Some(bybit_bid), Some(binance_ask)) = (maker_book.best_bid(), taker_book.best_ask()) {
            if bybit_bid > binance_ask {
                let spread = bybit_bid - binance_ask;
                let spread_bps = (spread / binance_ask * 10000.0).round();
                
                if spread_bps >= self.config.strategy.min_spread_bps as f64 {
                    let quantity = maker_book.best_bid_quantity().unwrap_or(0.0)
                        .min(taker_book.best_ask_quantity().unwrap_or(0.0))
                        .min(self.config.strategy.max_position_size);
                    
                    if quantity > self.config.execution.min_order_size {
                        let maker_fee: f64 = -0.00025; // Bybit maker rebate (-0.025%)
                        let taker_fee: f64 = 0.0004;   // Binance taker fee (0.04%)
                        
                        let maker_rebate = bybit_bid * quantity * maker_fee.abs();
                        let taker_cost = binance_ask * quantity * taker_fee;
                        let expected_profit = spread * quantity + maker_rebate - taker_cost;
                        
                        if expected_profit > 0.0 {
                            opportunities.push(FuturesArbitrageOpportunity {
                                symbol: symbol.to_string(),
                                maker_exchange: Exchange::Bybit,
                                taker_exchange: Exchange::Binance,
                                maker_side: OrderSide::Sell,
                                taker_side: OrderSide::Buy,
                                maker_price: bybit_bid,
                                taker_price: binance_ask,
                                quantity,
                                spread_bps,
                                expected_profit,
                                maker_fee,
                                taker_fee,
                                risk_score: self.calculate_risk_score(spread_bps, quantity).await,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }

        // Scenario 2: Buy on Bybit (maker), Sell on Binance (taker)
        // When Binance bid > Bybit ask
        if let (Some(bybit_ask), Some(binance_bid)) = (maker_book.best_ask(), taker_book.best_bid()) {
            if binance_bid > bybit_ask {
                let spread = binance_bid - bybit_ask;
                let spread_bps = (spread / bybit_ask * 10000.0).round();
                
                if spread_bps >= self.config.strategy.min_spread_bps as f64 {
                    let quantity = maker_book.best_ask_quantity().unwrap_or(0.0)
                        .min(taker_book.best_bid_quantity().unwrap_or(0.0))
                        .min(self.config.strategy.max_position_size);
                    
                    if quantity > self.config.execution.min_order_size {
                        let maker_fee: f64 = -0.00025; // Bybit maker rebate (-0.025%)
                        let taker_fee: f64 = 0.0004;   // Binance taker fee (0.04%)
                        
                        let maker_rebate = bybit_ask * quantity * maker_fee.abs();
                        let taker_cost = binance_bid * quantity * taker_fee;
                        let expected_profit = spread * quantity + maker_rebate - taker_cost;
                        
                        if expected_profit > 0.0 {
                            opportunities.push(FuturesArbitrageOpportunity {
                                symbol: symbol.to_string(),
                                maker_exchange: Exchange::Bybit,
                                taker_exchange: Exchange::Binance,
                                maker_side: OrderSide::Buy,
                                taker_side: OrderSide::Sell,
                                maker_price: bybit_ask,
                                taker_price: binance_bid,
                                quantity,
                                spread_bps,
                                expected_profit,
                                maker_fee,
                                taker_fee,
                                risk_score: self.calculate_risk_score(spread_bps, quantity).await,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Calculate risk score for an opportunity
    async fn calculate_risk_score(&self, spread_bps: f64, quantity: f64) -> f64 {
        let mut risk_score: f64 = 0.0;
        
        // Lower spread = higher risk
        if spread_bps < 10.0 {
            risk_score += 30.0;
        } else if spread_bps < 20.0 {
            risk_score += 15.0;
        }
        
        // Larger quantity = higher risk
        if quantity > 1.0 {
            risk_score += 20.0;
        } else if quantity > 0.5 {
            risk_score += 10.0;
        }
        
        // Market volatility (simplified)
        risk_score += 10.0;
        
        risk_score.min(100.0)
    }

    /// Execute an arbitrage opportunity
    pub async fn execute_opportunity(
        &self,
        opportunity: &FuturesArbitrageOpportunity,
        bybit_connector: &dyn FuturesConnector,
        binance_connector: &dyn FuturesConnector,
    ) -> Result<()> {
        info!("Executing futures arbitrage opportunity for {}", opportunity.symbol);

        // Step 1: Place maker order on Bybit
        let maker_order = FuturesOrder {
            symbol: opportunity.symbol.clone(),
            side: opportunity.maker_side,
            position_side: Some(PositionSide::Both),
            order_type: FuturesOrderType::Limit,
            quantity: opportunity.quantity,
            price: Some(opportunity.maker_price),
            stop_price: None,
            time_in_force: FuturesTimeInForce::GTX, // Post-only to ensure maker
            reduce_only: false,
            close_position: false,
            client_order_id: Some(format!("maker_{}_{}", opportunity.symbol, chrono::Utc::now().timestamp_millis())),
        };

        match bybit_connector.place_order(&maker_order).await {
            Ok(maker_response) => {
                info!("Maker order placed on Bybit: {}", maker_response.order_id);

                // Step 2: Immediately place taker order on Binance to hedge
                let taker_order = FuturesOrder {
                    symbol: opportunity.symbol.clone(),
                    side: opportunity.taker_side,
                    position_side: Some(PositionSide::Both),
                    order_type: FuturesOrderType::Market,
                    quantity: opportunity.quantity,
                    price: None,
                    stop_price: None,
                    time_in_force: FuturesTimeInForce::IOC,
                    reduce_only: false,
                    close_position: false,
                    client_order_id: Some(format!("taker_{}_{}", opportunity.symbol, chrono::Utc::now().timestamp_millis())),
                };

                match binance_connector.place_order(&taker_order).await {
                    Ok(taker_response) => {
                        info!("Taker order placed on Binance: {}", taker_response.order_id);
                        self.update_execution_statistics(opportunity).await;
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to place taker order on Binance: {}", e);
                        // TODO: Cancel the maker order on Bybit
                        warn!("Need to cancel maker order {} on Bybit", maker_response.order_id);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!("Failed to place maker order on Bybit: {}", e);
                Err(e)
            }
        }
    }

    /// Update execution statistics
    async fn update_execution_statistics(&self, opportunity: &FuturesArbitrageOpportunity) {
        let mut stats = self.statistics.write().await;
        stats.opportunities_executed += 1;
        stats.total_pnl += opportunity.expected_profit;
        stats.total_maker_rebates += opportunity.maker_price * opportunity.quantity * opportunity.maker_fee.abs();
        stats.total_taker_fees += opportunity.taker_price * opportunity.quantity * opportunity.taker_fee;
        stats.total_volume += opportunity.quantity * opportunity.maker_price;
        stats.last_execution = Some(chrono::Utc::now().timestamp());

        // Update average spread
        if stats.opportunities_executed > 1 {
            stats.avg_spread_bps = (stats.avg_spread_bps * (stats.opportunities_executed - 1) as f64 + opportunity.spread_bps) / stats.opportunities_executed as f64;
        } else {
            stats.avg_spread_bps = opportunity.spread_bps;
        }

        // Update success rate
        stats.success_rate = (stats.opportunities_executed as f64 / stats.opportunities_detected as f64) * 100.0;
    }

    /// Check if strategy is running
    async fn is_running(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, FuturesStrategyState::Running)
    }

    /// Get strategy statistics
    pub async fn get_statistics(&self) -> FuturesArbitrageStats {
        let stats = self.statistics.read().await;
        let mut stats_clone = stats.clone();
        stats_clone.uptime_seconds = self.start_time.elapsed().as_secs();
        stats_clone
    }

    /// Get current opportunities
    pub async fn get_current_opportunities(&self) -> Vec<FuturesArbitrageOpportunity> {
        self.opportunities.read().await.clone()
    }

    /// Get strategy state
    pub async fn get_state(&self) -> FuturesStrategyState {
        *self.state.read().await
    }

    /// Get active symbols
    pub fn get_active_symbols(&self) -> &[String] {
        &self.active_symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ArbitrageConfig {
        let mut config = ArbitrageConfig::default();
        config.strategy.min_spread_bps = 5;
        config.strategy.max_position_size = 1.0;
        config.execution.min_order_size = 0.001;
        config
    }

    #[tokio::test]
    async fn test_futures_strategy_creation() {
        let config = create_test_config();
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let strategy = FuturesArbitrageStrategy::new(config, symbols.clone()).await.unwrap();
        
        assert_eq!(strategy.get_state().await, FuturesStrategyState::Stopped);
        assert_eq!(strategy.get_active_symbols(), &symbols);
    }

    #[tokio::test]
    async fn test_strategy_state_management() {
        let config = create_test_config();
        let symbols = vec!["BTCUSDT".to_string()];
        let strategy = FuturesArbitrageStrategy::new(config, symbols).await.unwrap();
        
        assert_eq!(strategy.get_state().await, FuturesStrategyState::Stopped);
        
        strategy.start().await.unwrap();
        assert_eq!(strategy.get_state().await, FuturesStrategyState::Running);
        
        strategy.stop().await.unwrap();
        assert_eq!(strategy.get_state().await, FuturesStrategyState::Stopped);
    }

    #[tokio::test]
    async fn test_risk_score_calculation() {
        let config = create_test_config();
        let symbols = vec!["BTCUSDT".to_string()];
        let strategy = FuturesArbitrageStrategy::new(config, symbols).await.unwrap();
        
        // Low spread, high quantity = high risk
        let high_risk = strategy.calculate_risk_score(5.0, 2.0).await;
        assert!(high_risk > 50.0);
        
        // High spread, low quantity = low risk
        let low_risk = strategy.calculate_risk_score(25.0, 0.1).await;
        assert!(low_risk < 30.0);
    }
}
