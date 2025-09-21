# 🚀 使用示例和命令指南

## ✅ 命令行修正说明

### ❌ 错误的命令格式
```bash
# 这些命令会失败
cargo run -- --mode dry-run --config config/arbitrage.toml --live-data
cargo run -- --mode live --config config/arbitrage.toml
```

### ✅ 正确的命令格式
```bash
# 正确的子命令格式
cargo run -- dry-run --live-data
cargo run -- live --skip-balance-check
cargo run -- validate
cargo run -- status
```

## 🎯 完整使用指南

### 1. 基础命令
```bash
# 查看帮助
cargo run -- --help

# 查看版本信息
cargo run -- --version
```

### 2. 干跑模式 (Dry-Run)
```bash
# 使用模拟数据
cargo run -- dry-run

# 使用真实WebSocket数据 ⭐ 推荐
cargo run -- dry-run --live-data

# 使用历史数据
cargo run -- dry-run --start-date 2024-01-01 --end-date 2024-01-31

# 指定配置文件
cargo run -- --config config/arbitrage.toml dry-run --live-data

# 指定日志级别
cargo run -- --log-level debug dry-run --live-data
```

### 3. 实盘交易模式 (需要API密钥)
```bash
# 启动实盘交易
cargo run -- live

# 跳过余额检查
cargo run -- live --skip-balance-check

# 使用自定义配置
cargo run -- --config config/live_trading.toml live
```

### 4. 配置和状态
```bash
# 验证配置文件
cargo run -- validate

# 查看系统状态
cargo run -- status

# 指定配置文件验证
cargo run -- --config config/arbitrage.toml validate
```

## 🎮 示例程序 (推荐用于学习和测试)

### 真实数据相关
```bash
# 真实WebSocket数据套利扫描器 ⭐ 最新功能
cargo run --example real_data_arbitrage

# 永续合约套利演示 (Bybit Maker + Binance Taker)
cargo run --example futures_demo

# 完整的实时数据扫描器
cargo run --example futures_arbitrage_scanner
```

### 模拟数据测试
```bash
# 简单持续测试
cargo run --example simple_continuous_test

# 策略演示
cargo run --example strategy_demo

# 基础干跑示例
cargo run --example dry_run_example
```

### 高级功能
```bash
# 持续运行测试 (长期运行)
cargo run --example continuous_dry_run

# 实盘交易示例
cargo run --example live_trading_example
```

## 📊 实际运行效果

### 真实数据套利扫描器结果
```
🚀 REAL-TIME Arbitrage Scanner with Live Market Data
===================================================
📡 Connecting to ACTUAL exchange WebSocket feeds
⚠️  WARNING: This connects to REAL exchange data feeds

✅ Connected to Binance futures WebSocket
✅ Connected to Bybit futures WebSocket
📊 Processing live market data...

🎯 REAL OPPORTUNITY: BNBUSDT - Buy Bybit@1060.20 + Sell Binance@1060.95, Profit: $0.0059
💡 Total Opportunities: 4
🔄 Opportunity Rate: 9.7 per minute
```

### 主程序干跑模式结果
```
2025-09-21T11:09:52.746320Z  INFO Starting Cross-Exchange Arbitrage System v0.1.0
2025-09-21T11:09:52.748488Z  INFO Configuration loaded from: config/arbitrage.toml
2025-09-21T11:09:52.748547Z  INFO Starting dry-run mode
2025-09-21T11:09:52.748589Z  INFO Using live market data for simulation
✅ Dry-run completed successfully
```

## 🔧 故障排除

### 常见问题和解决方案

#### 1. 命令格式错误
```bash
# ❌ 错误
cargo run -- --mode dry-run

# ✅ 正确
cargo run -- dry-run
```

#### 2. 配置文件未找到
```bash
# 确保配置文件存在
ls config/arbitrage.toml

# 或使用默认配置
cargo run -- dry-run  # 会自动使用 config/arbitrage.toml
```

#### 3. 网络连接问题
```bash
# 测试网络连接
curl -s "https://fapi.binance.com/fapi/v1/ping"
curl -s "https://api.bybit.com/v5/market/time"
```

#### 4. 编译错误
```bash
# 清理并重新编译
cargo clean
cargo build --release
```

## 🎯 推荐使用流程

### 新用户入门
1. **编译项目**: `cargo build --release`
2. **运行简单测试**: `cargo run --example simple_continuous_test`
3. **体验真实数据**: `cargo run --example real_data_arbitrage`
4. **运行主程序**: `cargo run -- dry-run --live-data`

### 开发和调试
1. **查看帮助**: `cargo run -- --help`
2. **验证配置**: `cargo run -- validate`
3. **调试模式**: `cargo run -- --log-level debug dry-run --live-data`
4. **检查状态**: `cargo run -- status`

### 生产部署
1. **配置API密钥**: 编辑 `config/live_trading.toml`
2. **验证连接**: `cargo run -- validate`
3. **小额测试**: `cargo run -- live --skip-balance-check`
4. **正式运行**: `cargo run -- live`

## 📈 性能监控

### 日志文件位置
- **默认日志**: `logs/arbitrage.log`
- **自定义日志**: `cargo run -- --log-file custom.log dry-run`

### 监控指标
- **套利机会检测率**: 每分钟检测的机会数量
- **执行成功率**: 订单执行的成功百分比
- **净收益**: 扣除手续费后的实际收益
- **风险暴露**: 当前持仓和风险水平

---

**🎯 现在您可以使用正确的命令格式来运行系统了！推荐从 `cargo run --example real_data_arbitrage` 开始体验真实数据功能。**
