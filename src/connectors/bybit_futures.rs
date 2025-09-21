//! Bybit Futures connector implementation

use super::futures::*;
use crate::{connectors::Exchange, data::OrderBook, Result, ArbitrageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Bybit Futures connector
pub struct BybitFuturesConnector {
    /// API base URL
    api_url: String,
    /// WebSocket URL
    ws_url: String,
    /// API key
    api_key: Option<String>,
    /// Secret key
    secret_key: Option<String>,
    /// WebSocket connection
    ws_connection: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    /// Connection status
    is_connected: bool,
    /// Subscribed symbols
    subscribed_symbols: Vec<String>,
}

impl BybitFuturesConnector {
    /// Create new Bybit Futures connector
    pub fn new(api_key: Option<String>, secret_key: Option<String>) -> Self {
        Self {
            api_url: "https://api.bybit.com".to_string(),
            ws_url: "wss://stream.bybit.com/v5/public/linear".to_string(),
            api_key,
            secret_key,
            ws_connection: None,
            is_connected: false,
            subscribed_symbols: Vec::new(),
        }
    }

    /// Get common USDT perpetual symbols (same as Binance)
    pub async fn get_common_usdt_perpetuals(&self) -> Result<Vec<String>> {
        // Common USDT perpetual contracts available on both Binance and Bybit
        Ok(vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "ADAUSDT".to_string(),
            "BNBUSDT".to_string(),
            "XRPUSDT".to_string(),
            "SOLUSDT".to_string(),
            "DOTUSDT".to_string(),
            "DOGEUSDT".to_string(),
            "AVAXUSDT".to_string(),
            "MATICUSDT".to_string(),
            "LINKUSDT".to_string(),
            "LTCUSDT".to_string(),
            "UNIUSDT".to_string(),
            "ATOMUSDT".to_string(),
            "FILUSDT".to_string(),
            "TRXUSDT".to_string(),
            "ETCUSDT".to_string(),
            "XLMUSDT".to_string(),
            "VETUSDT".to_string(),
            "ICPUSDT".to_string(),
        ])
    }

    /// Parse depth message from WebSocket
    fn parse_depth_message(&self, msg: &str) -> Result<OrderBook> {
        let depth_msg: BybitFuturesMessage = serde_json::from_str(msg)
            .map_err(|e| ArbitrageError::ParseError(format!("Failed to parse Bybit futures depth: {}", e)))?;

        if let Some(data) = depth_msg.data {
            if let Some(symbol) = data.symbol {
                let mut orderbook = OrderBook::new(symbol, Exchange::Bybit);
                
                // Update bids
                if let Some(bids) = data.bids {
                    for bid in bids {
                        if bid.len() >= 2 {
                            let price = bid[0].parse::<f64>()
                                .map_err(|e| ArbitrageError::ParseError(format!("Invalid bid price: {}", e)))?;
                            let quantity = bid[1].parse::<f64>()
                                .map_err(|e| ArbitrageError::ParseError(format!("Invalid bid quantity: {}", e)))?;
                            orderbook.update_bid(price, quantity);
                        }
                    }
                }

                // Update asks
                if let Some(asks) = data.asks {
                    for ask in asks {
                        if ask.len() >= 2 {
                            let price = ask[0].parse::<f64>()
                                .map_err(|e| ArbitrageError::ParseError(format!("Invalid ask price: {}", e)))?;
                            let quantity = ask[1].parse::<f64>()
                                .map_err(|e| ArbitrageError::ParseError(format!("Invalid ask quantity: {}", e)))?;
                            orderbook.update_ask(price, quantity);
                        }
                    }
                }

                orderbook.set_timestamp(data.ts.unwrap_or(chrono::Utc::now().timestamp_millis()));
                return Ok(orderbook);
            }
        }

        Err(ArbitrageError::ParseError("Invalid depth message format".to_string()).into())
    }

    /// Parse mark price message
    fn parse_mark_price_message(&self, msg: &str) -> Result<MarkPrice> {
        let mark_msg: BybitFuturesMessage = serde_json::from_str(msg)
            .map_err(|e| ArbitrageError::ParseError(format!("Failed to parse mark price: {}", e)))?;

        if let Some(data) = mark_msg.data {
            if let Some(symbol) = data.symbol {
                return Ok(MarkPrice {
                    symbol,
                    mark_price: data.mark_price.and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    index_price: data.index_price.and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    estimated_settle_price: None,
                    last_funding_rate: data.funding_rate.and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    next_funding_time: data.next_funding_time.unwrap_or(0),
                    interest_rate: 0.0,
                    timestamp: data.ts.unwrap_or(chrono::Utc::now().timestamp_millis()),
                });
            }
        }

        Err(ArbitrageError::ParseError("Invalid mark price message format".to_string()).into())
    }

    /// Create subscription message for WebSocket
    fn create_subscription_message(&self, symbols: &[String]) -> String {
        let mut topics = Vec::new();
        
        for symbol in symbols {
            // Order book depth
            topics.push(format!("orderbook.50.{}", symbol));
            // Mark price
            topics.push(format!("tickers.{}", symbol));
        }
        
        serde_json::json!({
            "op": "subscribe",
            "args": topics
        }).to_string()
    }
}

