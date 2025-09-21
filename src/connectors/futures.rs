//! Futures trading connector traits and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{connectors::{Exchange, OrderSide}, Result};

/// Futures contract specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesContract {
    /// Contract symbol (e.g., "BTCUSDT")
    pub symbol: String,
    /// Base asset (e.g., "BTC")
    pub base_asset: String,
    /// Quote asset (e.g., "USDT")
    pub quote_asset: String,
    /// Contract type (perpetual, quarterly, etc.)
    pub contract_type: ContractType,
    /// Minimum order size
    pub min_order_size: f64,
    /// Price precision
    pub price_precision: u8,
    /// Quantity precision
    pub quantity_precision: u8,
    /// Tick size
    pub tick_size: f64,
    /// Lot size
    pub lot_size: f64,
    /// Maker fee rate
    pub maker_fee: f64,
    /// Taker fee rate
    pub taker_fee: f64,
}

/// Contract type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractType {
    /// Perpetual contract
    Perpetual,
    /// Quarterly contract
    Quarterly,
    /// Bi-quarterly contract
    BiQuarterly,
}

/// Futures position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesPosition {
    /// Symbol
    pub symbol: String,
    /// Position side (Long/Short)
    pub side: PositionSide,
    /// Position size
    pub size: f64,
    /// Entry price
    pub entry_price: f64,
    /// Mark price
    pub mark_price: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Realized PnL
    pub realized_pnl: f64,
    /// Margin
    pub margin: f64,
    /// Leverage
    pub leverage: f64,
    /// Last update time
    pub update_time: i64,
}

/// Position side
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositionSide {
    /// Long position
    Long,
    /// Short position
    Short,
    /// Both (hedge mode)
    Both,
}

/// Futures order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesOrder {
    /// Symbol
    pub symbol: String,
    /// Order side
    pub side: OrderSide,
    /// Position side (for hedge mode)
    pub position_side: Option<PositionSide>,
    /// Order type
    pub order_type: FuturesOrderType,
    /// Quantity
    pub quantity: f64,
    /// Price (for limit orders)
    pub price: Option<f64>,
    /// Stop price (for stop orders)
    pub stop_price: Option<f64>,
    /// Time in force
    pub time_in_force: FuturesTimeInForce,
    /// Reduce only flag
    pub reduce_only: bool,
    /// Close position flag
    pub close_position: bool,
    /// Client order ID
    pub client_order_id: Option<String>,
}

/// Futures order type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FuturesOrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop market order
    StopMarket,
    /// Stop limit order
    StopLimit,
    /// Take profit market order
    TakeProfitMarket,
    /// Take profit limit order
    TakeProfitLimit,
}

/// Futures time in force
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FuturesTimeInForce {
    /// Good till cancel
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
    /// Good till crossing
    GTX,
}

/// Futures order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesOrderResponse {
    /// Order ID
    pub order_id: String,
    /// Client order ID
    pub client_order_id: Option<String>,
    /// Symbol
    pub symbol: String,
    /// Order side
    pub side: OrderSide,
    /// Position side
    pub position_side: Option<PositionSide>,
    /// Order type
    pub order_type: FuturesOrderType,
    /// Quantity
    pub quantity: f64,
    /// Price
    pub price: Option<f64>,
    /// Order status
    pub status: FuturesOrderStatus,
    /// Filled quantity
    pub filled_quantity: f64,
    /// Average fill price
    pub average_price: Option<f64>,
    /// Commission
    pub commission: f64,
    /// Commission asset
    pub commission_asset: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Futures order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FuturesOrderStatus {
    /// New order
    New,
    /// Partially filled
    PartiallyFilled,
    /// Filled
    Filled,
    /// Canceled
    Canceled,
    /// Rejected
    Rejected,
    /// Expired
    Expired,
}

/// Funding rate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRate {
    /// Symbol
    pub symbol: String,
    /// Funding rate
    pub funding_rate: f64,
    /// Funding time
    pub funding_time: i64,
    /// Next funding time
    pub next_funding_time: i64,
}

