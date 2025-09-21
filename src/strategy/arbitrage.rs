//! Cross-exchange arbitrage strategy implementation

use crate::{
    config::ArbitrageConfig,
    connectors::{Exchange, LimitOrder, OrderSide, TimeInForce},
    data::{OrderBook, MarketDataManager},
    trading::{DryRunExecutor, LiveTradingExecutor},
    Result,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Symbol being arbitraged
    pub symbol: String,
    /// Buy exchange
    pub buy_exchange: Exchange,
    /// Sell exchange
    pub sell_exchange: Exchange,
    /// Buy price
    pub buy_price: f64,
    /// Sell price
    pub sell_price: f64,
    /// Available quantity
    pub quantity: f64,
    /// Spread in basis points
    pub spread_bps: f64,
    /// Expected profit
    pub expected_profit: f64,
    /// Timestamp when opportunity was detected
    pub timestamp: i64,
}

/// Strategy state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyState {
    /// Strategy is stopped
    Stopped,
    /// Strategy is running
    Running,
    /// Strategy is paused
    Paused,
    /// Strategy encountered an error
    Error,
}

/// Strategy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStatistics {
    /// Total opportunities detected
    pub opportunities_detected: u64,
    /// Total opportunities executed
    pub opportunities_executed: u64,
    /// Total profit/loss
    pub total_pnl: f64,
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

impl Default for StrategyStatistics {
    fn default() -> Self {
        Self {
            opportunities_detected: 0,
            opportunities_executed: 0,
            total_pnl: 0.0,
            success_rate: 0.0,
            avg_spread_bps: 0.0,
            total_volume: 0.0,
            uptime_seconds: 0,
            last_execution: None,
        }
    }
}

/// Cross-exchange arbitrage strategy
pub struct ArbitrageStrategy {
    /// Configuration
    config: ArbitrageConfig,
    /// Market data manager
    market_data: Arc<RwLock<MarketDataManager>>,
    /// Strategy state
    state: Arc<RwLock<StrategyState>>,
    /// Strategy statistics
    statistics: Arc<RwLock<StrategyStatistics>>,
    /// Current opportunities
    opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
    /// Start time
    start_time: std::time::Instant,
}

impl ArbitrageStrategy {
    /// Create a new arbitrage strategy
    pub async fn new(config: ArbitrageConfig) -> Result<Self> {
        let market_data = MarketDataManager::new(1000); // 1000 updates buffer
        
        Ok(Self {
            config,
            market_data: Arc::new(RwLock::new(market_data)),
            state: Arc::new(RwLock::new(StrategyState::Stopped)),
            statistics: Arc::new(RwLock::new(StrategyStatistics::default())),
            opportunities: Arc::new(RwLock::new(Vec::new())),
            start_time: std::time::Instant::now(),
        })
    }
    
    /// Run the strategy with dry-run executor
    pub async fn run_with_executor<T>(&mut self, executor: &mut T) -> Result<()>
    where
        T: StrategyExecutor,
    {
        info!("Starting arbitrage strategy");
        
        // Set strategy state to running
        {
            let mut state = self.state.write().await;
            *state = StrategyState::Running;
        }
        
        // Main strategy loop
        let mut iteration = 0;
        while self.is_running().await {
            iteration += 1;
            debug!("Strategy iteration {}", iteration);
            
            // Update market data (in real implementation, this would be from live feeds)
            self.update_market_data().await?;
            
            // Detect arbitrage opportunities
            let opportunities = self.detect_opportunities().await?;
            
            // Execute profitable opportunities
            for opportunity in opportunities {
                if let Err(e) = self.execute_opportunity(executor, &opportunity).await {
                    error!("Failed to execute opportunity: {}", e);
                    self.update_error_statistics().await;
                }
            }
            
            // Update statistics
            self.update_statistics().await;
            
            // Sleep for a short interval (in real implementation, this would be event-driven)
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            // For demonstration, stop after a few iterations
            if iteration >= 10 {
                break;
            }
        }
        
        // Set strategy state to stopped
        {
            let mut state = self.state.write().await;
            *state = StrategyState::Stopped;
        }
        
        info!("Arbitrage strategy completed");
        Ok(())
    }
    
