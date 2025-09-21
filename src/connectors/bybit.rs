//! Bybit exchange connector implementation

use crate::{
    config::ExchangeConfig,
    connectors::{
        traits::*,
        ConnectionStatus,
    },
    data::OrderBook,
    ArbitrageError,
    Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use url::Url;

/// Bybit exchange connector
pub struct BybitConnector {
    config: ExchangeConfig,
    connection_status: ConnectionStatus,
    market_data_tx: Option<mpsc::Sender<MarketDataUpdate>>,
    order_update_tx: Option<mpsc::Sender<OrderUpdate>>,
    websocket_client: Option<BybitWebSocketClient>,
    rest_client: BybitRestClient,
}

impl BybitConnector {
    /// Create a new Bybit connector
    pub async fn new(config: ExchangeConfig) -> Result<Self> {
        let rest_client = BybitRestClient::new(&config)?;
        
        Ok(Self {
            config,
            connection_status: ConnectionStatus::Disconnected,
            market_data_tx: None,
            order_update_tx: None,
            websocket_client: None,
            rest_client,
        })
    }
    
    /// Parse a depth message from Bybit WebSocket
    pub fn parse_depth_message(message: &str) -> Result<OrderBook> {
        let data: BybitDepthMessage = serde_json::from_str(message)
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse depth message: {}", e)))?;
        
        let symbol = data.topic.split('.').nth(2).unwrap_or("").to_string();
        let mut orderbook = OrderBook::new(symbol, crate::connectors::Exchange::Bybit);
        
        // Update bids
        for bid in data.data.b {
            let price: f64 = bid[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid price: {}", e)))?;
            let quantity: f64 = bid[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid quantity: {}", e)))?;
            orderbook.update_bid(price, quantity);
        }
        
        // Update asks
        for ask in data.data.a {
            let price: f64 = ask[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask price: {}", e)))?;
            let quantity: f64 = ask[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask quantity: {}", e)))?;
            orderbook.update_ask(price, quantity);
        }
        
        orderbook.set_timestamp(data.ts * 1_000_000); // Convert to nanoseconds
        
        Ok(orderbook)
    }
    
    /// Parse a trade message from Bybit WebSocket
    pub fn parse_trade_message(message: &str) -> Result<(String, f64, f64, OrderSide, i64)> {
        let data: BybitTradeMessage = serde_json::from_str(message)
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse trade message: {}", e)))?;
        
        let symbol = data.topic.split('.').nth(1).unwrap_or("").to_string();
        
        // Bybit sends an array of trades
        if let Some(trade) = data.data.first() {
            let price: f64 = trade.price.parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid trade price: {}", e)))?;
            let quantity: f64 = trade.size.parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid trade quantity: {}", e)))?;
            let side = match trade.side.as_str() {
                "Buy" => OrderSide::Buy,
                "Sell" => OrderSide::Sell,
                _ => return Err(ArbitrageError::DataParsing(format!("Invalid trade side: {}", trade.side)).into()),
            };
            let timestamp = trade.timestamp;
            
            Ok((symbol, price, quantity, side, timestamp))
        } else {
            Err(ArbitrageError::DataParsing("Empty trade data".to_string()).into())
        }
    }
}

#[async_trait]
impl ExchangeConnector for BybitConnector {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Bybit...");
        self.connection_status = ConnectionStatus::Connecting;
        
        // Create WebSocket client
        let ws_url = if self.config.auth.testnet {
            self.config.auth.testnet_websocket_url.as_ref()
                .unwrap_or(&self.config.connection.websocket_url)
        } else {
            &self.config.connection.websocket_url
        };
        
        match BybitWebSocketClient::new(ws_url).await {
            Ok(client) => {
                self.websocket_client = Some(client);
                self.connection_status = ConnectionStatus::Connected;
                info!("Successfully connected to Bybit");
                Ok(())
            }
            Err(e) => {
                self.connection_status = ConnectionStatus::Error;
                error!("Failed to connect to Bybit: {}", e);
                Err(e)
            }
        }
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Bybit...");
        
        if let Some(mut client) = self.websocket_client.take() {
            client.disconnect().await?;
        }
        
        self.connection_status = ConnectionStatus::Disconnected;
        info!("Disconnected from Bybit");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }
    
    fn connection_status(&self) -> ConnectionStatus {
        self.connection_status
    }
    
    async fn subscribe_orderbook(&mut self, symbol: &str) -> Result<()> {
        debug!("Subscribing to orderbook for symbol: {}", symbol);
        
        if let Some(client) = &mut self.websocket_client {
            let topic = format!("orderbook.50.{}", symbol);
            client.subscribe(&topic).await?;
            info!("Subscribed to Bybit orderbook for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Bybit".to_string()).into())
        }
    }
    
    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()> {
        debug!("Subscribing to trades for symbol: {}", symbol);
        
        if let Some(client) = &mut self.websocket_client {
            let topic = format!("publicTrade.{}", symbol);
            client.subscribe(&topic).await?;
            info!("Subscribed to Bybit trades for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Bybit".to_string()).into())
        }
    }
    
    async fn subscribe_ticker(&mut self, symbol: &str) -> Result<()> {
        debug!("Subscribing to ticker for symbol: {}", symbol);
        
        if let Some(client) = &mut self.websocket_client {
            let topic = format!("tickers.{}", symbol);
            client.subscribe(&topic).await?;
            info!("Subscribed to Bybit ticker for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Bybit".to_string()).into())
        }
    }
    
    async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook> {
        debug!("Getting orderbook snapshot for symbol: {}", symbol);
        
        let snapshot = self.rest_client.get_orderbook_snapshot(symbol).await?;
        
        let mut orderbook = OrderBook::new(symbol.to_string(), crate::connectors::Exchange::Bybit);
        
        // Update bids
        for bid in snapshot.result.b {
            let price: f64 = bid[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid price: {}", e)))?;
            let quantity: f64 = bid[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid quantity: {}", e)))?;
            orderbook.update_bid(price, quantity);
        }
        
        // Update asks
        for ask in snapshot.result.a {
            let price: f64 = ask[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask price: {}", e)))?;
            let quantity: f64 = ask[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask quantity: {}", e)))?;
            orderbook.update_ask(price, quantity);
        }
        
        orderbook.set_timestamp(snapshot.time * 1_000_000); // Convert to nanoseconds
        
        Ok(orderbook)
    }
    
    async fn get_balances(&self) -> Result<HashMap<String, Balance>> {
        debug!("Getting account balances");
        
        let wallet_balance = self.rest_client.get_wallet_balance().await?;
        let mut balances = HashMap::new();
        
        for coin in wallet_balance.result.list {
            for balance in coin.coin {
                if balance.wallet_balance > 0.0 || balance.locked_balance > 0.0 {
                    balances.insert(balance.coin.clone(), Balance {
                        asset: balance.coin,
                        free: balance.wallet_balance - balance.locked_balance,
                        locked: balance.locked_balance,
                    });
                }
            }
        }
        
        Ok(balances)
    }
    
    async fn place_limit_order(&self, order: &LimitOrder) -> Result<OrderResponse> {
        debug!("Placing limit order: {:?}", order);
        
        let response = self.rest_client.place_order(order).await?;
        
        Ok(OrderResponse {
            order_id: response.result.order_id,
            client_order_id: Some(response.result.order_link_id),
            symbol: response.result.symbol,
            side: response.result.side,
            quantity: response.result.qty,
            price: response.result.price,
            status: response.result.order_status,
            filled_quantity: response.result.cum_exec_qty,
            average_price: if response.result.cum_exec_qty > 0.0 {
                Some(response.result.cum_exec_value / response.result.cum_exec_qty)
            } else {
                None
            },
            timestamp: response.result.created_time,
        })
    }
    
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse> {
        debug!("Cancelling order: {} for symbol: {}", order_id, symbol);
        
        let response = self.rest_client.cancel_order(symbol, order_id).await?;
        
        Ok(OrderResponse {
            order_id: response.result.order_id,
            client_order_id: Some(response.result.order_link_id),
            symbol: response.result.symbol,
            side: response.result.side,
            quantity: response.result.qty,
            price: response.result.price,
            status: response.result.order_status,
            filled_quantity: response.result.cum_exec_qty,
            average_price: if response.result.cum_exec_qty > 0.0 {
                Some(response.result.cum_exec_value / response.result.cum_exec_qty)
            } else {
                None
            },
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        })
    }
    
    async fn get_order_status(&self, symbol: &str, order_id: &str) -> Result<OrderStatus> {
        debug!("Getting order status: {} for symbol: {}", order_id, symbol);
        
        let order = self.rest_client.get_order(symbol, order_id).await?;
        Ok(order.result.order_status)
    }
    
    fn get_market_data_receiver(&self) -> Option<mpsc::Receiver<MarketDataUpdate>> {
        // This would return a receiver for market data updates
        // Implementation depends on how we structure the message handling
        None
    }
    
    fn get_order_update_receiver(&self) -> Option<mpsc::Receiver<OrderUpdate>> {
        // This would return a receiver for order updates
        // Implementation depends on how we structure the message handling
        None
    }
}

/// Bybit WebSocket client
struct BybitWebSocketClient {
    url: String,
    // WebSocket connection would be stored here
    // For now, we'll use a placeholder
}

impl BybitWebSocketClient {
    async fn new(url: &str) -> Result<Self> {
        // Validate URL
        Url::parse(url)
            .map_err(|e| ArbitrageError::Connection(format!("Invalid WebSocket URL: {}", e)))?;
        
        Ok(Self {
            url: url.to_string(),
        })
    }
    
    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        debug!("Subscribing to topic: {}", topic);
        // WebSocket subscription logic would go here
        // For now, we'll just log the subscription
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        debug!("Disconnecting WebSocket client");
        // WebSocket disconnection logic would go here
        Ok(())
    }
}

/// Bybit REST client
struct BybitRestClient {
    base_url: String,
    api_key: String,
    secret_key: String,
    client: reqwest::Client,
}

impl BybitRestClient {
    fn new(config: &ExchangeConfig) -> Result<Self> {
        let base_url = if config.auth.testnet {
            config.auth.testnet_rest_api_url.as_ref()
                .unwrap_or(&config.connection.rest_api_url)
        } else {
            &config.connection.rest_api_url
        };
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.connection.connection_timeout_secs))
            .build()
            .map_err(|e| ArbitrageError::Connection(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            base_url: base_url.clone(),
            api_key: config.auth.api_key.clone(),
            secret_key: config.auth.secret_key.clone(),
            client,
        })
    }
    
    async fn get_orderbook_snapshot(&self, symbol: &str) -> Result<BybitOrderBookSnapshot> {
        let url = format!("{}/v5/market/orderbook?category=spot&symbol={}&limit=50", self.base_url, symbol);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ArbitrageError::Connection(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(ArbitrageError::Connection(
                format!("HTTP request failed with status: {}", response.status())
            ).into());
        }
        
        let snapshot: BybitOrderBookSnapshot = response
            .json()
            .await
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse orderbook snapshot: {}", e)))?;
        
        Ok(snapshot)
    }
    
    async fn get_wallet_balance(&self) -> Result<BybitWalletBalance> {
        // This would implement signed request to get wallet balance
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Wallet balance requires valid API credentials".to_string()).into())
    }
    
    async fn place_order(&self, _order: &LimitOrder) -> Result<BybitOrderResponse> {
        // This would implement signed request to place an order
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order placement requires valid API credentials".to_string()).into())
    }
    
    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<BybitOrderResponse> {
        // This would implement signed request to cancel an order
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order cancellation requires valid API credentials".to_string()).into())
    }
    
    async fn get_order(&self, _symbol: &str, _order_id: &str) -> Result<BybitOrderResponse> {
        // This would implement signed request to get order status
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order status requires valid API credentials".to_string()).into())
    }
}

// Bybit API response types
#[derive(Debug, Deserialize)]
struct BybitDepthMessage {
    topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    ts: i64,
    data: BybitDepthData,
}

#[derive(Debug, Deserialize)]
struct BybitDepthData {
    s: String, // symbol
    b: Vec<[String; 2]>, // bids
    a: Vec<[String; 2]>, // asks
    u: u64, // update id
    seq: u64, // sequence
}

#[derive(Debug, Deserialize)]
struct BybitTradeMessage {
    topic: String,
    #[serde(rename = "type")]
    msg_type: String,
    ts: i64,
    data: Vec<BybitTradeData>,
}

#[derive(Debug, Deserialize)]
struct BybitTradeData {
    #[serde(rename = "T")]
    timestamp: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "S")]
    side: String,
    #[serde(rename = "v")]
    size: String,
    #[serde(rename = "p")]
    price: String,
}

