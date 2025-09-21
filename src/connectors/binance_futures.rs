//! Binance Futures connector implementation

use super::futures::*;
use crate::{connectors::Exchange, data::OrderBook, Result, ArbitrageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Binance Futures connector
pub struct BinanceFuturesConnector {
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

impl BinanceFuturesConnector {
    /// Create new Binance Futures connector
    pub fn new(api_key: Option<String>, secret_key: Option<String>) -> Self {
        Self {
            api_url: "https://fapi.binance.com".to_string(),
            ws_url: "wss://fstream.binance.com/ws/".to_string(),
            api_key,
            secret_key,
            ws_connection: None,
            is_connected: false,
            subscribed_symbols: Vec::new(),
        }
    }

    /// Get common USDT perpetual symbols
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
        let depth_msg: BinanceFuturesDepthMessage = serde_json::from_str(msg)
            .map_err(|e| ArbitrageError::ParseError(format!("Failed to parse Binance futures depth: {}", e)))?;

        let mut orderbook = OrderBook::new(depth_msg.data.symbol.clone(), Exchange::Binance);
        
        // Update bids
        for bid in depth_msg.data.bids {
            let price = bid[0].parse::<f64>()
                .map_err(|e| ArbitrageError::ParseError(format!("Invalid bid price: {}", e)))?;
            let quantity = bid[1].parse::<f64>()
                .map_err(|e| ArbitrageError::ParseError(format!("Invalid bid quantity: {}", e)))?;
            orderbook.update_bid(price, quantity);
        }

        // Update asks
        for ask in depth_msg.data.asks {
            let price = ask[0].parse::<f64>()
                .map_err(|e| ArbitrageError::ParseError(format!("Invalid ask price: {}", e)))?;
            let quantity = ask[1].parse::<f64>()
                .map_err(|e| ArbitrageError::ParseError(format!("Invalid ask quantity: {}", e)))?;
            orderbook.update_ask(price, quantity);
        }

        orderbook.set_timestamp(depth_msg.data.event_time);
        Ok(orderbook)
    }

    /// Parse mark price message
    fn parse_mark_price_message(&self, msg: &str) -> Result<MarkPrice> {
        let mark_msg: BinanceFuturesMarkPriceMessage = serde_json::from_str(msg)
            .map_err(|e| ArbitrageError::ParseError(format!("Failed to parse mark price: {}", e)))?;

        Ok(MarkPrice {
            symbol: mark_msg.data.symbol,
            mark_price: mark_msg.data.mark_price.parse().unwrap_or(0.0),
            index_price: mark_msg.data.index_price.parse().unwrap_or(0.0),
            estimated_settle_price: mark_msg.data.estimated_settle_price.parse().ok(),
            last_funding_rate: mark_msg.data.funding_rate.parse().unwrap_or(0.0),
            next_funding_time: mark_msg.data.next_funding_time,
            interest_rate: mark_msg.data.interest_rate.parse().unwrap_or(0.0),
            timestamp: mark_msg.data.event_time,
        })
    }

    /// Create WebSocket URL for multiple streams
    fn create_stream_url(&self, symbols: &[String]) -> String {
        let mut streams = Vec::new();
        
        for symbol in symbols {
            let symbol_lower = symbol.to_lowercase();
            // Order book depth stream
            streams.push(format!("{}@depth20@100ms", symbol_lower));
            // Mark price stream
            streams.push(format!("{}@markPrice", symbol_lower));
            // 24hr ticker stream for additional data
            streams.push(format!("{}@ticker", symbol_lower));
        }
        
        format!("{}{}", self.ws_url, streams.join("/"))
    }
}

#[async_trait::async_trait]
impl FuturesConnector for BinanceFuturesConnector {
    async fn get_exchange_info(&self) -> Result<HashMap<String, FuturesContract>> {
        // In a real implementation, this would fetch from Binance API
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
                maker_fee: 0.0002, // 0.02%
                taker_fee: 0.0004, // 0.04%
            });
        }
        
        Ok(contracts)
    }

    async fn get_account_info(&self) -> Result<FuturesAccountInfo> {
        // Mock implementation - in production this would call Binance API
        Err(ArbitrageError::NotImplemented("get_account_info requires API keys".to_string()).into())
    }

    async fn get_positions(&self) -> Result<Vec<FuturesPosition>> {
        // Mock implementation - in production this would call Binance API
        Err(ArbitrageError::NotImplemented("get_positions requires API keys".to_string()).into())
    }

    async fn place_order(&self, _order: &FuturesOrder) -> Result<FuturesOrderResponse> {
        // Mock implementation - in production this would call Binance API
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
        // Mock implementation - in production this would call Binance API
        Err(ArbitrageError::NotImplemented("get_funding_rate not implemented".to_string()).into())
    }

    async fn get_mark_price(&self, _symbol: &str) -> Result<MarkPrice> {
        // Mock implementation - in production this would call Binance API
        Err(ArbitrageError::NotImplemented("get_mark_price not implemented".to_string()).into())
    }

    async fn subscribe_orderbook(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Binance futures orderbook for {}", symbol);
        if !self.subscribed_symbols.contains(&symbol.to_string()) {
            self.subscribed_symbols.push(symbol.to_string());
        }
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Binance futures trades for {}", symbol);
        Ok(())
    }

    async fn subscribe_mark_price(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Binance futures mark price for {}", symbol);
        Ok(())
    }

    async fn subscribe_funding_rate(&mut self, symbol: &str) -> Result<()> {
        info!("Subscribing to Binance futures funding rate for {}", symbol);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> Result<()> {
        if self.subscribed_symbols.is_empty() {
            return Err(ArbitrageError::Connection("No symbols subscribed".to_string()).into());
        }

        let url = self.create_stream_url(&self.subscribed_symbols);
        info!("Connecting to Binance futures WebSocket: {}", url);

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                self.ws_connection = Some(ws_stream);
                self.is_connected = true;
                info!("Successfully connected to Binance futures WebSocket");
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to Binance futures WebSocket: {}", e);
                Err(ArbitrageError::Connection(format!("WebSocket connection failed: {}", e)).into())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut ws) = self.ws_connection.take() {
            let _ = ws.close(None).await;
        }
        self.is_connected = false;
        info!("Disconnected from Binance futures WebSocket");
        Ok(())
    }
}