#[async_trait::async_trait]
impl FuturesConnector for BybitFuturesConnector {
    async fn get_exchange_info(&self) -> Result<HashMap<String, FuturesContract>> {
        // In a real implementation, this would fetch from Bybit API
        // For now, return mock data for common symbols
        let symbols = self.get_common_usdt_perpetuals().await?;
        let mut contracts = HashMap::new();
        
        for symbol in symbols {
            contracts.insert(symbol.clone(), FuturesContract {
                symbol: symbol.clone(),
                base_asset: symbol.replace("USDT", ""),
                quote_asset: "USDT".to_string(),
                contract_type: ContractType::Perpetual,
                min_order_size: 0.001,
                price_precision: 2,
                quantity_precision: 3,
                tick_size: 0.01,
                lot_size: 0.001,
                maker_fee: -0.00025, // -0.025% (rebate)
                taker_fee: 0.00075,  // 0.075%
            });
        }
        
        Ok(contracts)
    }

    async fn get_account_info(&self) -> Result<FuturesAccountInfo> {
        // Mock implementation - in production this would call Bybit API
        Err(ArbitrageError::NotImplemented("get_account_info requires API keys".to_string()).into())
    }

    async fn get_positions(&self) -> Result<Vec<FuturesPosition>> {
        // Mock implementation - in production this would call Bybit API
        Err(ArbitrageError::NotImplemented("get_positions requires API keys".to_string()).into())
    }

    async fn place_order(&self, _order: &FuturesOrder) -> Result<FuturesOrderResponse> {
        // Mock implementation - in production this would call Bybit API
        Err(ArbitrageError::NotImplemented("place_order requires API keys".to_string()).into())
    }

    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<FuturesOrderResponse> {
        // Mock implementation
        Err(ArbitrageError::NotImplemented("cancel_order requires API keys".to_string()).into())
    }

    async fn get_order_status(&self, _symbol: &str, _order_id: &str) -> Result<FuturesOrderResponse> {
        // Mock implementation
        Err(ArbitrageError::NotImplemented("get_order_status requires API keys".to_string()).into())
    }

    async fn get_funding_rate(&self, _symbol: &str) -> Result<FundingRate> {
        // Mock implementation - in production this would call Bybit API
        Err(ArbitrageError::NotImplemented("get_funding_rate not implemented".to_string()).into())
    }

    async fn get_mark_price(&self, _symbol: &str) -> Result<MarkPrice> {
        // Mock implementation - in production this would call Bybit API
        Err(ArbitrageError::NotImplemented("get_mark_price not implemented".to_string()).into())
    }

    async fn subscribe_orderbook(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Bybit futures orderbook for {}", symbol);
        if !self.subscribed_symbols.contains(&symbol.to_string()) {
            self.subscribed_symbols.push(symbol.to_string());
        }
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Bybit futures trades for {}", symbol);
        Ok(())
    }

    async fn subscribe_mark_price(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Bybit futures mark price for {}", symbol);
        Ok(())
    }

