//! Dry-run trading executor implementation

use crate::{
    config::ArbitrageConfig,
    connectors::{Exchange, LimitOrder, OrderResponse, OrderSide, OrderStatus},
    data::OrderBook,
    ArbitrageError,
    Result,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Portfolio state for dry-run simulation
#[derive(Debug, Clone, Default)]
pub struct Portfolio {
    /// Asset positions (symbol -> quantity)
    positions: HashMap<String, f64>,
    /// Cash balances (currency -> amount)
    balances: HashMap<String, f64>,
    /// Initial balances for PnL calculation
    initial_balances: HashMap<String, f64>,
}

impl Portfolio {
    /// Create a new portfolio with initial balances
    pub fn new(initial_balances: HashMap<String, f64>) -> Self {
        Self {
            positions: HashMap::new(),
            balances: initial_balances.clone(),
            initial_balances,
        }
    }
    
    /// Get position for a symbol
    pub fn get_position(&self, symbol: &str) -> f64 {
        self.positions.get(symbol).copied().unwrap_or(0.0)
    }
    
    /// Get balance for a currency
    pub fn get_balance(&self, currency: &str) -> f64 {
        self.balances.get(currency).copied().unwrap_or(0.0)
    }
    
    /// Update position
    pub fn update_position(&mut self, symbol: &str, delta: f64) {
        let current = self.get_position(symbol);
        self.positions.insert(symbol.to_string(), current + delta);
    }
    
    /// Update balance
    pub fn update_balance(&mut self, currency: &str, delta: f64) {
        let current = self.get_balance(currency);
        self.balances.insert(currency.to_string(), current + delta);
    }
    
    /// Calculate total PnL
    pub fn calculate_pnl(&self, current_prices: &HashMap<String, f64>) -> f64 {
        let mut total_pnl = 0.0;
        
        // Calculate PnL from position changes
        for (symbol, position) in &self.positions {
            if let Some(price) = current_prices.get(symbol) {
                total_pnl += position * price;
            }
        }
        
        // Add cash balance changes
        for (currency, balance) in &self.balances {
            let initial = self.initial_balances.get(currency).copied().unwrap_or(0.0);
            if currency == "USDT" || currency == "USD" {
                total_pnl += balance - initial;
            }
        }
        
        total_pnl
    }
}

/// Performance metrics for dry-run execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total number of orders
    pub total_orders: u64,
    /// Total trading volume
    pub total_volume: f64,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Success rate (filled orders / total orders)
    pub success_rate: f64,
    /// Total fees paid
    pub total_fees: f64,
    /// Sharpe ratio
    pub sharpe_ratio: Option<f64>,
    /// Maximum drawdown
    pub max_drawdown: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_orders: 0,
            total_volume: 0.0,
            average_execution_time: Duration::from_nanos(0),
            success_rate: 0.0,
            total_fees: 0.0,
            sharpe_ratio: None,
            max_drawdown: 0.0,
        }
    }
}

/// Execution configuration for dry-run mode
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Enable fee simulation
    pub enable_fees: bool,
    /// Maker fee rate
    pub maker_fee: f64,
    /// Taker fee rate
    pub taker_fee: f64,
    /// Simulate execution delays
    pub simulate_delays: bool,
    /// Simulate market impact
    pub simulate_market_impact: bool,
    /// Market impact factor (price impact per unit volume)
    pub market_impact_factor: f64,
    /// Allow partial fills
    pub allow_partial_fills: bool,
    /// Probability of partial fill (0.0 to 1.0)
    pub partial_fill_probability: f64,
    /// Order rejection probability (0.0 to 1.0)
    pub rejection_probability: f64,
    /// Minimum fill ratio for partial fills
    pub min_fill_ratio: f64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            enable_fees: true,
            maker_fee: 0.001,
            taker_fee: 0.001,
            simulate_delays: false,
            simulate_market_impact: false,
            market_impact_factor: 0.0001,
            allow_partial_fills: false,
            partial_fill_probability: 0.0,
            rejection_probability: 0.0,
            min_fill_ratio: 0.1,
        }
    }
}

