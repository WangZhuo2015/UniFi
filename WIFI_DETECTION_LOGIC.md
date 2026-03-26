# WiFi 标准检测逻辑 - 专业版

## 核心：Extension ID 正确区分

**关键发现**：HE (WiFi 6) 和 EHT (WiFi 7) 都使用 Element ID 255 (Extended Element)，但 Extension ID 不同！

| 标准 | Capabilities | Operation |
|------|--------------|-----------|
| **WiFi 6 (HE)** | Extension ID 35 (0x23) | Extension ID 36 (0x24) |
| **WiFi 7 (EHT)** | Extension ID 108 (0x6c) | Extension ID 106 (0x6a) |

## 判断优先级

```
1. 检查 Extended Element ID 108 (EHT Capabilities)
   └── 存在 → WiFi 7

2. 检查 Extended Element ID 35 (HE Capabilities)
   └── 存在 → WiFi 6

3. 检查 Element ID 191 (VHT Capabilities)
   └── 存在 → WiFi 5

4. 检查 Element ID 45 (HT Capabilities)
   └── 存在 → WiFi 4

5. 否则 → Legacy (802.11a/b/g)
```

## IE 完整映射表

### 标准 IE (Element ID 0-254)

| ID | Hex | 名称 | 说明 |
|----|-----|------|------|
| 0 | 0x00 | SSID | 网络名称 |
| 1 | 0x01 | Supported Rates | 支持的速率 |
| 3 | 0x03 | DS Parameter Set | 信道信息 |
| 5 | 0x05 | TIM | 流量指示图 |
| 7 | 0x07 | Country | 国家代码 |
| 11 | 0x0b | QBSS Load | BSS 负载（设备数、利用率） |
| 45 | 0x2d | HT Capabilities | WiFi 4 能力 |
| 48 | 0x30 | RSN | 安全信息 (WPA2/WPA3) |
| 50 | 0x32 | Extended Supported Rates | 扩展速率 |
| 61 | 0x3d | HT Operation | WiFi 4 操作参数 |
| 70 | 0x46 | RM Enabled Capabilities | 802.11k 无线资源管理 |
| 127 | 0x7f | Extended Capabilities | 扩展能力 (k/v/r) |
| 191 | 0xbf | VHT Capabilities | WiFi 5 能力 |
| 192 | 0xc0 | VHT Operation | WiFi 5 操作参数 |
| 221 | 0xdd | Vendor Specific | 厂商特定 (WPS, WMM等) |

### Extended IE (Element ID 255, 需要解析 Extension ID)

| Ext ID | Hex | 名称 | 标准 |
|--------|-----|------|------|
| 35 | 0x23 | HE Capabilities | WiFi 6 |
| 36 | 0x24 | HE Operation | WiFi 6 |
| 37 | 0x25 | MU EDCA Parameter Set | WiFi 6 |
| 38 | 0x26 | Multi-BSSID | 多 BSSID |
| 39 | 0x27 | Non-Inheritance | 非继承 |
| 106 | 0x6a | EHT Operation | **WiFi 7** |
| 107 | 0x6b | EHT Multi-Link | WiFi 7 MLO |
| 108 | 0x6c | EHT Capabilities | **WiFi 7** |

## 数据格式示例

### EHT Capabilities (Ext ID 108)

```
原始数据: ff 0f 6c 27 00 e0 1f 1b e0 18 75 80 36 00 44 44 44

解析:
  ff          = Element ID (Extended)
  0f          = Length (15 bytes)
  6c          = Extension ID (108 = EHT Capabilities)
  27 00       = MAC Capabilities (0x0027)
  e0 1f 1b e0 = PHY Capabilities
  ...         = EHT MCS Set
```

### MAC Capabilities 位定义 (EHT)

```
Bit 0:    EPCS Priority Access
Bit 1:    OM Control
Bit 2:    TXOP Return Response
Bit 3:    Two BQR
Bit 4:    MSCS
Bit 5:    SEL-TRA
Bit 7:    Link Adaptation
Bit 8:    Triggered TXOP Sharing
Bit 9:    Restricted TWT
Bit 10:   SCS Traffic Description
```

### PHY Capabilities 位定义 (EHT)

```
Bits 0-1:  320MHz in 6GHz
Bit 2:     242-tone RU (<=80MHz)
Bit 3:     NDP 4x EHT-LTF
Bit 4:     Partial BW UL MU-MIMO
Bit 5:     SU Beamformer
Bit 6:     SU Beamformee
Bits 7-9:  BFee STS <= 80MHz
Bits 10-12: BFee STS > 80MHz
...
Bit 18:    Rx 4096-QAM 160MHz
Bit 22:    Tx 4096-QAM 160MHz
```

## 检测代码

```rust
fn detect_wifi_generation(ie_data: &[u8]) -> &'static str {
    let mut has_eht_cap = false;
    let mut has_he_cap = false;
    let mut has_vht = false;
    let mut has_ht = false;
    
    let mut pos = 0;
    while pos + 1 < ie_data.len() {
        let elem_id = ie_data[pos];
        let length = ie_data[pos + 1] as usize;
        
        if pos + 2 + length > ie_data.len() { break; }
        
        match elem_id {
            45 => has_ht = true,
            191 => has_vht = true,
            255 if length >= 1 => {
                match ie_data[pos + 2] {
                    35 => has_he_cap = true,  // HE (WiFi 6)
                    108 => has_eht_cap = true, // EHT (WiFi 7)
                    _ => {}
                }
            }
            _ => {}
        }
        
        pos += 2 + length;
    }
    
    if has_eht_cap { "WiFi 7" }
    else if has_he_cap { "WiFi 6" }
    else if has_vht { "WiFi 5" }
    else if has_ht { "WiFi 4" }
    else { "Legacy" }
}
```

## 参考标准

- IEEE 802.11-2020 (WiFi 6 / 802.11ax)
- IEEE 802.11be D3.0 (WiFi 7 Draft)
- Wi-Fi Alliance: WFA Beacon Element Extension Specification
