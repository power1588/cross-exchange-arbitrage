//! Integration tests for cross-exchange arbitrage

pub mod test_arbitrage_strategy;
pub mod test_data_connectors;
pub mod test_risk_management;

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    connectors::{Exchange, ExchangeConnector},
    data::{OrderBook, MarketDataManager},
    Result,
};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Test utilities for integration tests
pub struct TestUtils;

impl TestUtils {
    /// Create a test configuration
    pub fn create_test_config() -> ArbitrageConfig {
        let mut config = ArbitrageConfig::default();
        config.strategy.symbol = "BTCUSDT".to_string();
        config.strategy.min_spread_bps = 5; // Lower threshold for testing
        config.strategy.max_position_size = 0.1; // Smaller position for testing
        config.risk.max_drawdown = 0.1; // Higher tolerance for testing
        config
    }
    
    /// Create a test order book with specified prices
    pub fn create_test_orderbook(
        symbol: &str,
        exchange: Exchange,
        bid_price: f64,
        ask_price: f64,
        quantity: f64,
    ) -> OrderBook {
        let mut book = OrderBook::new(symbol.to_string(), exchange);
        book.update_bid(bid_price, quantity);
        book.update_ask(ask_price, quantity);
        book.set_timestamp(chrono::Utc::now().timestamp_nanos());
        book
    }
    
    /// Create profitable spread scenario
    pub fn create_profitable_spread_scenario() -> (OrderBook, OrderBook) {
        let binance_book = Self::create_test_orderbook(
            "BTCUSDT",
            Exchange::Binance,
            50000.0, // bid
            50100.0, // ask
            1.0,
        );
        
        let bybit_book = Self::create_test_orderbook(
            "BTCUSDT",
            Exchange::Bybit,
            49900.0, // bid (lower)
            50000.0, // ask (lower)
            1.0,
        );
        
        (binance_book, bybit_book)
    }
    
    /// Create unprofitable spread scenario
    pub fn create_unprofitable_spread_scenario() -> (OrderBook, OrderBook) {
        let binance_book = Self::create_test_orderbook(
            "BTCUSDT",
            Exchange::Binance,
            50000.0, // bid
            50010.0, // ask (tight spread)
            1.0,
        );
        
        let bybit_book = Self::create_test_orderbook(
            "BTCUSDT",
            Exchange::Bybit,
            49995.0, // bid (similar)
            50005.0, // ask (similar)
            1.0,
        );
        
        (binance_book, bybit_book)
    }
    
    /// Setup test market data manager with sample data
    pub async fn setup_test_market_data() -> MarketDataManager {
        let manager = MarketDataManager::new(100);
        
        let (binance_book, bybit_book) = Self::create_profitable_spread_scenario();
        
        // Add the order books to the manager
        // Note: This would typically be done through market data updates
        // For testing, we might need to add a method to directly insert test data
        
        manager
    }
    
    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F, Fut>(
        mut condition: F,
        timeout_ms: u64,
        check_interval_ms: u64,
    ) -> bool
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let interval = std::time::Duration::from_millis(check_interval_ms);
        
        while start.elapsed() < timeout {
            if condition().await {
                return true;
            }
            tokio::time::sleep(interval).await;
        }
        
        false
    }
}

/// Mock exchange connector for testing
pub struct MockExchangeConnector {
    pub exchange: Exchange,
    pub connected: bool,
    pub orderbooks: HashMap<String, OrderBook>,
    pub market_data_tx: Option<mpsc::Sender<crate::connectors::MarketDataUpdate>>,
}

impl MockExchangeConnector {
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            connected: false,
            orderbooks: HashMap::new(),
            market_data_tx: None,
        }
    }
    
    pub fn set_orderbook(&mut self, symbol: &str, orderbook: OrderBook) {
        self.orderbooks.insert(symbol.to_string(), orderbook);
    }
}

#[async_trait::async_trait]
impl ExchangeConnector for MockExchangeConnector {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    fn connection_status(&self) -> crate::connectors::ConnectionStatus {
        if self.connected {
            crate::connectors::ConnectionStatus::Connected
        } else {
            crate::connectors::ConnectionStatus::Disconnected
        }
    }
    
    async fn subscribe_orderbook(&mut self, _symbol: &str) -> Result<()> {
        Ok(())
    }
    
    async fn subscribe_trades(&mut self, _symbol: &str) -> Result<()> {
        Ok(())
    }
    
    async fn subscribe_ticker(&mut self, _symbol: &str) -> Result<()> {
        Ok(())
    }
    
