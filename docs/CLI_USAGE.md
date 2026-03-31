# UniFi CLI 使用指南

UniFi 是一个专业的 WiFi 分析工具，提供命令行界面用于扫描、分析和诊断无线网络。

## 安装

```bash
# 从源码构建
git clone https://github.com/WangZhuo2015/UniFi.git
cd UniFi
cargo build --release

# 可执行文件位置
./target/release/unifi
```

## 基本用法

```bash
unifi <COMMAND> [OPTIONS]
```

## 命令概览

| 命令 | 描述 |
|------|------|
| `scan` | 扫描附近的 WiFi 网络 |
| `current` | 显示当前连接的网络详情 |
| `info` | 显示指定网络的详细信息 |
| `parse-ie` | 解析十六进制 IE 数据 |
| `scanners` | 列出可用的扫描器 |

---

## scan - 扫描网络

扫描附近的 WiFi 网络并显示基本信息。

### 用法

```bash
unifi scan [OPTIONS]
```

### 选项

| 选项 | 简写 | 默认值 | 描述 |
|------|------|--------|------|
| `--format` | `-f` | `table` | 输出格式：`table`、`json`、`csv` |
| `--band` | `-b` | `all` | 频段过滤：`2.4`、`5`、`6`、`all` |
| `--ie` | | | 显示 IE (Information Element) 详情 |
| `--scanner` | `-s` | `default` | 使用的扫描器 |

### 示例

```bash
# 基本扫描
unifi scan

# JSON 格式输出
unifi scan --format json

# 只扫描 5GHz 网络
unifi scan --band 5

# 显示 IE 详情
unifi scan --ie

# 使用 libpcap 扫描器（需要 root）
sudo unifi scan --scanner libpcap

# 导出 CSV
unifi scan --format csv > networks.csv
```

### 输出格式

**表格格式 (默认)**
```
SSID                             BSSID              Ch   Band   Signal    Standard
--------------------------------------------------------------------------------
MyNetwork                        AA:BB:CC:DD:EE:FF  36   5      -45 dBm  ax
Guest-Network                    AA:BB:CC:DD:EE:00  6    2.4    -60 dBm  n
[Hidden]                         AA:BB:CC:DD:EE:11  149  5      -75 dBm  ac
```

**JSON 格式**
```json
[
  {
    "ssid": "MyNetwork",
    "bssid": "AA:BB:CC:DD:EE:FF",
    "channel": 36,
    "band": "5",
    "signal": -45,
    "standards": ["n", "ac", "ax"],
    "wifiGeneration": 6,
    "channelWidth": 80,
    "security": "wpa3"
  }
]
```

---

## current - 当前连接

显示当前连接的 WiFi 网络详细信息，包括本地适配器能力。

### 用法

```bash
unifi current [OPTIONS]
```

### 选项

| 选项 | 简写 | 默认值 | 描述 |
|------|------|--------|------|
| `--scanner` | `-s` | `default` | 使用的扫描器 |

### 示例

```bash
unifi current
```

### 输出示例

```
Connected to: MyNetwork
BSSID:        AA:BB:CC:DD:EE:FF
Channel:      36 (5 GHz)
Signal:       -45 dBm
Standard:     ["n", "ac", "ax"]
Security:     wpa3
AP Streams:   4
Current Width: 80 MHz
AP Max Width: 160 MHz
AP Current Peak: 1200.5 Mbps
AP Max Peak:     2401.0 Mbps
Local Peak:      1200.5 Mbps
Local Streams:   2
Local Max Width: 160 MHz
Local Standards: ["n", "ac", "ax"]
Local Adapter:   Apple Wi-Fi Controller
Link RX:      866.7 Mbps
Link TX:      866.7 Mbps
```

---

## info - 网络详情

显示指定 BSSID 网络的完整详细信息，包括 MIMO、OFDMA、安全、漫游协议等。

### 用法

```bash
unifi info <BSSID> [OPTIONS]
```

### 参数

| 参数 | 描述 |
|------|------|
| `BSSID` | 目标网络的 BSSID（MAC 地址） |

### 选项

