//! Live trading executor implementation

use crate::{
    config::ArbitrageConfig,
    connectors::{
        Exchange, ExchangeConnector,
        LimitOrder, OrderResponse, OrderSide, OrderStatus, Balance,
    },
    data::OrderBook,
    ArbitrageError,
    Result,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Health status for the live trading system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall system health
    pub is_healthy: bool,
    /// Exchange connection statuses
    pub exchange_connections: HashMap<Exchange, bool>,
    /// Last heartbeat timestamp
    pub last_heartbeat: i64,
    /// Active orders count
    pub active_orders: u64,
    /// Error count in last hour
    pub recent_errors: u64,
    /// System uptime in seconds
    pub uptime_seconds: u64,
}

/// Execution statistics for live trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Total orders placed
    pub total_orders: u64,
    /// Successful orders
    pub successful_orders: u64,
    /// Failed orders
    pub failed_orders: u64,
    /// Average execution time
    pub avg_execution_time_ms: f64,
    /// Total volume traded
    pub total_volume: f64,
    /// Total fees paid
    pub total_fees: f64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Last execution timestamp
    pub last_execution: Option<i64>,
}

impl Default for ExecutionStatistics {
    fn default() -> Self {
        Self {
            total_orders: 0,
            successful_orders: 0,
            failed_orders: 0,
            avg_execution_time_ms: 0.0,
            total_volume: 0.0,
            total_fees: 0.0,
            success_rate: 0.0,
            last_execution: None,
        }
    }
}

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Exchange where position is held
    pub exchange: Exchange,
    /// Symbol
    pub symbol: String,
    /// Position size (positive for long, negative for short)
    pub size: f64,
    /// Average entry price
    pub avg_price: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Last update timestamp
    pub last_update: i64,
}

/// Exchange information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeInfo {
    /// Exchange name
    pub exchange: Exchange,
    /// Available symbols
    pub symbols: Vec<String>,
    /// Trading fees
    pub fees: HashMap<String, f64>,
    /// Minimum order sizes
    pub min_order_sizes: HashMap<String, f64>,
    /// Tick sizes
    pub tick_sizes: HashMap<String, f64>,
    /// Server time
    pub server_time: i64,
}

/// Live trading executor
pub struct LiveTradingExecutor {
    /// Configuration
    config: ArbitrageConfig,
    /// Exchange connectors
    connectors: Arc<RwLock<HashMap<Exchange, Box<dyn ExchangeConnector + Send + Sync>>>>,
    /// Active orders tracking
    active_orders: Arc<RwLock<HashMap<String, (Exchange, OrderResponse)>>>,
    /// Positions tracking
    positions: Arc<RwLock<HashMap<String, Position>>>,
    /// Execution statistics
    statistics: Arc<RwLock<ExecutionStatistics>>,
    /// Health status
    health: Arc<RwLock<HealthStatus>>,
    /// Market data cache
    market_data: Arc<RwLock<HashMap<Exchange, HashMap<String, OrderBook>>>>,
    /// System start time
    start_time: Instant,
    /// Emergency shutdown flag
    emergency_shutdown: Arc<RwLock<bool>>,
}

