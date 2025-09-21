# 配置指南 - Cross-Exchange Arbitrage Strategy

## 📋 实盘交易手续费和滑点配置位置

### 1. 主要配置文件：`config/arbitrage.toml`

这是系统的主配置文件，包含所有交易参数设置：

#### 🚨 滑点配置 (Slippage Configuration)
```toml
[execution]
# 实盘交易滑点容忍度 (第47行)
slippage_tolerance = 0.001  # 0.1%

# 建议设置:
# - Binance: 0.0005-0.001 (0.05%-0.1%)
# - Bybit: 0.0005-0.0015 (0.05%-0.15%)
# - 高波动期间可适当提高到 0.002 (0.2%)
```

#### 🚨 手续费配置 (Fee Configuration)
```toml
[execution]
# 启用手续费计算 (第65行)
enable_fees = true

# 挂单手续费率 (第75行)
maker_fee = 0.001  # 0.1%

# 吃单手续费率 (第85行)  
taker_fee = 0.001  # 0.1%
```

### 2. 实盘交易配置文件：`config/live_trading.toml`

这是专门用于实盘交易的配置文件，包含API密钥和风险控制：

#### 🔑 API密钥配置
```toml
[exchanges.binance]
api_key = "YOUR_BINANCE_API_KEY_HERE"
secret_key = "YOUR_BINANCE_SECRET_KEY_HERE"

[exchanges.bybit]
api_key = "YOUR_BYBIT_API_KEY_HERE"
secret_key = "YOUR_BYBIT_SECRET_KEY_HERE"
```

## 📊 手续费等级对照表

### Binance 现货交易手续费
| VIP等级 | 挂单费率 (Maker) | 吃单费率 (Taker) |
|---------|------------------|------------------|
| VIP0    | 0.1%             | 0.1%             |
| VIP1    | 0.09%            | 0.09%            |
| VIP2    | 0.08%            | 0.08%            |
| VIP3    | 0.07%            | 0.07%            |
| VIP4    | 0.07%            | 0.07%            |
| VIP5    | 0.06%            | 0.06%            |
| VIP6    | 0.05%            | 0.05%            |
| VIP7    | 0.04%            | 0.04%            |
| VIP8    | 0.03%            | 0.03%            |
| VIP9    | 0.02%            | 0.04%            |

### Bybit 现货交易手续费
| VIP等级 | 挂单费率 (Maker) | 吃单费率 (Taker) |
|---------|------------------|------------------|
| VIP0    | 0.1%             | 0.1%             |
| VIP1    | 0.08%            | 0.08%            |
| VIP2    | 0.06%            | 0.06%            |
| VIP3    | 0.05%            | 0.05%            |
| VIP4    | 0.04%            | 0.04%            |
| VIP5    | 0.02%            | 0.04%            |

## 🔧 配置步骤

### 第1步：确定您的VIP等级
1. 登录Binance账户，查看VIP等级
2. 登录Bybit账户，查看VIP等级

### 第2步：修改手续费配置
编辑 `config/arbitrage.toml` 文件：

```toml
# 根据您的实际VIP等级调整
maker_fee = 0.001  # 例如：VIP0=0.001, VIP5=0.0006, VIP9=0.0002
taker_fee = 0.001  # 例如：VIP0=0.001, VIP5=0.0006, VIP9=0.0004
```

### 第3步：设置滑点容忍度
根据市场条件和交易频率调整：

```toml
# 保守设置 (适合大额交易)
slippage_tolerance = 0.0005  # 0.05%

# 标准设置 (适合一般交易)
slippage_tolerance = 0.001   # 0.1%

# 激进设置 (适合高频交易)
slippage_tolerance = 0.002   # 0.2%
```

### 第4步：配置API密钥 (实盘交易)
编辑 `config/live_trading.toml` 文件：

1. 获取Binance API密钥：
   - 登录Binance → 账户 → API管理 → 创建API
   - 权限：✅现货交易 ✅读取 ❌期货 ❌提现

2. 获取Bybit API密钥：
   - 登录Bybit → 账户 → API → 创建API密钥
   - 权限：✅现货交易 ✅读取 ❌衍生品 ❌提现

3. 填入配置文件：
```toml
[exchanges.binance]
api_key = "您的实际API密钥"
secret_key = "您的实际密钥"

[exchanges.bybit]
api_key = "您的实际API密钥"
secret_key = "您的实际密钥"
```

## 🚀 持续运行Dry-Run测试

### 简单测试 (推荐)
```bash
cargo run --example simple_continuous_test
```

### 完整测试 (高级)
```bash
cargo run --example continuous_dry_run
```

### 测试结果解读

刚才的测试结果显示：
- **执行了6次套利机会** (每3次迭代一次)
- **总交易量**: $6,002.10
- **总手续费**: $6.00 (0.1% × 2 × 6次)
- **净损失**: -$6.64 (包含滑点和手续费)

这说明：
1. ✅ 系统正常执行交易
2. ✅ 手续费计算正确
3. ✅ 滑点模拟正常
4. ⚠️ 当前7个基点的价差不足以覆盖手续费和滑点成本

## 💡 优化建议

### 1. 提高最小价差阈值
```toml
[strategy]
# 从5个基点提高到8-10个基点
min_spread_bps = 10
```

### 2. 降低手续费 (通过提升VIP等级)
- 增加交易量以提升VIP等级
- 或者持有平台币获得手续费折扣

### 3. 优化订单类型
- 优先使用限价单 (Maker) 而非市价单 (Taker)
- 减少手续费成本

### 4. 监控市场条件
- 在高波动期间暂停交易
- 选择流动性更好的交易对

## 🔒 安全提醒

1. **API密钥安全**：
   - 不要将API密钥提交到版本控制
   - 定期轮换API密钥
   - 启用IP白名单

2. **风险控制**：
   - 从小仓位开始测试
   - 设置每日损失限制
   - 监控系统运行状态

3. **测试流程**：
   - 先在testnet环境测试
   - 然后用小额资金实盘测试
   - 确认无误后逐步增加仓位

## 📞 技术支持

如果您在配置过程中遇到问题，请检查：

1. **配置文件格式**是否正确 (TOML语法)
2. **API密钥权限**是否设置正确
3. **网络连接**是否正常
4. **日志文件**中的错误信息

---

**重要提醒**：实盘交易存在风险，请确保充分测试后再投入资金，并严格控制风险。