/// Dry-run trading executor
pub struct DryRunExecutor {
    /// Configuration
    config: ArbitrageConfig,
    /// Execution configuration
    exec_config: ExecutionConfig,
    /// Portfolio state
    portfolio: Arc<RwLock<Portfolio>>,
    /// Market data (exchange -> symbol -> orderbook)
    market_data: Arc<RwLock<HashMap<Exchange, HashMap<String, OrderBook>>>>,
    /// Execution history
    execution_history: Arc<RwLock<Vec<OrderResponse>>>,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Current prices for PnL calculation
    current_prices: Arc<RwLock<HashMap<String, f64>>>,
    /// Random number generator
    rng: Arc<RwLock<rand::rngs::ThreadRng>>,
}

impl DryRunExecutor {
    /// Create a new dry-run executor
    pub async fn new(config: ArbitrageConfig) -> Result<Self> {
        let exec_config = ExecutionConfig::default();
        
        // Initialize portfolio with default balances
        let mut initial_balances = HashMap::new();
        initial_balances.insert("USDT".to_string(), 100000.0); // $100k USDT
        initial_balances.insert("BTC".to_string(), 0.0);
        initial_balances.insert("ETH".to_string(), 0.0);
        
        let portfolio = Portfolio::new(initial_balances);
        
        Ok(Self {
            config,
            exec_config,
            portfolio: Arc::new(RwLock::new(portfolio)),
            market_data: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            current_prices: Arc::new(RwLock::new(HashMap::new())),
            rng: Arc::new(RwLock::new(rand::thread_rng())),
        })
    }
    
    /// Execute a limit order in dry-run mode
    pub async fn execute_order(&mut self, order: LimitOrder) -> Result<OrderResponse> {
        let start_time = Instant::now();
        
        debug!("Executing dry-run order: {:?}", order);
        
        // Check for order rejection
        if self.should_reject_order().await {
            warn!("Order rejected in simulation");
            return Err(ArbitrageError::Trading("Order rejected in simulation".to_string()).into());
        }
        
        // Simulate execution delay
        if self.exec_config.simulate_delays {
            let delay = self.calculate_execution_delay().await;
            tokio::time::sleep(delay).await;
        }
        
        // Calculate execution price with slippage and market impact
        let execution_price = self.calculate_execution_price(&order).await?;
        
        // Determine fill quantity
        let fill_quantity = self.calculate_fill_quantity(&order).await;
        
        // Calculate fees
        let fees = if self.exec_config.enable_fees {
            self.calculate_fees(&order, fill_quantity, execution_price).await
        } else {
            0.0
        };
        
        // Update portfolio
        self.update_portfolio(&order, fill_quantity, execution_price, fees).await?;
        
        // Create order response
        let order_response = OrderResponse {
            order_id: Uuid::new_v4().to_string(),
            client_order_id: order.client_order_id.clone(),
            symbol: order.symbol.clone(),
            side: order.side,
            quantity: order.quantity,
            price: order.price,
            status: if fill_quantity == order.quantity {
                OrderStatus::Filled
            } else if fill_quantity > 0.0 {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::New
            },
            filled_quantity: fill_quantity,
            average_price: if fill_quantity > 0.0 { Some(execution_price) } else { None },
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };
        
        // Update execution history
        {
            let mut history = self.execution_history.write().await;
            history.push(order_response.clone());
        }
        
        // Update metrics
        self.update_metrics(start_time.elapsed(), &order_response, fees).await;
        
        info!("Dry-run order executed: {} {} {} @ {}", 
              order_response.side, order_response.filled_quantity, 
              order_response.symbol, execution_price);
        
        Ok(order_response)
    }
    
    /// Update market data for pricing
    pub async fn update_market_data(&mut self, exchange: Exchange, orderbook: OrderBook) -> Result<()> {
        let symbol = orderbook.symbol.clone();
        
        // Update market data
        {
            let mut market_data = self.market_data.write().await;
            let exchange_data = market_data.entry(exchange).or_insert_with(HashMap::new);
            exchange_data.insert(symbol.clone(), orderbook.clone());
        }
        
        // Update current price for PnL calculation
        if let Some(mid_price) = orderbook.mid_price() {
            let mut prices = self.current_prices.write().await;
            prices.insert(symbol, mid_price);
        }
        
        Ok(())
    }
    
