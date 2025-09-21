//! Exchange connector traits and common types

use crate::{data::OrderBook, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Exchange connector trait
#[async_trait]
pub trait ExchangeConnector {
    /// Connect to the exchange
    async fn connect(&mut self) -> Result<()>;
    
    /// Disconnect from the exchange
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Get connection status
    fn connection_status(&self) -> super::ConnectionStatus;
    
    /// Subscribe to order book updates for a symbol
    async fn subscribe_orderbook(&mut self, symbol: &str) -> Result<()>;
    
    /// Subscribe to trade updates for a symbol
    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()>;
    
    /// Subscribe to ticker updates for a symbol
    async fn subscribe_ticker(&mut self, symbol: &str) -> Result<()>;
    
    /// Get current order book snapshot
    async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook>;
    
    /// Get account balances
    async fn get_balances(&self) -> Result<HashMap<String, Balance>>;
    
    /// Place a limit order
    async fn place_limit_order(&self, order: &LimitOrder) -> Result<OrderResponse>;
    
    /// Cancel an order
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse>;
    
    /// Get order status
    async fn get_order_status(&self, symbol: &str, order_id: &str) -> Result<OrderStatus>;
    
    /// Get market data receiver
    fn get_market_data_receiver(&self) -> Option<mpsc::Receiver<MarketDataUpdate>>;
    
    /// Get order update receiver
    fn get_order_update_receiver(&self) -> Option<mpsc::Receiver<OrderUpdate>>;
}

/// Account balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Asset symbol
    pub asset: String,
    /// Available balance
    pub free: f64,
    /// Locked balance (in orders)
    pub locked: f64,
}

impl Balance {
    /// Get total balance (free + locked)
    pub fn total(&self) -> f64 {
        self.free + self.locked
    }
}

/// Limit order request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrder {
    /// Trading symbol
    pub symbol: String,
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    /// Order quantity
    pub quantity: f64,
    /// Order price
    pub price: f64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Client order ID (optional)
    pub client_order_id: Option<String>,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good Till Canceled
    GTC,
    /// Immediate or Cancel
    IOC,
    /// Fill or Kill
    FOK,
    /// Good Till Crossing (Post Only)
    GTX,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
            TimeInForce::GTX => write!(f, "GTX"),
        }
    }
}

/// Order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// Exchange order ID
    pub order_id: String,
    /// Client order ID
    pub client_order_id: Option<String>,
    /// Trading symbol
    pub symbol: String,
    /// Order side
    pub side: OrderSide,
    /// Order quantity
    pub quantity: f64,
    /// Order price
    pub price: f64,
    /// Order status
    pub status: OrderStatus,
    /// Filled quantity
    pub filled_quantity: f64,
    /// Average fill price
    pub average_price: Option<f64>,
    /// Order timestamp
    pub timestamp: i64,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is new/pending
    New,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is fully filled
    Filled,
    /// Order is canceled
    Canceled,
    /// Order is rejected
    Rejected,
    /// Order is expired
    Expired,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::New => write!(f, "NEW"),
            OrderStatus::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
            OrderStatus::Filled => write!(f, "FILLED"),
            OrderStatus::Canceled => write!(f, "CANCELED"),
            OrderStatus::Rejected => write!(f, "REJECTED"),
            OrderStatus::Expired => write!(f, "EXPIRED"),
        }
    }
}

/// Market data update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataUpdate {
    /// Order book update
    OrderBook {
        /// Exchange name
        exchange: String,
        /// Trading symbol
        symbol: String,
        /// Order book data
        orderbook: OrderBook,
        /// Update timestamp
        timestamp: i64,
    },
    /// Trade update
    Trade {
        /// Exchange name
        exchange: String,
        /// Trading symbol
        symbol: String,
        /// Trade price
        price: f64,
        /// Trade quantity
        quantity: f64,
        /// Trade side (from taker perspective)
        side: OrderSide,
        /// Trade timestamp
        timestamp: i64,
    },
    /// Ticker update
    Ticker {
        /// Exchange name
        exchange: String,
        /// Trading symbol
        symbol: String,
        /// Best bid price
        bid_price: f64,
        /// Best ask price
        ask_price: f64,
        /// 24h volume
        volume_24h: f64,
        /// 24h price change
        price_change_24h: f64,
        /// Update timestamp
        timestamp: i64,
    },
}

/// Order update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdate {
    /// Exchange name
    pub exchange: String,
    /// Order response data
    pub order: OrderResponse,
    /// Update timestamp
    pub timestamp: i64,
}

/// Connection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEvent {
    /// Connection established
    Connected,
    /// Connection lost
    Disconnected,
    /// Reconnection attempt
    Reconnecting,
    /// Connection error
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_total() {
        let balance = Balance {
            asset: "BTC".to_string(),
            free: 1.0,
            locked: 0.5,
        };
        assert_eq!(balance.total(), 1.5);
    }

    #[test]
    fn test_order_side_display() {
        assert_eq!(OrderSide::Buy.to_string(), "BUY");
        assert_eq!(OrderSide::Sell.to_string(), "SELL");
    }

    #[test]
    fn test_time_in_force_display() {
        assert_eq!(TimeInForce::GTC.to_string(), "GTC");
        assert_eq!(TimeInForce::IOC.to_string(), "IOC");
        assert_eq!(TimeInForce::FOK.to_string(), "FOK");
        assert_eq!(TimeInForce::GTX.to_string(), "GTX");
    }

    #[test]
    fn test_order_status_display() {
        assert_eq!(OrderStatus::New.to_string(), "NEW");
        assert_eq!(OrderStatus::Filled.to_string(), "FILLED");
        assert_eq!(OrderStatus::Canceled.to_string(), "CANCELED");
    }
}
