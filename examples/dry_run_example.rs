//! Dry-run example for cross-exchange arbitrage

use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    strategy::ArbitrageStrategy,
    trading::DryRunExecutor,
    utils::logger,
    Result,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    logger::init("info", "logs/dry_run_example.log")?;
    
    info!("Starting dry-run example");
    
    // Load configuration
    let config = ArbitrageConfig::from_file("config/arbitrage.toml")?;
    
    // Validate configuration
    config.validate()?;
    info!("Configuration validated successfully");
    
    // Create dry-run executor
    let mut executor = DryRunExecutor::new(config.clone()).await?;
    info!("Dry-run executor created");
    
    // Create arbitrage strategy
    let mut strategy = ArbitrageStrategy::new(config).await?;
    info!("Arbitrage strategy created");
    
    // Run the strategy
    info!("Starting arbitrage strategy simulation...");
    strategy.run_with_executor(&mut executor).await?;
    
    // Get and display results
    let results = executor.get_results();
    info!("Simulation completed. Results: {:#?}", results);
    
    println!("Dry-run simulation completed successfully!");
    println!("Check the logs for detailed information.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dry_run_example() {
        // This test would require mock data and configurations
        // For now, we just test that the main function compiles
        assert!(true);
    }
}