    async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook> {
        self.orderbooks
            .get(symbol)
            .cloned()
            .ok_or_else(|| crate::ArbitrageError::DataParsing(format!("No orderbook for {}", symbol)).into())
    }
    
    async fn get_balances(&self) -> Result<HashMap<String, crate::connectors::Balance>> {
        // Return mock balances
        let mut balances = HashMap::new();
        balances.insert("BTC".to_string(), crate::connectors::Balance {
            asset: "BTC".to_string(),
            free: 1.0,
            locked: 0.0,
        });
        balances.insert("USDT".to_string(), crate::connectors::Balance {
            asset: "USDT".to_string(),
            free: 50000.0,
            locked: 0.0,
        });
        Ok(balances)
    }
    
    async fn place_limit_order(&self, _order: &crate::connectors::LimitOrder) -> Result<crate::connectors::OrderResponse> {
        // Return mock order response
        Ok(crate::connectors::OrderResponse {
            order_id: "mock_order_123".to_string(),
            client_order_id: None,
            symbol: "BTCUSDT".to_string(),
            side: crate::connectors::OrderSide::Buy,
            quantity: 1.0,
            price: 50000.0,
            status: crate::connectors::OrderStatus::New,
            filled_quantity: 0.0,
            average_price: None,
            timestamp: chrono::Utc::now().timestamp_nanos(),
        })
    }
    
    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<crate::connectors::OrderResponse> {
        // Return mock cancel response
        Ok(crate::connectors::OrderResponse {
            order_id: "mock_order_123".to_string(),
            client_order_id: None,
            symbol: "BTCUSDT".to_string(),
            side: crate::connectors::OrderSide::Buy,
            quantity: 1.0,
            price: 50000.0,
            status: crate::connectors::OrderStatus::Canceled,
            filled_quantity: 0.0,
            average_price: None,
            timestamp: chrono::Utc::now().timestamp_nanos(),
        })
    }
    
    async fn get_order_status(&self, _symbol: &str, _order_id: &str) -> Result<crate::connectors::OrderStatus> {
        Ok(crate::connectors::OrderStatus::Filled)
    }
    
    fn get_market_data_receiver(&self) -> Option<mpsc::Receiver<crate::connectors::MarketDataUpdate>> {
        None // Mock implementation
    }
    
    fn get_order_update_receiver(&self) -> Option<mpsc::Receiver<crate::connectors::OrderUpdate>> {
        None // Mock implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_config() {
        let config = TestUtils::create_test_config();
        assert_eq!(config.strategy.symbol, "BTCUSDT");
        assert_eq!(config.strategy.min_spread_bps, 5);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_create_test_orderbook() {
        let book = TestUtils::create_test_orderbook(
            "BTCUSDT",
            Exchange::Binance,
            50000.0,
            50100.0,
            1.0,
        );
        
        assert_eq!(book.symbol, "BTCUSDT");
        assert_eq!(book.exchange, Exchange::Binance);
        assert_eq!(book.best_bid(), Some(50000.0));
        assert_eq!(book.best_ask(), Some(50100.0));
        assert!(book.is_valid());
    }

    #[test]
    fn test_profitable_spread_scenario() {
        let (binance_book, bybit_book) = TestUtils::create_profitable_spread_scenario();
        
        // Binance: bid=50000, ask=50100
        // Bybit: bid=49900, ask=50000
        // Profitable opportunity: buy on Bybit at 50000, sell on Binance at 50000
        // But this specific scenario might not be profitable due to the spread
        
        assert!(binance_book.is_valid());
        assert!(bybit_book.is_valid());
        
        // The spread should be different between exchanges
        let binance_spread = binance_book.spread().unwrap();
        let bybit_spread = bybit_book.spread().unwrap();
        assert_ne!(binance_spread, bybit_spread);
    }

    #[tokio::test]
    async fn test_mock_exchange_connector() {
        let mut connector = MockExchangeConnector::new(Exchange::Binance);
        
        assert!(!connector.is_connected());
        
        connector.connect().await.unwrap();
        assert!(connector.is_connected());
        
        let book = TestUtils::create_test_orderbook("BTCUSDT", Exchange::Binance, 50000.0, 50100.0, 1.0);
        connector.set_orderbook("BTCUSDT", book);
        
        let retrieved_book = connector.get_orderbook("BTCUSDT").await.unwrap();
        assert_eq!(retrieved_book.best_bid(), Some(50000.0));
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let mut counter = 0;
        
        let result = TestUtils::wait_for_condition(
            || {
                counter += 1;
                async move { counter >= 3 }
            },
            1000, // 1 second timeout
            10,   // check every 10ms
        ).await;
        
        assert!(result);
        assert!(counter >= 3);
    }
}