| 选项 | 简写 | 默认值 | 描述 |
|------|------|--------|------|
| `--scanner` | `-s` | `default` | 使用的扫描器 |

### 示例

```bash
unifi info AA:BB:CC:DD:EE:FF
```

### 输出部分

- **基本信息**: SSID、BSSID、信道、频段、信号强度、SNR
- **WiFi 标准**: 支持的标准、信道宽度、空间流数量、峰值速率
- **MIMO & 波束成形**: SU-MIMO、MU-MIMO、Beamformer/Beamformee
- **信道详情**: 主/副信道、中心频率、带宽模式
- **OFDMA & TWT**: DL/UL OFDMA、RU 尺寸、TWT 省电特性
- **WiFi 7 特性**: MLO、Punctured Preamble、Multi-RU
- **MCS & 调制**: 最大 QAM、MCS 索引、BSS Coloring、保护间隔
- **安全**: 加密类型、认证方法、SAE/OWE/PMF
- **漫游协议**: 802.11k/r/v/w、WMM/U-APSD

---

## parse-ie - 解析 IE 数据

从十六进制字符串解析 802.11 IE 数据。

### 用法

```bash
unifi parse-ie <HEX_DATA>
```

### 示例

```bash
unifi parse-ie "000857694669362d4158030124"
```

---

## scanners - 扫描器列表

列出系统上可用的 WiFi 扫描器及其状态。

### 用法

```bash
unifi scanners [OPTIONS]
```

### 选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--verbose` | `-v` | 显示详细信息 |

### macOS 扫描器

| 扫描器 | 需要 Root | IE 数据 | 说明 |
|--------|-----------|---------|------|
| CoreWLAN | 否 | 否 | App Store 兼容，无 WiFi 标准检测 |
| Airport | 否 | 完整 | macOS 26+ 可能不可用 |
| Libpcap | 是 | 完整 | 支持 macOS 26+，不能用于 App Store |

### Linux 扫描器

| 扫描器 | 需要 Root | IE 数据 | 说明 |
|--------|-----------|---------|------|
| nl80211 | 否 | 完整 | 推荐 |
| Libpcap | 是 | 完整 | 需要监控模式 |

### Windows 扫描器

| 扫描器 | 需要 Root | IE 数据 | 说明 |
|--------|-----------|---------|------|
| WlanAPI | 否 | 部分 | Windows 原生 API |

---

## 扫描器选择指南

### macOS

| 场景 | 推荐扫描器 |
|------|-----------|
| 快速扫描，不需要 root | `airport`（默认） |
| 需要 WiFi 6/7 检测 | `airport` 或 `libpcap` |
| macOS 26+ | `libpcap`（需要 sudo） |
| App Store 发布 | `corewlan` |

### Linux

| 场景 | 推荐扫描器 |
|------|-----------|
| 日常使用 | `nl80211`（默认） |
| 需要 IE 详情 | `nl80211` |

### Windows

| 场景 | 推荐扫描器 |
|------|-----------|
| 所有场景 | `wlanapi`（默认） |

---

## 高级用法

### JSON 输出配合 jq

```bash
# 只显示 SSID 和信号
unifi scan --format json | jq '.[] | {ssid, signal}'

# 过滤 WiFi 6 网络
unifi scan --format json | jq '.[] | select(.wifiGeneration == 6)'

# 统计各频段网络数
unifi scan --format json | jq 'group_by(.band) | map({band: .[0].band, count: length})'
```

### 导出网络列表

```bash
# CSV 格式
unifi scan --format csv > wifi_networks.csv

# JSON 格式（完整数据）
unifi scan --format json > wifi_networks.json
```

---

## 故障排除

### 扫描器不可用

```bash
# 检查可用扫描器
unifi scanners --verbose

# Libpcap 需要 root
sudo unifi scan --scanner libpcap
```

### macOS 上 BSSID 显示为 00:00:00:00:00:00

CoreWLAN 扫描器需要位置权限才能获取 BSSID。使用 Airport 或 Libpcap 扫描器。

### WiFi 标准显示为 "Legacy"

CoreWLAN 扫描器不提供 IE 数据，无法检测 WiFi 标准。使用 Airport 或 Libpcap。
