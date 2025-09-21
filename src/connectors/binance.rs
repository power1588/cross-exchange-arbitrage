//! Binance exchange connector implementation

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

/// Binance exchange connector
pub struct BinanceConnector {
    config: ExchangeConfig,
    connection_status: ConnectionStatus,
    market_data_tx: Option<mpsc::Sender<MarketDataUpdate>>,
    order_update_tx: Option<mpsc::Sender<OrderUpdate>>,
    websocket_client: Option<BinanceWebSocketClient>,
    rest_client: BinanceRestClient,
}

impl BinanceConnector {
    /// Create a new Binance connector
    pub async fn new(config: ExchangeConfig) -> Result<Self> {
        let rest_client = BinanceRestClient::new(&config)?;
        
        Ok(Self {
            config,
            connection_status: ConnectionStatus::Disconnected,
            market_data_tx: None,
            order_update_tx: None,
            websocket_client: None,
            rest_client,
        })
    }
    
    /// Parse a depth message from Binance WebSocket
    pub fn parse_depth_message(message: &str) -> Result<OrderBook> {
        let data: BinanceDepthMessage = serde_json::from_str(message)
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse depth message: {}", e)))?;
        
        let mut orderbook = OrderBook::new(
            data.stream.split('@').next().unwrap_or("").to_uppercase(),
            crate::connectors::Exchange::Binance,
        );
        
        // Update bids
        for bid in data.data.bids {
            let price: f64 = bid[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid price: {}", e)))?;
            let quantity: f64 = bid[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid quantity: {}", e)))?;
            orderbook.update_bid(price, quantity);
        }
        
        // Update asks
        for ask in data.data.asks {
            let price: f64 = ask[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask price: {}", e)))?;
            let quantity: f64 = ask[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask quantity: {}", e)))?;
            orderbook.update_ask(price, quantity);
        }
        
        orderbook.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        Ok(orderbook)
    }
    
    /// Parse a trade message from Binance WebSocket
    pub fn parse_trade_message(message: &str) -> Result<(String, f64, f64, OrderSide, i64)> {
        let data: BinanceTradeMessage = serde_json::from_str(message)
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse trade message: {}", e)))?;
        
        let symbol = data.stream.split('@').next().unwrap_or("").to_uppercase();
        let price: f64 = data.data.price.parse()
            .map_err(|e| ArbitrageError::DataParsing(format!("Invalid trade price: {}", e)))?;
        let quantity: f64 = data.data.quantity.parse()
            .map_err(|e| ArbitrageError::DataParsing(format!("Invalid trade quantity: {}", e)))?;
        let side = if data.data.is_buyer_maker { OrderSide::Sell } else { OrderSide::Buy };
        let timestamp = data.data.trade_time;
        
        Ok((symbol, price, quantity, side, timestamp))
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Binance...");
        self.connection_status = ConnectionStatus::Connecting;
        
        // Create WebSocket client
        let ws_url = if self.config.auth.testnet {
            self.config.auth.testnet_websocket_url.as_ref()
                .unwrap_or(&self.config.connection.websocket_url)
        } else {
            &self.config.connection.websocket_url
        };
        
        match BinanceWebSocketClient::new(ws_url).await {
            Ok(client) => {
                self.websocket_client = Some(client);
                self.connection_status = ConnectionStatus::Connected;
                info!("Successfully connected to Binance");
                Ok(())
            }
            Err(e) => {
                self.connection_status = ConnectionStatus::Error;
                error!("Failed to connect to Binance: {}", e);
                Err(e)
            }
        }
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Binance...");
        
        if let Some(mut client) = self.websocket_client.take() {
            client.disconnect().await?;
        }
        
        self.connection_status = ConnectionStatus::Disconnected;
        info!("Disconnected from Binance");
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
            let stream = format!("{}@depth@100ms", symbol.to_lowercase());
            client.subscribe(&stream).await?;
            info!("Subscribed to Binance orderbook for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Binance".to_string()).into())
        }
    }
    
    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()> {
        debug!("Subscribing to trades for symbol: {}", symbol);
        
        if let Some(client) = &mut self.websocket_client {
            let stream = format!("{}@trade", symbol.to_lowercase());
            client.subscribe(&stream).await?;
            info!("Subscribed to Binance trades for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Binance".to_string()).into())
        }
    }
    
    async fn subscribe_ticker(&mut self, symbol: &str) -> Result<()> {
        debug!("Subscribing to ticker for symbol: {}", symbol);
        
        if let Some(client) = &mut self.websocket_client {
            let stream = format!("{}@ticker", symbol.to_lowercase());
            client.subscribe(&stream).await?;
            info!("Subscribed to Binance ticker for {}", symbol);
            Ok(())
        } else {
            Err(ArbitrageError::Connection("Not connected to Binance".to_string()).into())
        }
    }
    
    async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook> {
        debug!("Getting orderbook snapshot for symbol: {}", symbol);
        
        let snapshot = self.rest_client.get_orderbook_snapshot(symbol).await?;
        
        let mut orderbook = OrderBook::new(symbol.to_string(), crate::connectors::Exchange::Binance);
        
        // Update bids
        for bid in snapshot.bids {
            let price: f64 = bid[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid price: {}", e)))?;
            let quantity: f64 = bid[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid bid quantity: {}", e)))?;
            orderbook.update_bid(price, quantity);
        }
        
        // Update asks
        for ask in snapshot.asks {
            let price: f64 = ask[0].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask price: {}", e)))?;
            let quantity: f64 = ask[1].parse()
                .map_err(|e| ArbitrageError::DataParsing(format!("Invalid ask quantity: {}", e)))?;
            orderbook.update_ask(price, quantity);
        }
        
        orderbook.set_timestamp(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        Ok(orderbook)
    }
    
    async fn get_balances(&self) -> Result<HashMap<String, Balance>> {
        debug!("Getting account balances");
        
        let account_info = self.rest_client.get_account_info().await?;
        let mut balances = HashMap::new();
        
        for balance in account_info.balances {
            if balance.free > 0.0 || balance.locked > 0.0 {
                balances.insert(balance.asset.clone(), Balance {
                    asset: balance.asset,
                    free: balance.free,
                    locked: balance.locked,
                });
            }
        }
        
        Ok(balances)
    }
    
    async fn place_limit_order(&self, order: &LimitOrder) -> Result<OrderResponse> {
        debug!("Placing limit order: {:?}", order);
        
        let response = self.rest_client.place_order(order).await?;
        
        Ok(OrderResponse {
            order_id: response.order_id.to_string(),
            client_order_id: Some(response.client_order_id),
            symbol: response.symbol,
            side: response.side,
            quantity: response.orig_qty,
            price: response.price,
            status: response.status,
            filled_quantity: response.executed_qty,
            average_price: if response.executed_qty > 0.0 {
                Some(response.cummulative_quote_qty / response.executed_qty)
            } else {
                None
            },
            timestamp: response.transact_time,
        })
    }
    
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse> {
        debug!("Cancelling order: {} for symbol: {}", order_id, symbol);
        
        let response = self.rest_client.cancel_order(symbol, order_id).await?;
        
        Ok(OrderResponse {
            order_id: response.order_id.to_string(),
            client_order_id: Some(response.client_order_id),
            symbol: response.symbol,
            side: response.side,
            quantity: response.orig_qty,
            price: response.price,
            status: response.status,
            filled_quantity: response.executed_qty,
            average_price: if response.executed_qty > 0.0 {
                Some(response.cummulative_quote_qty / response.executed_qty)
            } else {
                None
            },
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        })
    }
    
    async fn get_order_status(&self, symbol: &str, order_id: &str) -> Result<OrderStatus> {
        debug!("Getting order status: {} for symbol: {}", order_id, symbol);
        
        let order = self.rest_client.get_order(symbol, order_id).await?;
        Ok(order.status)
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

/// Binance WebSocket client
struct BinanceWebSocketClient {
    url: String,
    // WebSocket connection would be stored here
    // For now, we'll use a placeholder
}

impl BinanceWebSocketClient {
    async fn new(url: &str) -> Result<Self> {
        // Validate URL
        Url::parse(url)
            .map_err(|e| ArbitrageError::Connection(format!("Invalid WebSocket URL: {}", e)))?;
        
        Ok(Self {
            url: url.to_string(),
        })
    }
    
    async fn subscribe(&mut self, stream: &str) -> Result<()> {
        debug!("Subscribing to stream: {}", stream);
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

/// Binance REST client
struct BinanceRestClient {
    base_url: String,
    api_key: String,
    secret_key: String,
    client: reqwest::Client,
}

impl BinanceRestClient {
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
    
    async fn get_orderbook_snapshot(&self, symbol: &str) -> Result<BinanceOrderBookSnapshot> {
        let url = format!("{}/api/v3/depth?symbol={}&limit=100", self.base_url, symbol);
        
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
        
        let snapshot: BinanceOrderBookSnapshot = response
            .json()
            .await
            .map_err(|e| ArbitrageError::DataParsing(format!("Failed to parse orderbook snapshot: {}", e)))?;
        
        Ok(snapshot)
    }
    
    async fn get_account_info(&self) -> Result<BinanceAccountInfo> {
        // This would implement signed request to get account info
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Account info requires valid API credentials".to_string()).into())
    }
    
    async fn place_order(&self, _order: &LimitOrder) -> Result<BinanceOrderResponse> {
        // This would implement signed request to place an order
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order placement requires valid API credentials".to_string()).into())
    }
    
    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<BinanceOrderResponse> {
        // This would implement signed request to cancel an order
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order cancellation requires valid API credentials".to_string()).into())
    }
    
    async fn get_order(&self, _symbol: &str, _order_id: &str) -> Result<BinanceOrderResponse> {
        // This would implement signed request to get order status
        // For now, return an error since we don't have real API keys in tests
        Err(ArbitrageError::Connection("Order status requires valid API credentials".to_string()).into())
    }
}

// Binance API response types
#[derive(Debug, Deserialize)]
struct BinanceDepthMessage {
    stream: String,
    data: BinanceDepthData,
}

#[derive(Debug, Deserialize)]
struct BinanceDepthData {
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct BinanceTradeMessage {
    stream: String,
    data: BinanceTradeData,
}

#[derive(Debug, Deserialize)]
struct BinanceTradeData {
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "T")]
    trade_time: i64,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}

#[derive(Debug, Deserialize)]
struct BinanceOrderBookSnapshot {
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct BinanceAccountInfo {
    balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
struct BinanceBalance {
    asset: String,
    free: f64,
    locked: f64,
}

#[derive(Debug, Deserialize)]
struct BinanceOrderResponse {
    #[serde(rename = "orderId")]
    order_id: u64,
    #[serde(rename = "clientOrderId")]
    client_order_id: String,
    symbol: String,
    side: OrderSide,
    #[serde(rename = "origQty")]
    orig_qty: f64,
    price: f64,
    status: OrderStatus,
    #[serde(rename = "executedQty")]
    executed_qty: f64,
    #[serde(rename = "cummulativeQuoteQty")]
    cummulative_quote_qty: f64,
    #[serde(rename = "transactTime")]
    transact_time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_depth_message() {
        let message = r#"{"stream":"btcusdt@depth","data":{"b":[["50000.00","1.00000000"]],"a":[["50100.00","1.00000000"]]}}"#;
        
        let orderbook = BinanceConnector::parse_depth_message(message).unwrap();
        assert_eq!(orderbook.symbol, "BTCUSDT");
        assert_eq!(orderbook.best_bid(), Some(50000.0));
        assert_eq!(orderbook.best_ask(), Some(50100.0));
    }

    #[test]
    fn test_parse_trade_message() {
        let message = r#"{"stream":"btcusdt@trade","data":{"p":"50050.00","q":"0.10000000","T":1234567890,"m":false}}"#;
        
        let (symbol, price, quantity, side, timestamp) = BinanceConnector::parse_trade_message(message).unwrap();
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(price, 50050.0);
        assert_eq!(quantity, 0.1);
        assert_eq!(side, OrderSide::Buy);
        assert_eq!(timestamp, 1234567890);
    }

    #[tokio::test]
    async fn test_binance_connector_creation() {
        use crate::config::*;
        use std::collections::HashMap;
        
        let config = ExchangeConfig {
            connection: ConnectionConfig {
                websocket_url: "wss://stream.binance.com:9443/ws".to_string(),
                rest_api_url: "https://api.binance.com".to_string(),
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
                default_order_type: "LIMIT".to_string(),
                default_time_in_force: "GTC".to_string(),
                additional: HashMap::new(),
            },
            fees: FeeConfig {
                maker_fee: 0.001,
                taker_fee: 0.001,
                fee_currency: "BNB".to_string(),
                additional: HashMap::new(),
            },
            limits: LimitsConfig {
                order_rate_limit: 1200,
                market_data_rate_limit: 6000,
                min_order_sizes: HashMap::new(),
                tick_sizes: HashMap::new(),
            },
            market_data: MarketDataConfig {
                streams: vec![],
                topics: vec![],
                depth_levels: 20,
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
        
        let connector = BinanceConnector::new(config).await.unwrap();
        assert!(!connector.is_connected());
        assert_eq!(connector.connection_status(), ConnectionStatus::Disconnected);
    }
}
