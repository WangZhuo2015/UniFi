# WiFi Scanner Plugin Architecture

## 概述

扫描器插件化设计，支持多平台、可扩展的 WiFi 扫描能力。

## 架构设计

```
src-tauri/
├── crates/
│   ├── scanner-core/           # 核心 trait 定义
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Scanner trait
│   │       ├── types.rs        # 共享类型
│   │       └── registry.rs     # 插件注册表
│   │
│   ├── scanner-airport/        # macOS Airport 插件
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   ├── scanner-corewlan/       # macOS CoreWLAN 插件
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   ├── scanner-libpcap/        # 跨平台 Libpcap 插件
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   ├── scanner-wlanapi/        # Windows WlanAPI 插件
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   └── scanner-nl80211/        # Linux nl80211 插件
│       ├── Cargo.toml
│       └── src/lib.rs
│
└── tests/
    └── scanner_integration/    # 集成测试
        └── tests/
```

## Scanner Trait

```rust
pub trait Scanner: Send + Sync {
    /// 扫描器名称
    fn name(&self) -> &'static str;
    
    /// 扫描网络
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError>;
    
    /// 获取当前连接的网络
    fn current(&self) -> Result<Option<RawBeacon>, ScanError>;
    
    /// 是否需要 root/管理员权限
    fn requires_privilege(&self) -> bool { false }
    
    /// 扫描器是否可用
    fn is_available(&self) -> bool { true }
    
    /// 平台支持
    fn platforms(&self) -> &[Platform];
    
    /// 功能支持
    fn capabilities(&self) -> ScannerCapabilities;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
}

#[derive(Debug, Clone)]
pub struct ScannerCapabilities {
    pub has_ie_data: bool,
    pub has_bssid: bool,
    pub has_signal: bool,
    pub has_security: bool,
    pub app_store_compatible: bool,
}
```

## 插件注册表

```rust
pub struct ScannerRegistry {
    scanners: Vec<Box<dyn Scanner>>,
}

impl ScannerRegistry {
    pub fn register(&mut self, scanner: Box<dyn Scanner>);
    pub fn get(&self, name: &str) -> Option<&dyn Scanner>;
    pub fn get_default(&self) -> &dyn Scanner;
    pub fn list(&self) -> Vec<&dyn Scanner>;
}
```

## 测试框架

每个插件需要实现以下测试：

1. **单元测试** - 核心功能测试
2. **集成测试** - 与注册表集成测试
3. **Mock 测试** - 使用模拟数据测试

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_scanner_available() { ... }
    
    #[test]
    fn test_scan_returns_valid_data() { ... }
    
    #[test]
    fn test_current_network() { ... }
}
```

## 插件实现模板

```rust
// crates/scanner-xxx/src/lib.rs
use scanner_core::{Scanner, RawBeacon, ScanError, Platform, ScannerCapabilities};

pub struct XxxScanner {
    // internal state
}

impl XxxScanner {
    pub fn new() -> Self { ... }
}

impl Scanner for XxxScanner {
    fn name(&self) -> &'static str { "xxx" }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        // implementation
    }
    
    fn platforms(&self) -> &[Platform] {
        &[Platform::MacOS] // or other platforms
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: true,
            has_bssid: true,
            has_signal: true,
            has_security: true,
            app_store_compatible: false,
        }
    }
}

// 注册到全局注册表
scanner_core::register_scanner!(XxxScanner::new());
```

## 构建系统

```toml
# Workspace Cargo.toml
[workspace]
members = [
    "crates/scanner-core",
    "crates/scanner-airport",
    "crates/scanner-corewlan",
    "crates/scanner-libpcap",
    "crates/scanner-wlanapi",
    "crates/scanner-nl80211",
    "src-tauri",  # main app
]

[workspace.dependencies]
scanner-core = { path = "crates/scanner-core" }
```

## 自动化测试流程

```yaml
# .github/workflows/scanner-tests.yml
name: Scanner Tests

on: [push, pull_request]

jobs:
  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test -p scanner-core -p scanner-airport -p scanner-corewlan
      
  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test -p scanner-core -p scanner-wlanapi
      
  test-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test -p scanner-core -p scanner-nl80211 -p scanner-libpcap
```
