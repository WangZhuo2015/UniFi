# UniFi 开发计划

## 一、数据源分析

### macOS airport -s -x 输出（真实数据）

| 字段 | 说明 | 可提取信息 |
|------|------|-----------|
| BSSID | MAC地址 | ✅ 厂商识别 |
| SSID_STR | 网络名称 | ✅ 直接显示 |
| RSSI | 信号强度 | ✅ 直接显示 |
| CHANNEL | 信道 | ✅ 直接显示 |
| CHANNEL_FLAGS | 信道标志 | ✅ 频段/带宽 |
| BEACON_INT | Beacon间隔 | ✅ 直接显示 |
| HT_CAPS_IE | 802.11n能力 | ✅ WiFi 4, MU-MIMO |
| VHT_CAPS | 802.11ac能力 | ✅ WiFi 5, MU-MIMO |
| HE_CAP | 802.11ax能力 | ✅ WiFi 6, OFDMA |
| EXT_CAPS | 扩展能力 | ✅ **802.11v (BSS_TRANS_MGMT)** |
| RSN_IE | 安全信息 | ✅ 加密类型, **802.11w (PMF)** |
| WPA_IE | WPA信息 | ✅ 认证方式 |
| QBSS_LOAD_IE | BSS负载 | ✅ 信道利用率 |
| 80211D_IE | 国家信息 | ✅ 国家代码 |
| RATES | 支持速率 | ✅ 最大速率 |

### 需要额外解析的字段

| 信息 | 来源 | 解析难度 |
|------|------|---------|
| 802.11k (RRM) | 原始IE位图 | 中等 |
| 802.11r (FT) | MDIE/RSN_IE | 中等 |
| MU-MIMO | VHT_CAPS/HE_CAP | 简单 |
| OFDMA | HE_CAP | 简单 |
| BSS Coloring | HE_CAP | 简单 |

---

## 二、版本规划

### 普通版（Lite）
- 基础网络信息（SSID/BSSID/信号/信道）
- 频段识别（2.4G/5G/6G）
- 安全类型（Open/WPA2/WPA3）
- 信道分析
- 厂商识别

### 专业版（Pro）
- **WiFi代数识别**（WiFi 4/5/6/7）
- **802.11k/v/r/w 协议支持**
- **MU-MIMO / OFDMA 检测**
- **信道带宽**（20/40/80/160MHz）
- **空间流数量**
- **最大理论速率**
- **信道利用率**
- **Beacon间隔**
- **国家代码**

---

## 三、实施步骤

### Phase 1: 解析器重构（1天）

**目标**：解析 airport XML 输出，提取真实数据

```
src-tauri/src/
├── lib.rs
├── scanner/
│   ├── mod.rs
│   ├── macos.rs      # airport XML 解析
│   └── parser.rs     # plist 解析
├── types.rs          # 数据类型
└── oui.rs            # 厂商数据库
```

### Phase 2: 类型系统更新（半天）

**目标**：更新类型定义，区分确定数据和可选数据

```rust
pub struct Network {
    // === 确定数据（直接从API获取）===
    pub ssid: Option<String>,
    pub bssid: String,
    pub signal: i16,
    pub channel: u16,
    pub band: Band,
    pub security: Security,
    
    // === 专业版数据 ===
    #[cfg(feature = "pro")]
    pub wifi_generation: WifiGeneration,
    #[cfg(feature = "pro")]
    pub protocols: ProtocolExtensions,  // 真实的 k/v/r/w
    #[cfg(feature = "pro")]
    pub features: PerformanceFeatures,  // 真实的 MU-MIMO/OFDMA
}
```

### Phase 3: 专业解析实现（1天）

**目标**：实现真实的IE解析

- 解析 HT_CAPS_IE → WiFi 4 信息
- 解析 VHT_CAPS → WiFi 5, MU-MIMO
- 解析 HE_CAP → WiFi 6, OFDMA, BSS Coloring
- 解析 EXT_CAPS → 802.11v
- 解析 RSN_IE → 802.11w, 安全详情

### Phase 4: UI适配（1天）

**目标**：前端显示真实数据

- 普通版：隐藏专业信息
- 专业版：显示完整信息

### Phase 5: 功能开关（半天）

**目标**：实现版本切换

```toml
# Cargo.toml
[features]
default = ["lite"]
lite = []
pro = []
```

---

## 四、验证方案

使用当前设备测试：
- 已连接: Redmi_C7A0_5G (WiFi 6, 信道48, 80MHz)
- 扫描结果包含多个网络，可验证解析正确性

---

## 五、下一步

1. 重写 scanner/macos.rs，解析 XML plist
2. 实现真实的IE解析
3. 更新前端显示
