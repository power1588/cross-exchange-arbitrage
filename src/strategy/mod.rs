//! Arbitrage strategy implementation

pub mod arbitrage;
pub mod futures_arbitrage;
// pub mod risk_manager; // Will be implemented later
// pub mod position_manager; // Will be implemented later

pub use arbitrage::{
    ArbitrageStrategy, ArbitrageOpportunity, StrategyState, 
    StrategyStatistics, StrategyExecutor
};
pub use futures_arbitrage::{
    FuturesArbitrageStrategy, FuturesArbitrageOpportunity, 
    FuturesStrategyState, FuturesArbitrageStats
};