#[derive(Debug, Deserialize)]
struct BybitOrderBookSnapshot {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitOrderBookResult,
    time: i64,
}

#[derive(Debug, Deserialize)]
struct BybitOrderBookResult {
    s: String, // symbol
    b: Vec<[String; 2]>, // bids
    a: Vec<[String; 2]>, // asks
    ts: i64, // timestamp
    u: u64, // update id
}

#[derive(Debug, Deserialize)]
struct BybitWalletBalance {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitWalletResult,
}

#[derive(Debug, Deserialize)]
struct BybitWalletResult {
    list: Vec<BybitAccountInfo>,
}

#[derive(Debug, Deserialize)]
struct BybitAccountInfo {
    #[serde(rename = "accountType")]
    account_type: String,
    coin: Vec<BybitBalance>,
}

#[derive(Debug, Deserialize)]
struct BybitBalance {
    coin: String,
    #[serde(rename = "walletBalance")]
    wallet_balance: f64,
    #[serde(rename = "transferBalance")]
    transfer_balance: f64,
    #[serde(rename = "bonus")]
    bonus: f64,
    #[serde(rename = "lockedBalance")]
    locked_balance: f64,
}

#[derive(Debug, Deserialize)]
struct BybitOrderResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: BybitOrderResult,
}