impl BinanceFuturesConnector {
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
        debug!("Received Binance futures message: {}", msg);
        // Message processing logic would go here
        Ok(())
    }

    /// Process incoming WebSocket message
    async fn process_message(&self, msg: &str) -> Result<()> {
        debug!("Received Binance futures message: {}", msg);
        
        // Try to parse as different message types
        if msg.contains("depthUpdate") {
            match self.parse_depth_message(msg) {
                Ok(orderbook) => {
                    debug!("Parsed orderbook for {}: best_bid={:?}, best_ask={:?}", 
                           orderbook.symbol, orderbook.best_bid(), orderbook.best_ask());
                }
                Err(e) => warn!("Failed to parse depth message: {}", e),
            }
        } else if msg.contains("markPrice") {
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
struct BinanceFuturesDepthMessage {
    stream: String,
    data: BinanceFuturesDepthData,
}

#[derive(Debug, Deserialize)]
struct BinanceFuturesDepthData {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "T")]
    transaction_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "pu")]
    prev_final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct BinanceFuturesMarkPriceMessage {
    stream: String,
    data: BinanceFuturesMarkPriceData,
}

#[derive(Debug, Deserialize)]
struct BinanceFuturesMarkPriceData {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    mark_price: String,
    #[serde(rename = "i")]
    index_price: String,
    #[serde(rename = "P")]
    estimated_settle_price: String,
    #[serde(rename = "r")]
    funding_rate: String,
    #[serde(rename = "T")]
    next_funding_time: i64,
    #[serde(rename = "d")]
    interest_rate: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binance_futures_connector_creation() {
        let connector = BinanceFuturesConnector::new(None, None);
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_get_common_symbols() {
        let connector = BinanceFuturesConnector::new(None, None);
        let symbols = connector.get_common_usdt_perpetuals().await.unwrap();
        assert!(symbols.contains(&"BTCUSDT".to_string()));
        assert!(symbols.contains(&"ETHUSDT".to_string()));
        assert!(symbols.len() >= 10);
    }

    #[tokio::test]
    async fn test_exchange_info() {
        let connector = BinanceFuturesConnector::new(None, None);
        let contracts = connector.get_exchange_info().await.unwrap();
        assert!(contracts.contains_key("BTCUSDT"));
        
        let btc_contract = &contracts["BTCUSDT"];
        assert_eq!(btc_contract.contract_type, ContractType::Perpetual);
        assert_eq!(btc_contract.quote_asset, "USDT");
    }
}