    async fn subscribe_funding_rate(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Bybit futures funding rate for {}", symbol);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> Result<()> {
        if self.subscribed_symbols.is_empty() {
            return Err(ArbitrageError::Connection("No symbols subscribed".to_string()).into());
        }

        info!("Connecting to Bybit futures WebSocket: {}", self.ws_url);

        match connect_async(&self.ws_url).await {
            Ok((mut ws_stream, _)) => {
                // Send subscription message
                let subscription = self.create_subscription_message(&self.subscribed_symbols);
                info!("Sending subscription: {}", subscription);
                
                if let Err(e) = ws_stream.send(Message::Text(subscription)).await {
                    error!("Failed to send subscription message: {}", e);
                    return Err(ArbitrageError::Connection(format!("Subscription failed: {}", e)).into());
                }

                self.ws_connection = Some(ws_stream);
                self.is_connected = true;
                info!("Successfully connected to Bybit futures WebSocket");
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to Bybit futures WebSocket: {}", e);
                Err(ArbitrageError::Connection(format!("WebSocket connection failed: {}", e)).into())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut ws) = self.ws_connection.take() {
            let _ = ws.close(None).await;
        }
        self.is_connected = false;
        info!("Disconnected from Bybit futures WebSocket");
        Ok(())
    }
}

impl BybitFuturesConnector {
    /// Start receiving messages (for testing/demo purposes)
    pub async fn start_message_loop(&mut self) -> Result<()> {
        let ws = self.ws_connection.take();
        if let Some(mut ws_stream) = ws {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = Self::process_message_static(&text).await {
                            warn!("Failed to process message: {}", e);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            self.ws_connection = Some(ws_stream);
        }
        Ok(())
    }

    /// Static message processing (to avoid borrowing issues)
    async fn process_message_static(msg: &str) -> Result<()> {
        debug!("Received Bybit futures message: {}", msg);
        // Message processing logic would go here
        Ok(())
    }

    /// Process incoming WebSocket message
    async fn process_message(&self, msg: &str) -> Result<()> {
        debug!("Received Bybit futures message: {}", msg);
        
        // Try to parse as different message types
        if msg.contains("orderbook") {
            match self.parse_depth_message(msg) {
                Ok(orderbook) => {
                    debug!("Parsed orderbook for {}: best_bid={:?}, best_ask={:?}", 
                           orderbook.symbol, orderbook.best_bid(), orderbook.best_ask());
                }
                Err(e) => warn!("Failed to parse depth message: {}", e),
            }
        } else if msg.contains("tickers") {
            match self.parse_mark_price_message(msg) {
                Ok(mark_price) => {
                    debug!("Parsed mark price for {}: {}", 
                           mark_price.symbol, mark_price.mark_price);
                }
                Err(e) => warn!("Failed to parse mark price message: {}", e),
            }
        }
        
        Ok(())
    }
}

// WebSocket message structures
#[derive(Debug, Deserialize)]
struct BybitFuturesMessage {
    topic: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    data: Option<BybitFuturesData>,
    ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BybitFuturesData {
    #[serde(rename = "s")]
    symbol: Option<String>,
    #[serde(rename = "b")]
    bids: Option<Vec<Vec<String>>>,
    #[serde(rename = "a")]
    asks: Option<Vec<Vec<String>>>,
    #[serde(rename = "u")]
    update_id: Option<u64>,
    ts: Option<i64>,
    // Mark price fields
    #[serde(rename = "markPrice")]
    mark_price: Option<String>,
    #[serde(rename = "indexPrice")]
    index_price: Option<String>,
    #[serde(rename = "fundingRate")]
    funding_rate: Option<String>,
    #[serde(rename = "nextFundingTime")]
    next_funding_time: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bybit_futures_connector_creation() {
        let connector = BybitFuturesConnector::new(None, None);
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_get_common_symbols() {
        let connector = BybitFuturesConnector::new(None, None);
        let symbols = connector.get_common_usdt_perpetuals().await.unwrap();
        assert!(symbols.contains(&"BTCUSDT".to_string()));
        assert!(symbols.contains(&"ETHUSDT".to_string()));
        assert!(symbols.len() >= 10);
    }

    #[tokio::test]
    async fn test_exchange_info() {
        let connector = BybitFuturesConnector::new(None, None);
        let contracts = connector.get_exchange_info().await.unwrap();
        assert!(contracts.contains_key("BTCUSDT"));
        
        let btc_contract = &contracts["BTCUSDT"];
        assert_eq!(btc_contract.contract_type, ContractType::Perpetual);
        assert_eq!(btc_contract.quote_asset, "USDT");
        // Bybit has maker rebate
        assert!(btc_contract.maker_fee < 0.0);
    }

    #[test]
    fn test_subscription_message() {
        let connector = BybitFuturesConnector::new(None, None);
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let msg = connector.create_subscription_message(&symbols);
        
        assert!(msg.contains("subscribe"));
        assert!(msg.contains("orderbook.50.BTCUSDT"));
        assert!(msg.contains("tickers.BTCUSDT"));
    }
}
