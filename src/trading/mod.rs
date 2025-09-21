//! Trading execution modules

pub mod dry_run;
pub mod live_trading;
// pub mod executor; // Will be implemented later

pub use dry_run::{DryRunExecutor, Portfolio, PerformanceMetrics};
pub use live_trading::{LiveTradingExecutor, HealthStatus, ExecutionStatistics, Position, ExchangeInfo};

/// Results structure for execution summary
#[derive(Debug)]
pub struct ExecutionResults {
    /// Total trades executed
    pub total_trades: u64,
    /// Total profit/loss
    pub total_pnl: f64,
}