#[derive(Debug, Deserialize)]
struct BybitOrderResult {
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "orderLinkId")]
    order_link_id: String,
    symbol: String,
    side: OrderSide,
    qty: f64,
    price: f64,
    #[serde(rename = "orderStatus")]
    order_status: OrderStatus,
    #[serde(rename = "cumExecQty")]
    cum_exec_qty: f64,
    #[serde(rename = "cumExecValue")]
    cum_exec_value: f64,
    #[serde(rename = "createdTime")]
    created_time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_depth_message() {
        let message = r#"{"topic":"orderbook.1.BTCUSDT","type":"snapshot","ts":1234567890,"data":{"s":"BTCUSDT","b":[["50000.00","1.00"]],"a":[["50100.00","1.00"]],"u":123,"seq":456}}"#;
        
        let orderbook = BybitConnector::parse_depth_message(message).unwrap();
        assert_eq!(orderbook.symbol, "BTCUSDT");
        assert_eq!(orderbook.best_bid(), Some(50000.0));
        assert_eq!(orderbook.best_ask(), Some(50100.0));
    }

    #[test]
    fn test_parse_trade_message() {
        let message = r#"{"topic":"publicTrade.BTCUSDT","type":"snapshot","ts":1234567890,"data":[{"T":1234567890,"s":"BTCUSDT","S":"Buy","v":"0.10","p":"50050.00"}]}"#;
        
        let (symbol, price, quantity, side, timestamp) = BybitConnector::parse_trade_message(message).unwrap();
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(price, 50050.0);
        assert_eq!(quantity, 0.1);
        assert_eq!(side, OrderSide::Buy);
        assert_eq!(timestamp, 1234567890);
    }

    #[tokio::test]
    async fn test_bybit_connector_creation() {
        use crate::config::*;
        use std::collections::HashMap;
        
        let config = ExchangeConfig {
            connection: ConnectionConfig {
                websocket_url: "wss://stream.bybit.com/v5/public/spot".to_string(),
                rest_api_url: "https://api.bybit.com".to_string(),
                connection_timeout_secs: 10,
                max_reconnect_attempts: 5,
                reconnect_delay_secs: 5,
            },
            auth: AuthConfig {
                api_key: "test_key".to_string(),
                secret_key: "test_secret".to_string(),
                testnet: false,
                testnet_websocket_url: None,
                testnet_rest_api_url: None,
            },
            trading: TradingConfig {
                default_order_type: "Limit".to_string(),
                default_time_in_force: "GTC".to_string(),
                additional: HashMap::new(),
            },
            fees: FeeConfig {
                maker_fee: 0.001,
                taker_fee: 0.001,
                fee_currency: "USDT".to_string(),
                additional: HashMap::new(),
            },
            limits: LimitsConfig {
                order_rate_limit: 600,
                market_data_rate_limit: 6000,
                min_order_sizes: HashMap::new(),
                tick_sizes: HashMap::new(),
            },
            market_data: MarketDataConfig {
                streams: vec![],
                topics: vec![],
                depth_levels: 50,
                additional: HashMap::new(),
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_interval_secs: 60,
                enable_trade_logging: true,
                log_rotation_size_mb: 100,
                health_check_interval_secs: 30,
            },
        };
        
        let connector = BybitConnector::new(config).await.unwrap();
        assert!(!connector.is_connected());
        assert_eq!(connector.connection_status(), ConnectionStatus::Disconnected);
    }
}