    /// Get execution results
    pub async fn get_results(&self) -> super::ExecutionResults {
        let history = self.execution_history.read().await.clone();
        let portfolio = self.portfolio.read().await.clone();
        let current_prices = self.current_prices.read().await.clone();
        
        super::ExecutionResults {
            total_trades: history.len() as u64,
            total_pnl: portfolio.calculate_pnl(&current_prices),
        }
    }
    
    /// Get portfolio state
    pub async fn get_portfolio(&self) -> Portfolio {
        self.portfolio.read().await.clone()
    }
    
    /// Get total fees paid
    pub async fn get_total_fees(&self) -> f64 {
        let metrics = self.metrics.read().await.clone();
        metrics.total_fees
    }
    
    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Reset executor state
    pub async fn reset(&mut self) -> Result<()> {
        // Reset portfolio to initial state
        {
            let mut portfolio = self.portfolio.write().await;
            *portfolio = Portfolio::new(portfolio.initial_balances.clone());
        }
        
        // Clear execution history
        {
            let mut history = self.execution_history.write().await;
            history.clear();
        }
        
        // Reset metrics
        {
            let mut metrics = self.metrics.write().await;
            *metrics = PerformanceMetrics::default();
        }
        
        // Clear market data
        {
            let mut market_data = self.market_data.write().await;
            market_data.clear();
        }
        
        // Clear current prices
        {
            let mut prices = self.current_prices.write().await;
            prices.clear();
        }
        
        info!("Dry-run executor state reset");
        Ok(())
    }
    
    // Private helper methods
    
    async fn should_reject_order(&self) -> bool {
        if self.exec_config.rejection_probability <= 0.0 {
            return false;
        }
        
        let mut rng = self.rng.write().await;
        rng.gen::<f64>() < self.exec_config.rejection_probability
    }
    
    async fn calculate_execution_delay(&self) -> Duration {
        let mut rng = self.rng.write().await;
        let base_delay = Duration::from_millis(self.config.execution.order_timeout_ms / 10);
        let jitter = rng.gen_range(0.5..1.5);
        Duration::from_nanos((base_delay.as_nanos() as f64 * jitter) as u64)
    }
    
    async fn calculate_execution_price(&self, order: &LimitOrder) -> Result<f64> {
        let mut execution_price = order.price;
        
        // Apply slippage
        let slippage = self.config.execution.slippage_tolerance;
        if slippage > 0.0 {
            let mut rng = self.rng.write().await;
            let slippage_factor = rng.gen_range(0.0..slippage);
            match order.side {
                OrderSide::Buy => execution_price *= 1.0 + slippage_factor,
                OrderSide::Sell => execution_price *= 1.0 - slippage_factor,
            }
        }
        
        // Apply market impact
        if self.exec_config.simulate_market_impact {
            let impact = order.quantity * self.exec_config.market_impact_factor;
            match order.side {
                OrderSide::Buy => execution_price *= 1.0 + impact,
                OrderSide::Sell => execution_price *= 1.0 - impact,
            }
        }
        
        // Check against market data if available
        let market_data = self.market_data.read().await;
        for exchange_data in market_data.values() {
            if let Some(orderbook) = exchange_data.get(&order.symbol) {
                match order.side {
                    OrderSide::Buy => {
                        if let Some(ask) = orderbook.best_ask() {
                            execution_price = execution_price.max(ask);
                        }
                    }
                    OrderSide::Sell => {
                        if let Some(bid) = orderbook.best_bid() {
                            execution_price = execution_price.min(bid);
                        }
                    }
                }
                break;
            }
        }
        
        Ok(execution_price)
    }
    
    async fn calculate_fill_quantity(&self, order: &LimitOrder) -> f64 {
        if !self.exec_config.allow_partial_fills {
            return order.quantity;
        }
        
        let mut rng = self.rng.write().await;
        if rng.gen::<f64>() < self.exec_config.partial_fill_probability {
            let min_fill = order.quantity * self.exec_config.min_fill_ratio;
            rng.gen_range(min_fill..=order.quantity)
        } else {
            order.quantity
        }
    }
    