    /// Update market data with mock data for testing
    async fn update_market_data(&self) -> Result<()> {
        let market_data = self.market_data.write().await;
        
        // Create mock orderbooks with slight price differences for arbitrage opportunities
        let base_price = 50000.0 + (rand::random::<f64>() - 0.5) * 1000.0; // BTC price around $50k
        
        // Binance orderbook (slightly lower ask)
        let mut binance_book = OrderBook::new("BTCUSDT".to_string(), Exchange::Binance);
        binance_book.update_bid(base_price - 10.0, 1.0);
        binance_book.update_ask(base_price + 5.0, 1.0); // Lower ask for buying
        binance_book.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        // Bybit orderbook (slightly higher bid)
        let mut bybit_book = OrderBook::new("BTCUSDT".to_string(), Exchange::Bybit);
        bybit_book.update_bid(base_price + 15.0, 1.0); // Higher bid for selling
        bybit_book.update_ask(base_price + 25.0, 1.0);
        bybit_book.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        // Update market data (simulate the process_update method)
        use crate::connectors::MarketDataUpdate;
        
        let binance_update = MarketDataUpdate::OrderBook {
            exchange: "binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            orderbook: binance_book,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };
        
        let bybit_update = MarketDataUpdate::OrderBook {
            exchange: "bybit".to_string(),
            symbol: "BTCUSDT".to_string(),
            orderbook: bybit_book,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };
        
        market_data.process_update(binance_update).await;
        market_data.process_update(bybit_update).await;
        
        debug!("Updated market data with base price: {}", base_price);
        Ok(())
    }
    
    /// Detect arbitrage opportunities
    async fn detect_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let market_data = self.market_data.read().await;
        let mut opportunities = Vec::new();
        
        // Get orderbooks for the symbol
        let binance_book = market_data.get_orderbook(Exchange::Binance, &self.config.strategy.symbol).await;
        let bybit_book = market_data.get_orderbook(Exchange::Bybit, &self.config.strategy.symbol).await;
        
        if let (Some(binance_book), Some(bybit_book)) = (binance_book, bybit_book) {
            // Check for arbitrage opportunities
            
            // Opportunity 1: Buy on Binance, Sell on Bybit
            if let (Some(binance_ask), Some(bybit_bid)) = (binance_book.best_ask(), bybit_book.best_bid()) {
                if bybit_bid > binance_ask {
                    let spread_bps = ((bybit_bid - binance_ask) / binance_ask * 10000.0).round();
                    
                    if spread_bps >= self.config.strategy.min_spread_bps as f64 {
                        let quantity = (binance_book.best_ask_quantity().unwrap_or(0.0))
                            .min(bybit_book.best_bid_quantity().unwrap_or(0.0))
                            .min(self.config.strategy.max_position_size);
                        
                        if quantity > self.config.execution.min_order_size {
                            let expected_profit = (bybit_bid - binance_ask) * quantity;
                            
                            opportunities.push(ArbitrageOpportunity {
                                symbol: self.config.strategy.symbol.clone(),
                                buy_exchange: Exchange::Binance,
                                sell_exchange: Exchange::Bybit,
                                buy_price: binance_ask,
                                sell_price: bybit_bid,
                                quantity,
                                spread_bps,
                                expected_profit,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                    }
                }
            }
            
            // Opportunity 2: Buy on Bybit, Sell on Binance
            if let (Some(bybit_ask), Some(binance_bid)) = (bybit_book.best_ask(), binance_book.best_bid()) {
                if binance_bid > bybit_ask {
                    let spread_bps = ((binance_bid - bybit_ask) / bybit_ask * 10000.0).round();
                    
                    if spread_bps >= self.config.strategy.min_spread_bps as f64 {
                        let quantity = (bybit_book.best_ask_quantity().unwrap_or(0.0))
                            .min(binance_book.best_bid_quantity().unwrap_or(0.0))
                            .min(self.config.strategy.max_position_size);
                        
                        if quantity > self.config.execution.min_order_size {
                            let expected_profit = (binance_bid - bybit_ask) * quantity;
                            
                            opportunities.push(ArbitrageOpportunity {
                                symbol: self.config.strategy.symbol.clone(),
                                buy_exchange: Exchange::Bybit,
                                sell_exchange: Exchange::Binance,
                                buy_price: bybit_ask,
                                sell_price: binance_bid,
                                quantity,
                                spread_bps,
                                expected_profit,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }
        
        // Update opportunities count
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
            info!("Detected {} arbitrage opportunities", opportunities.len());
            for opp in &opportunities {
                info!("Opportunity: Buy {} @ {} on {}, Sell @ {} on {}, Spread: {:.2} bps, Profit: ${:.2}",
                      opp.quantity, opp.buy_price, opp.buy_exchange, 
                      opp.sell_price, opp.sell_exchange, opp.spread_bps, opp.expected_profit);
            }
        }
        
        Ok(opportunities)
    }
    
    /// Execute an arbitrage opportunity
    async fn execute_opportunity<T>(&self, executor: &mut T, opportunity: &ArbitrageOpportunity) -> Result<()>
    where
        T: StrategyExecutor,
    {
        info!("Executing arbitrage opportunity: {:.2} bps spread", opportunity.spread_bps);
        
        // Create buy order
        let buy_order = LimitOrder {
            symbol: opportunity.symbol.clone(),
            side: OrderSide::Buy,
            quantity: opportunity.quantity,
            price: opportunity.buy_price,
            time_in_force: TimeInForce::GTC,
            client_order_id: Some(format!("arb_buy_{}", uuid::Uuid::new_v4())),
        };
        
        // Create sell order
        let sell_order = LimitOrder {
            symbol: opportunity.symbol.clone(),
            side: OrderSide::Sell,
            quantity: opportunity.quantity,
            price: opportunity.sell_price,
            time_in_force: TimeInForce::GTC,
            client_order_id: Some(format!("arb_sell_{}", uuid::Uuid::new_v4())),
        };
        
        // Execute orders
        let buy_result = executor.execute_order(opportunity.buy_exchange, buy_order).await;
        let sell_result = executor.execute_order(opportunity.sell_exchange, sell_order).await;
        
        match (buy_result, sell_result) {
            (Ok(buy_response), Ok(sell_response)) => {
                info!("Successfully executed arbitrage: Buy order {} on {}, Sell order {} on {}",
                      buy_response.order_id, opportunity.buy_exchange,
                      sell_response.order_id, opportunity.sell_exchange);
                
                // Update success statistics
                self.update_success_statistics(opportunity).await;
                Ok(())
            }
            (Err(buy_error), Ok(_)) => {
                error!("Buy order failed: {}", buy_error);
                Err(buy_error)
            }
            (Ok(_), Err(sell_error)) => {
                error!("Sell order failed: {}", sell_error);
                Err(sell_error)
            }
            (Err(buy_error), Err(sell_error)) => {
                error!("Both orders failed: Buy: {}, Sell: {}", buy_error, sell_error);
                Err(buy_error)
            }
        }
    }
    
    /// Check if strategy is running
    async fn is_running(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, StrategyState::Running)
    }
    
    /// Update success statistics
    async fn update_success_statistics(&self, opportunity: &ArbitrageOpportunity) {
        let mut stats = self.statistics.write().await;
        stats.opportunities_executed += 1;
        stats.total_pnl += opportunity.expected_profit;
        stats.total_volume += opportunity.quantity * opportunity.buy_price;
        stats.last_execution = Some(chrono::Utc::now().timestamp());
        
        // Update average spread
        let total_spread = stats.avg_spread_bps * (stats.opportunities_executed - 1) as f64 + opportunity.spread_bps;
        stats.avg_spread_bps = total_spread / stats.opportunities_executed as f64;
        
        // Update success rate
        stats.success_rate = (stats.opportunities_executed as f64 / stats.opportunities_detected as f64) * 100.0;
    }
    
    /// Update error statistics
    async fn update_error_statistics(&self) {
        // Error statistics would be tracked here
        warn!("Arbitrage execution failed");
    }
    
    /// Update general statistics
    async fn update_statistics(&self) {
        let mut stats = self.statistics.write().await;
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get strategy statistics
    pub async fn get_statistics(&self) -> StrategyStatistics {
        self.statistics.read().await.clone()
    }
    
    /// Get current opportunities
    pub async fn get_current_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        self.opportunities.read().await.clone()
    }
    
    /// Stop the strategy
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = StrategyState::Stopped;
        info!("Strategy stopped");
    }
    
    /// Pause the strategy
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        *state = StrategyState::Paused;
        info!("Strategy paused");
    }
    
    /// Resume the strategy
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        *state = StrategyState::Running;
        info!("Strategy resumed");
    }
    
    /// Get strategy state
    pub async fn get_state(&self) -> StrategyState {
        *self.state.read().await
    }
}

/// Trait for strategy executors
pub trait StrategyExecutor {
    /// Execute an order on the specified exchange
    fn execute_order(&mut self, exchange: Exchange, order: LimitOrder) -> impl std::future::Future<Output = Result<crate::connectors::OrderResponse>> + '_;
}

/// Implementation for DryRunExecutor
impl StrategyExecutor for DryRunExecutor {
    fn execute_order(&mut self, _exchange: Exchange, order: LimitOrder) -> impl std::future::Future<Output = Result<crate::connectors::OrderResponse>> + '_ {
        async move {
            // For dry-run, we don't need to specify exchange as it's simulated
            self.execute_order(order).await
        }
    }
}

/// Implementation for LiveTradingExecutor
impl StrategyExecutor for LiveTradingExecutor {
    fn execute_order(&mut self, exchange: Exchange, order: LimitOrder) -> impl std::future::Future<Output = Result<crate::connectors::OrderResponse>> + '_ {
        async move {
            self.place_order(exchange, order).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ArbitrageConfig {
        let mut config = ArbitrageConfig::default();
        config.strategy.symbol = "BTCUSDT".to_string();
        config.strategy.min_spread_bps = 5;
        config.strategy.max_position_size = 1.0;
        config.execution.min_order_size = 0.001;
        config
    }

    #[tokio::test]
    async fn test_arbitrage_strategy_creation() {
        let config = create_test_config();
        let strategy = ArbitrageStrategy::new(config).await.unwrap();
        
        assert_eq!(strategy.get_state().await, StrategyState::Stopped);
        
        let stats = strategy.get_statistics().await;
        assert_eq!(stats.opportunities_detected, 0);
        assert_eq!(stats.opportunities_executed, 0);
    }

    #[tokio::test]
    async fn test_opportunity_detection() {
        let config = create_test_config();
        let strategy = ArbitrageStrategy::new(config).await.unwrap();
        
        // Update market data to create opportunities
        strategy.update_market_data().await.unwrap();
        
        // Detect opportunities
        let opportunities = strategy.detect_opportunities().await.unwrap();
        
        // Should detect at least one opportunity with mock data
        assert!(!opportunities.is_empty());
        
        for opp in opportunities {
            assert!(opp.spread_bps >= 5.0); // Min spread threshold
            assert!(opp.expected_profit > 0.0);
        }
    }

    #[tokio::test]
    async fn test_strategy_state_management() {
        let config = create_test_config();
        let strategy = ArbitrageStrategy::new(config).await.unwrap();
        
        // Test state transitions
        assert_eq!(strategy.get_state().await, StrategyState::Stopped);
        
        strategy.pause().await;
        assert_eq!(strategy.get_state().await, StrategyState::Paused);
        
        strategy.resume().await;
        assert_eq!(strategy.get_state().await, StrategyState::Running);
        
        strategy.stop().await;
        assert_eq!(strategy.get_state().await, StrategyState::Stopped);
    }
}