/// Mark price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPrice {
    /// Symbol
    pub symbol: String,
    /// Mark price
    pub mark_price: f64,
    /// Index price
    pub index_price: f64,
    /// Estimated settle price
    pub estimated_settle_price: Option<f64>,
    /// Last funding rate
    pub last_funding_rate: f64,
    /// Next funding time
    pub next_funding_time: i64,
    /// Interest rate
    pub interest_rate: f64,
    /// Timestamp
    pub timestamp: i64,
}

/// Futures trading connector trait
#[async_trait::async_trait]
pub trait FuturesConnector: Send + Sync {
    /// Get exchange info
    async fn get_exchange_info(&self) -> Result<HashMap<String, FuturesContract>>;
    
    /// Get account information
    async fn get_account_info(&self) -> Result<FuturesAccountInfo>;
    
    /// Get positions
    async fn get_positions(&self) -> Result<Vec<FuturesPosition>>;
    
    /// Place order
    async fn place_order(&self, order: &FuturesOrder) -> Result<FuturesOrderResponse>;
    
    /// Cancel order
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<FuturesOrderResponse>;
    
    /// Get order status
    async fn get_order_status(&self, symbol: &str, order_id: &str) -> Result<FuturesOrderResponse>;
    
    /// Get funding rate
    async fn get_funding_rate(&self, symbol: &str) -> Result<FundingRate>;
    
    /// Get mark price
    async fn get_mark_price(&self, symbol: &str) -> Result<MarkPrice>;
    
    /// Subscribe to order book updates
    async fn subscribe_orderbook(&mut self, symbol: &str) -> Result<()>;
    
    /// Subscribe to trade updates
    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()>;
    
    /// Subscribe to mark price updates
    async fn subscribe_mark_price(&mut self, symbol: &str) -> Result<()>;
    
    /// Subscribe to funding rate updates
    async fn subscribe_funding_rate(&mut self, symbol: &str) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Connect to exchange
    async fn connect(&mut self) -> Result<()>;
    
    /// Disconnect from exchange
    async fn disconnect(&mut self) -> Result<()>;
}

/// Futures account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesAccountInfo {
    /// Total wallet balance
    pub total_wallet_balance: f64,
    /// Total unrealized PnL
    pub total_unrealized_pnl: f64,
    /// Total margin balance
    pub total_margin_balance: f64,
    /// Total position initial margin
    pub total_position_initial_margin: f64,
    /// Total order initial margin
    pub total_order_initial_margin: f64,
    /// Available balance
    pub available_balance: f64,
    /// Maximum withdraw amount
    pub max_withdraw_amount: f64,
    /// Margin ratio
    pub margin_ratio: Option<f64>,
    /// Update time
    pub update_time: i64,
    /// Asset balances
    pub balances: Vec<FuturesBalance>,
    /// Positions
    pub positions: Vec<FuturesPosition>,
}

/// Futures balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesBalance {
    /// Asset
    pub asset: String,
    /// Wallet balance
    pub wallet_balance: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Margin balance
    pub margin_balance: f64,
    /// Maintenance margin
    pub maint_margin: f64,
    /// Initial margin
    pub initial_margin: f64,
    /// Position initial margin
    pub position_initial_margin: f64,
    /// Order initial margin
    pub order_initial_margin: f64,
    /// Available balance
    pub available_balance: f64,
    /// Maximum withdraw amount
    pub max_withdraw_amount: f64,
    /// Margin available
    pub margin_available: bool,
    /// Update time
    pub update_time: i64,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
            PositionSide::Both => write!(f, "BOTH"),
        }
    }
}

impl std::fmt::Display for FuturesOrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuturesOrderType::Market => write!(f, "MARKET"),
            FuturesOrderType::Limit => write!(f, "LIMIT"),
            FuturesOrderType::StopMarket => write!(f, "STOP_MARKET"),
            FuturesOrderType::StopLimit => write!(f, "STOP_LIMIT"),
            FuturesOrderType::TakeProfitMarket => write!(f, "TAKE_PROFIT_MARKET"),
            FuturesOrderType::TakeProfitLimit => write!(f, "TAKE_PROFIT_LIMIT"),
        }
    }
}

impl std::fmt::Display for FuturesTimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuturesTimeInForce::GTC => write!(f, "GTC"),
            FuturesTimeInForce::IOC => write!(f, "IOC"),
            FuturesTimeInForce::FOK => write!(f, "FOK"),
            FuturesTimeInForce::GTX => write!(f, "GTX"),
        }
    }
}