    async fn calculate_fees(&self, order: &LimitOrder, fill_quantity: f64, execution_price: f64) -> f64 {
        let notional_value = fill_quantity * execution_price;
        let fee_rate = if order.time_in_force == crate::connectors::TimeInForce::GTX {
            self.exec_config.maker_fee
        } else {
            self.exec_config.taker_fee
        };
        notional_value * fee_rate
    }
    
    async fn update_portfolio(&self, order: &LimitOrder, fill_quantity: f64, execution_price: f64, fees: f64) -> Result<()> {
        let mut portfolio = self.portfolio.write().await;
        
        let notional_value = fill_quantity * execution_price;
        
        match order.side {
            OrderSide::Buy => {
                // Increase position, decrease cash
                portfolio.update_position(&order.symbol, fill_quantity);
                portfolio.update_balance("USDT", -(notional_value + fees));
            }
            OrderSide::Sell => {
                // Decrease position, increase cash
                portfolio.update_position(&order.symbol, -fill_quantity);
                portfolio.update_balance("USDT", notional_value - fees);
            }
        }
        
        Ok(())
    }
    
    async fn update_metrics(&self, execution_time: Duration, order_response: &OrderResponse, fees: f64) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_orders += 1;
        metrics.total_volume += order_response.filled_quantity * order_response.price;
        metrics.total_fees += fees;
        
        // Update average execution time
        let total_time = metrics.average_execution_time * (metrics.total_orders - 1) as u32 + execution_time;
        metrics.average_execution_time = total_time / metrics.total_orders as u32;
        
        // Update success rate
        let filled_orders = if order_response.filled_quantity > 0.0 { 1 } else { 0 };
        metrics.success_rate = (metrics.success_rate * (metrics.total_orders - 1) as f64 + filled_orders as f64) / metrics.total_orders as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::TimeInForce;

    fn create_test_config() -> ArbitrageConfig {
        ArbitrageConfig::default()
    }

    fn create_test_order() -> LimitOrder {
        LimitOrder {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            quantity: 0.1,
            price: 50000.0,
            time_in_force: TimeInForce::GTC,
            client_order_id: Some("test_order".to_string()),
        }
    }

    #[tokio::test]
    async fn test_dry_run_executor_creation() {
        let config = create_test_config();
        let executor = DryRunExecutor::new(config).await.unwrap();
        
        let results = executor.get_results().await;
        assert_eq!(results.total_trades, 0);
        assert_eq!(results.total_pnl, 0.0);
    }

    #[tokio::test]
    async fn test_order_execution() {
        let config = create_test_config();
        let mut executor = DryRunExecutor::new(config).await.unwrap();
        
        let order = create_test_order();
        let result = executor.execute_order(order).await.unwrap();
        
        assert_eq!(result.symbol, "BTCUSDT");
        assert_eq!(result.side, OrderSide::Buy);
        assert_eq!(result.quantity, 0.1);
        assert!(result.filled_quantity > 0.0);
    }

    #[tokio::test]
    async fn test_portfolio_update() {
        let config = create_test_config();
        let mut executor = DryRunExecutor::new(config).await.unwrap();
        
        let order = create_test_order();
        executor.execute_order(order).await.unwrap();
        
        let portfolio = executor.get_portfolio().await;
        assert!(portfolio.get_position("BTCUSDT") > 0.0);
        assert!(portfolio.get_balance("USDT") < 100000.0); // Should have spent some USDT
    }

    #[tokio::test]
    async fn test_reset_functionality() {
        let config = create_test_config();
        let mut executor = DryRunExecutor::new(config).await.unwrap();
        
        // Execute an order
        let order = create_test_order();
        executor.execute_order(order).await.unwrap();
        
        // Verify state changed
        let results_before = executor.get_results().await;
        assert!(results_before.total_trades > 0);
        
        // Reset
        executor.reset().await.unwrap();
        
        // Verify state cleared
        let results_after = executor.get_results().await;
        assert_eq!(results_after.total_trades, 0);
        assert_eq!(results_after.total_pnl, 0.0);
    }
}
