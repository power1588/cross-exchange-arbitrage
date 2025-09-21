use clap::{Parser, Subcommand};
use cross_exchange_arbitrage::{
    config::ArbitrageConfig,
    strategy::ArbitrageStrategy,
    trading::{DryRunExecutor, LiveTradingExecutor},
    utils::logger,
    Result,
};
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "arbitrage")]
#[command(about = "Cross-exchange arbitrage trading system")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/arbitrage.toml")]
    config: PathBuf,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Log file path
    #[arg(long, default_value = "logs/arbitrage.log")]
    log_file: PathBuf,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in dry-run mode (simulation)
    DryRun {
        /// Use live market data for simulation
        #[arg(long)]
        live_data: bool,
        
        /// Start date for historical data (YYYY-MM-DD)
        #[arg(long)]
        start_date: Option<String>,
        
        /// End date for historical data (YYYY-MM-DD)
        #[arg(long)]
        end_date: Option<String>,
    },
    /// Run in live trading mode
    Live {
        /// Skip initial balance check
        #[arg(long)]
        skip_balance_check: bool,
    },
    /// Validate configuration
    Validate,
    /// Show system status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    logger::init(&cli.log_level, &cli.log_file)?;
    
    info!("Starting Cross-Exchange Arbitrage System v{}", cross_exchange_arbitrage::VERSION);
    
    // Load configuration
    let config = ArbitrageConfig::from_file(&cli.config)?;
    info!("Configuration loaded from: {}", cli.config.display());
    
    match cli.command {
        Commands::DryRun { live_data, start_date, end_date } => {
            run_dry_run(config, live_data, start_date, end_date).await
        }
        Commands::Live { skip_balance_check } => {
            run_live_trading(config, skip_balance_check).await
        }
        Commands::Validate => {
            validate_config(config).await
        }
        Commands::Status => {
            show_status().await
        }
    }
}

async fn run_dry_run(
    config: ArbitrageConfig,
    live_data: bool,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<()> {
    info!("Starting dry-run mode");
    
    if live_data {
        info!("Using live market data for simulation");
    } else {
        info!("Using historical data: {} to {}", 
              start_date.as_deref().unwrap_or("N/A"),
              end_date.as_deref().unwrap_or("N/A"));
    }
    
    let mut executor = DryRunExecutor::new(config.clone()).await?;
    let mut strategy = ArbitrageStrategy::new(config).await?;
    
    // Start the simulation
    strategy.run_with_executor(&mut executor).await?;
    
    // Print results
    let results = executor.get_results().await;
    info!("Dry-run completed. Results: {:#?}", results);
    
    Ok(())
}

async fn run_live_trading(config: ArbitrageConfig, skip_balance_check: bool) -> Result<()> {
    info!("Starting live trading mode");
    
    if skip_balance_check {
        info!("Skipping initial balance check");
    }
    
    let mut executor = LiveTradingExecutor::new(config.clone()).await?;
    let mut strategy = ArbitrageStrategy::new(config).await?;
    
    // Perform pre-flight checks
    if !skip_balance_check {
        executor.check_balances().await?;
        info!("Balance check passed");
    }
    
    executor.check_connectivity().await?;
    info!("Connectivity check passed");
    
    // Start live trading
    info!("Starting live trading...");
    strategy.run_with_executor(&mut executor).await?;
    
    Ok(())
}

async fn validate_config(config: ArbitrageConfig) -> Result<()> {
    info!("Validating configuration...");
    
    match config.validate() {
        Ok(_) => {
            info!("✅ Configuration is valid");
            println!("Configuration validation passed!");
        }
        Err(e) => {
            error!("❌ Configuration validation failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn show_status() -> Result<()> {
    info!("Checking system status...");
    
    // TODO: Implement system status check
    println!("System Status:");
    println!("  Version: {}", cross_exchange_arbitrage::VERSION);
    println!("  Status: Not implemented yet");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }
}