impl LiveTradingExecutor {
    /// Create a new live trading executor
    pub async fn new(config: ArbitrageConfig) -> Result<Self> {
        // Validate configuration
        Self::validate_config(&config)?;
        
        let start_time = Instant::now();
        
        // Initialize health status
        let health = HealthStatus {
            is_healthy: false,
            exchange_connections: HashMap::new(),
            last_heartbeat: chrono::Utc::now().timestamp(),
            active_orders: 0,
            recent_errors: 0,
            uptime_seconds: 0,
        };
        
        Ok(Self {
            config,
            connectors: Arc::new(RwLock::new(HashMap::new())),
            active_orders: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(ExecutionStatistics::default())),
            health: Arc::new(RwLock::new(health)),
            market_data: Arc::new(RwLock::new(HashMap::new())),
            start_time,
            emergency_shutdown: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Connect to all configured exchanges
    pub async fn connect_to_exchanges(&mut self) -> Result<()> {
        info!("Connecting to exchanges...");
        
        // Create exchange configurations (would be loaded from config in real implementation)
        let exchanges = vec![Exchange::Binance, Exchange::Bybit];
        
        let mut connectors = self.connectors.write().await;
        let mut health = self.health.write().await;
        
        for exchange in exchanges {
            match self.create_connector(exchange).await {
                Ok(mut connector) => {
                    match connector.connect().await {
                        Ok(()) => {
                            info!("Successfully connected to {}", exchange);
                            health.exchange_connections.insert(exchange, true);
                            connectors.insert(exchange, connector);
                        }
                        Err(e) => {
                            error!("Failed to connect to {}: {}", exchange, e);
                            health.exchange_connections.insert(exchange, false);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create connector for {}: {}", exchange, e);
                    health.exchange_connections.insert(exchange, false);
                    return Err(e);
                }
            }
        }
        
        health.is_healthy = health.exchange_connections.values().all(|&connected| connected);
        info!("Exchange connection completed. Healthy: {}", health.is_healthy);
        
        Ok(())
    }
    
    /// Check if connected to exchanges
    pub async fn is_connected(&self) -> bool {
        let health = self.health.read().await;
        health.is_healthy
    }
    
    /// Get list of connected exchanges
    pub async fn get_connected_exchanges(&self) -> Vec<Exchange> {
        let health = self.health.read().await;
        health.exchange_connections
            .iter()
            .filter_map(|(exchange, &connected)| if connected { Some(*exchange) } else { None })
            .collect()
    }
    
    /// Check account balances on all exchanges
    pub async fn check_balances(&self) -> Result<HashMap<Exchange, HashMap<String, Balance>>> {
        debug!("Checking balances across all exchanges");
        
        let connectors = self.connectors.read().await;
        let mut all_balances = HashMap::new();
        
        for (exchange, connector) in connectors.iter() {
            match connector.get_balances().await {
                Ok(balances) => {
                    info!("Retrieved {} balances from {}", balances.len(), exchange);
                    all_balances.insert(*exchange, balances);
                }
                Err(e) => {
                    error!("Failed to get balances from {}: {}", exchange, e);
                    return Err(e);
                }
            }
        }
        
        Ok(all_balances)
    }
    
    /// Check connectivity to all exchanges
    pub async fn check_connectivity(&self) -> Result<()> {
        debug!("Checking connectivity to all exchanges");
        
        let connectors = self.connectors.read().await;
        let mut health = self.health.write().await;
        
        for (exchange, connector) in connectors.iter() {
            let connected = connector.is_connected();
            health.exchange_connections.insert(*exchange, connected);
            
            if !connected {
                warn!("Lost connection to {}", exchange);
            }
        }
        
        health.is_healthy = health.exchange_connections.values().all(|&connected| connected);
        health.last_heartbeat = chrono::Utc::now().timestamp();
        
        if !health.is_healthy {
            return Err(ArbitrageError::Connection("Not all exchanges are connected".to_string()).into());
        }
        
        Ok(())
    }
    
    /// Place a limit order on specified exchange
    pub async fn place_order(&mut self, exchange: Exchange, order: LimitOrder) -> Result<OrderResponse> {
        // Check emergency shutdown
        if *self.emergency_shutdown.read().await {
            return Err(ArbitrageError::Trading("System in emergency shutdown".to_string()).into());
        }
        
        // Check risk limits
        self.check_risk_limits(&order).await?;
        
        let start_time = Instant::now();
        debug!("Placing order on {}: {:?}", exchange, order);
        
        let connectors = self.connectors.read().await;
        let connector = connectors.get(&exchange)
            .ok_or_else(|| ArbitrageError::Trading(format!("No connector for {}", exchange)))?;
        
        match connector.place_limit_order(&order).await {
            Ok(response) => {
                info!("Order placed successfully: {} on {}", response.order_id, exchange);
                
                // Track active order
                {
                    let mut active_orders = self.active_orders.write().await;
                    active_orders.insert(response.order_id.clone(), (exchange, response.clone()));
                }
                
                // Update statistics
                self.update_statistics(start_time.elapsed(), true, &response).await;
                
                Ok(response)
            }
            Err(e) => {
                error!("Failed to place order on {}: {}", exchange, e);
                self.update_statistics(start_time.elapsed(), false, &OrderResponse::default()).await;
                Err(e)
            }
        }
    }
    
    /// Place order with retry mechanism
    pub async fn place_order_with_retry(&mut self, exchange: Exchange, order: LimitOrder) -> Result<OrderResponse> {
        let max_retries = self.config.execution.max_retry_attempts;
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            match self.place_order(exchange, order.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!("Order placement attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    /// Cancel an order
    pub async fn cancel_order(&mut self, exchange: Exchange, symbol: &str, order_id: &str) -> Result<OrderResponse> {
        debug!("Cancelling order {} on {}", order_id, exchange);
        
        let connectors = self.connectors.read().await;
        let connector = connectors.get(&exchange)
            .ok_or_else(|| ArbitrageError::Trading(format!("No connector for {}", exchange)))?;
        
        match connector.cancel_order(symbol, order_id).await {
            Ok(response) => {
                info!("Order cancelled successfully: {} on {}", order_id, exchange);
                
                // Remove from active orders
                {
                    let mut active_orders = self.active_orders.write().await;
                    active_orders.remove(order_id);
                }
                
                Ok(response)
            }
            Err(e) => {
                error!("Failed to cancel order {} on {}: {}", order_id, exchange, e);
                Err(e)
            }
        }
    }
    
    /// Get order status
    pub async fn get_order_status(&self, exchange: Exchange, symbol: &str, order_id: &str) -> Result<OrderStatus> {
        let connectors = self.connectors.read().await;
        let connector = connectors.get(&exchange)
            .ok_or_else(|| ArbitrageError::Trading(format!("No connector for {}", exchange)))?;
        
        connector.get_order_status(symbol, order_id).await
    }
    
    /// Get current positions
    pub async fn get_positions(&self) -> Result<HashMap<String, Position>> {
        Ok(self.positions.read().await.clone())
    }
    
    /// Check risk limits for an order
    pub async fn check_risk_limits(&self, order: &LimitOrder) -> Result<()> {
        // Check position size limits
        let positions = self.positions.read().await;
        let current_position = positions.get(&order.symbol)
            .map(|p| p.size)
            .unwrap_or(0.0);
        
        let new_position = match order.side {
            OrderSide::Buy => current_position + order.quantity,
            OrderSide::Sell => current_position - order.quantity,
        };
        
        if new_position.abs() > self.config.strategy.max_position_size {
            return Err(ArbitrageError::RiskManagement(
                format!("Position size {} exceeds limit {}", 
                       new_position.abs(), self.config.strategy.max_position_size)
            ).into());
        }
        
        // Check minimum order size
        if order.quantity < self.config.execution.min_order_size {
            return Err(ArbitrageError::RiskManagement(
                format!("Order size {} below minimum {}", 
                       order.quantity, self.config.execution.min_order_size)
            ).into());
        }
        
        Ok(())
    }
    
    /// Subscribe to market data for a symbol
    pub async fn subscribe_market_data(&mut self, exchange: Exchange, symbol: &str) -> Result<()> {
        debug!("Subscribing to market data for {} on {}", symbol, exchange);
        
        let mut connectors = self.connectors.write().await;
        let connector = connectors.get_mut(&exchange)
            .ok_or_else(|| ArbitrageError::Trading(format!("No connector for {}", exchange)))?;
        
        // Subscribe to orderbook and trades
        connector.subscribe_orderbook(symbol).await?;
        connector.subscribe_trades(symbol).await?;
        
        info!("Subscribed to market data for {} on {}", symbol, exchange);
        Ok(())
    }
    
    /// Process incoming market data
    pub async fn process_market_data(&mut self, exchange: Exchange, orderbook: OrderBook) -> Result<()> {
        let symbol = orderbook.symbol.clone();
        
        // Update market data cache
        {
            let mut market_data = self.market_data.write().await;
            let exchange_data = market_data.entry(exchange).or_insert_with(HashMap::new);
            exchange_data.insert(symbol.clone(), orderbook);
        }
        
        // Update positions with current market prices
        self.update_position_pnl(exchange, &symbol).await?;
        
        Ok(())
    }
    
    /// Handle connection error and attempt recovery
    pub async fn handle_connection_error(&mut self, exchange: Exchange) -> Result<()> {
        warn!("Handling connection error for {}", exchange);
        
        // Update health status
        {
            let mut health = self.health.write().await;
            health.exchange_connections.insert(exchange, false);
            health.is_healthy = false;
            health.recent_errors += 1;
        }
        
        // Attempt reconnection
        let mut connectors = self.connectors.write().await;
        if let Some(connector) = connectors.get_mut(&exchange) {
            match connector.connect().await {
                Ok(()) => {
                    info!("Successfully reconnected to {}", exchange);
                    let mut health = self.health.write().await;
                    health.exchange_connections.insert(exchange, true);
                    health.is_healthy = health.exchange_connections.values().all(|&connected| connected);
                }
                Err(e) => {
                    error!("Failed to reconnect to {}: {}", exchange, e);
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Emergency shutdown - cancel all orders and disconnect
    pub async fn emergency_shutdown(&mut self) -> Result<()> {
        warn!("Initiating emergency shutdown");
        
        // Set emergency flag
        {
            let mut shutdown = self.emergency_shutdown.write().await;
            *shutdown = true;
        }
        
        // Cancel all active orders
        let active_orders = self.active_orders.read().await.clone();
        for (order_id, (exchange, order_response)) in active_orders {
            if let Err(e) = self.cancel_order(exchange, &order_response.symbol, &order_id).await {
                error!("Failed to cancel order {} during emergency shutdown: {}", order_id, e);
            }
        }
        
        // Disconnect from all exchanges
        let mut connectors = self.connectors.write().await;
        for (exchange, connector) in connectors.iter_mut() {
            if let Err(e) = connector.disconnect().await {
                error!("Failed to disconnect from {} during emergency shutdown: {}", exchange, e);
            }
        }
        
        info!("Emergency shutdown completed");
        Ok(())
    }
    
    /// Get health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let mut health = self.health.write().await;
        health.uptime_seconds = self.start_time.elapsed().as_secs();
        health.active_orders = self.active_orders.read().await.len() as u64;
        Ok(health.clone())
    }
    
    /// Get execution statistics
    pub async fn get_execution_statistics(&self) -> Result<ExecutionStatistics> {
        Ok(self.statistics.read().await.clone())
    }
    
    /// Get exchange information
    pub async fn get_exchange_info(&self, exchange: Exchange) -> Result<ExchangeInfo> {
        // This would fetch real exchange info in production
        // For now, return mock data
        Ok(ExchangeInfo {
            exchange,
            symbols: vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
            fees: HashMap::new(),
            min_order_sizes: HashMap::new(),
            tick_sizes: HashMap::new(),
            server_time: chrono::Utc::now().timestamp(),
        })
    }
    
    // Private helper methods
    
    fn validate_config(config: &ArbitrageConfig) -> Result<()> {
        if config.strategy.max_position_size <= 0.0 {
            return Err(ArbitrageError::Config("Invalid max_position_size".to_string()).into());
        }
        
        if config.execution.min_order_size <= 0.0 {
            return Err(ArbitrageError::Config("Invalid min_order_size".to_string()).into());
        }
        
        Ok(())
    }
    
    async fn create_connector(&self, exchange: Exchange) -> Result<Box<dyn ExchangeConnector + Send + Sync>> {
        // In a real implementation, this would create connectors with proper configuration
        // For now, return an error since we don't have real API keys
        Err(ArbitrageError::Config(format!("Cannot create connector for {} without API keys", exchange)).into())
    }
    
    async fn update_statistics(&self, execution_time: Duration, success: bool, response: &OrderResponse) {
        let mut stats = self.statistics.write().await;
        
        stats.total_orders += 1;
        if success {
            stats.successful_orders += 1;
            stats.total_volume += response.quantity * response.price;
            stats.last_execution = Some(chrono::Utc::now().timestamp());
        } else {
            stats.failed_orders += 1;
        }
        
        // Update average execution time
        let total_time = stats.avg_execution_time_ms * (stats.total_orders - 1) as f64 + execution_time.as_millis() as f64;
        stats.avg_execution_time_ms = total_time / stats.total_orders as f64;
        
        // Update success rate
        stats.success_rate = (stats.successful_orders as f64 / stats.total_orders as f64) * 100.0;
    }
    
    async fn update_position_pnl(&self, _exchange: Exchange, _symbol: &str) -> Result<()> {
        // This would update position PnL based on current market prices
        // Implementation would depend on position tracking logic
        Ok(())
    }
}

// Default implementation for OrderResponse (used in error cases)
impl Default for OrderResponse {
    fn default() -> Self {
        Self {
            order_id: String::new(),
            client_order_id: None,
            symbol: String::new(),
            side: OrderSide::Buy,
            quantity: 0.0,
            price: 0.0,
            status: OrderStatus::New,
            filled_quantity: 0.0,
            average_price: None,
            timestamp: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::connectors::TimeInForce; // Not needed in tests

    fn create_test_config() -> ArbitrageConfig {
        ArbitrageConfig::default()
    }

    #[tokio::test]
    async fn test_live_trading_executor_creation() {
        let config = create_test_config();
        let executor = LiveTradingExecutor::new(config).await.unwrap();
        
        assert!(!executor.is_connected().await);
        assert_eq!(executor.get_connected_exchanges().await.len(), 0);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = create_test_config();
        config.strategy.max_position_size = -1.0;
        
        let result = LiveTradingExecutor::new(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_status() {
        let config = create_test_config();
        let executor = LiveTradingExecutor::new(config).await.unwrap();
        
        let health = executor.get_health_status().await.unwrap();
        assert!(!health.is_healthy);
        // uptime_seconds is u64, so it's always >= 0
    }

    #[tokio::test]
    async fn test_execution_statistics() {
        let config = create_test_config();
        let executor = LiveTradingExecutor::new(config).await.unwrap();
        
        let stats = executor.get_execution_statistics().await.unwrap();
        assert_eq!(stats.total_orders, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_emergency_shutdown() {
        let config = create_test_config();
        let mut executor = LiveTradingExecutor::new(config).await.unwrap();
        
        let result = executor.emergency_shutdown().await;
        assert!(result.is_ok());
        
        // Check that emergency flag is set
        assert!(*executor.emergency_shutdown.read().await);
    }
}
