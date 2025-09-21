# Cross-Exchange Arbitrage Strategy

基于HFTBacktest框架的跨交易所价差套利策略项目，支持Binance和Bybit交易所的实时数据接入，提供干跑(dry-run)和实盘交易模式。

## 项目概述

### 核心功能
- **跨交易所价差监控**: 实时监控Binance和Bybit之间的价格差异
- **自动套利执行**: 当价差超过阈值时自动执行套利交易
- **双模式支持**: 支持干跑模式(回测/模拟)和实盘交易模式
- **风险管理**: 内置仓位管理、止损和风险控制机制
- **实时监控**: 提供详细的交易日志和性能监控

### 技术架构
- **后端**: Rust + HFTBacktest框架
- **数据源**: Binance WebSocket API + Bybit WebSocket API
- **策略引擎**: 基于HFTBacktest的事件驱动架构
- **配置管理**: TOML配置文件
- **日志系统**: 结构化日志记录

## 项目结构

```
cross-exchange-arbitrage/
├── Cargo.toml                 # Rust项目配置
├── README.md                  # 项目文档
├── config/                    # 配置文件目录
│   ├── arbitrage.toml        # 套利策略配置
│   ├── binance.toml          # Binance连接配置
│   └── bybit.toml            # Bybit连接配置
├── src/                       # 源代码目录
│   ├── main.rs               # 程序入口
│   ├── lib.rs                # 库文件
│   ├── config/               # 配置管理模块
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── connectors/           # 交易所连接器
│   │   ├── mod.rs
│   │   ├── binance.rs
│   │   ├── bybit.rs
│   │   └── traits.rs
│   ├── strategy/             # 策略模块
│   │   ├── mod.rs
│   │   ├── arbitrage.rs
│   │   ├── risk_manager.rs
│   │   └── position_manager.rs
│   ├── data/                 # 数据处理模块
│   │   ├── mod.rs
│   │   ├── market_data.rs
│   │   └── orderbook.rs
│   ├── trading/              # 交易执行模块
│   │   ├── mod.rs
│   │   ├── executor.rs
│   │   ├── dry_run.rs
│   │   └── live_trading.rs
│   └── utils/                # 工具模块
│       ├── mod.rs
│       ├── logger.rs
│       └── metrics.rs
├── tests/                     # 测试目录
│   ├── integration/          # 集成测试
│   └── unit/                 # 单元测试
├── examples/                  # 示例代码
│   ├── dry_run_example.rs
│   └── live_trading_example.rs
└── docs/                     # 文档目录
    ├── architecture.md
    ├── configuration.md
    └── deployment.md
```

## 核心组件设计

### 1. 数据连接器 (Connectors)
- **ExchangeConnector Trait**: 统一的交易所接口
- **BinanceConnector**: Binance WebSocket数据接入
- **BybitConnector**: Bybit WebSocket数据接入
- **数据标准化**: 统一的市场数据格式

### 2. 套利策略引擎 (Strategy Engine)
- **ArbitrageStrategy**: 核心套利逻辑
- **价差计算**: 实时计算两个交易所间的价差
- **信号生成**: 基于价差阈值生成交易信号
- **仓位管理**: 动态调整仓位大小

### 3. 风险管理 (Risk Management)
- **RiskManager**: 风险控制核心
- **仓位限制**: 最大仓位和杠杆控制
- **止损机制**: 自动止损和风险退出
- **资金管理**: 资金分配和使用率控制

### 4. 交易执行 (Trading Execution)
- **DryRunExecutor**: 模拟交易执行器
- **LiveTradingExecutor**: 实盘交易执行器
- **订单管理**: 订单生命周期管理
- **执行优化**: 最优执行路径选择

## 配置系统

### 策略配置 (arbitrage.toml)
```toml
[strategy]
symbol = "BTCUSDT"
min_spread_bps = 10          # 最小价差(基点)
max_position_size = 1.0      # 最大仓位
rebalance_threshold = 0.1    # 再平衡阈值

[risk]
max_drawdown = 0.05          # 最大回撤
stop_loss_bps = 50           # 止损点(基点)
position_limit = 10.0        # 仓位限制

[execution]
order_timeout_ms = 5000      # 订单超时时间
slippage_tolerance = 0.001   # 滑点容忍度
```

### 交易所配置
```toml
# binance.toml
[connection]
websocket_url = "wss://stream.binance.com:9443/ws"
rest_api_url = "https://api.binance.com"

[auth]
api_key = "${BINANCE_API_KEY}"
secret_key = "${BINANCE_SECRET_KEY}"

# bybit.toml
[connection]
websocket_url = "wss://stream.bybit.com/v5/public/spot"
rest_api_url = "https://api.bybit.com"

[auth]
api_key = "${BYBIT_API_KEY}"
secret_key = "${BYBIT_SECRET_KEY}"
```

## 运行模式

### Dry-Run模式
```bash
# 使用历史数据进行回测
cargo run -- --mode dry-run --config config/arbitrage.toml --start-date 2024-01-01 --end-date 2024-01-31

# 使用实时数据进行模拟交易
cargo run -- --mode dry-run --config config/arbitrage.toml --live-data
```

### 实盘交易模式
```bash
# 启动实盘交易
cargo run -- --mode live --config config/arbitrage.toml
```

## 监控和日志

### 性能指标
- **收益率**: 总收益率、年化收益率
- **夏普比率**: 风险调整后收益
- **最大回撤**: 历史最大回撤
- **胜率**: 盈利交易占比
- **平均持仓时间**: 套利机会持续时间

### 实时监控
- **价差监控**: 实时价差变化图表
- **仓位监控**: 当前仓位和风险暴露
- **交易日志**: 详细的交易执行记录
- **系统状态**: 连接状态和系统健康度

## 部署和运维

### 系统要求
- **操作系统**: Linux/macOS/Windows
- **Rust版本**: 1.70+
- **内存**: 最少4GB RAM
- **网络**: 稳定的互联网连接

### 部署步骤
1. 克隆项目代码
2. 配置交易所API密钥
3. 调整策略参数
4. 运行系统测试
5. 启动交易程序

### 监控告警
- **连接异常**: WebSocket连接断开告警
- **价差异常**: 异常价差波动告警
- **风险告警**: 超过风险阈值告警
- **系统告警**: 系统资源使用告警

## 安全考虑

### API安全
- **密钥管理**: 环境变量存储API密钥
- **权限控制**: 最小权限原则
- **加密传输**: 所有API调用使用HTTPS/WSS

### 交易安全
- **订单验证**: 多重订单合法性检查
- **资金保护**: 严格的资金使用限制
- **异常处理**: 完善的异常情况处理机制

## 扩展性

### 支持更多交易所
- 实现ExchangeConnector trait
- 添加新的配置文件
- 更新策略逻辑以支持新交易所

### 支持更多策略
- 基于现有框架扩展新的套利策略
- 支持多币种套利
- 支持期货现货套利

### 性能优化
- 使用更高效的数据结构
- 实现并行处理
- 优化网络通信

## 许可证

本项目基于MIT许可证开源，详见LICENSE文件。
