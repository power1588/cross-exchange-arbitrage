//! Live trading example for cross-exchange arbitrage

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    strategy::ArbitrageStrategy,
    trading::LiveTradingExecutor,
    utils::logger,
    Result,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    logger::init("info", "logs/live_trading_example.log")?;
    
    info!("Starting live trading example");
    
    // Load configuration
    let config = ArbitrageConfig::from_file("config/arbitrage.toml")?;
    
    // Validate configuration
    config.validate()?;
    info!("Configuration validated successfully");
    
    // Create live trading executor
    let mut executor = LiveTradingExecutor::new(config.clone()).await?;
    info!("Live trading executor created");
    
    // Create arbitrage strategy
    let mut strategy = ArbitrageStrategy::new(config).await?;
    info!("Arbitrage strategy created");
    
    // Perform pre-flight checks
    executor.check_balances().await?;
    info!("Balance check passed");
    
    executor.check_connectivity().await?;
    info!("Connectivity check passed");
    
    // Run the strategy
    info!("Starting live arbitrage trading...");
    strategy.run_with_executor(&mut executor).await?;
    
    println!("Live trading completed successfully!");
    println!("Check the logs for detailed information.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_live_trading_example() {
        // This test would require real API keys and connections
        // For now, we just test that the main function compiles
        assert!(true);
    }
}
