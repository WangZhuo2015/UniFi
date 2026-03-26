# Architecture

## Design Philosophy

Following Linus Torvalds' programming philosophy:

1. **Data structures define the code** - Define data first, then write code
2. **Simple and direct** - Avoid over-abstraction
3. **Eliminate special cases** - Unified data flow makes code more generic
4. **Pragmatism** - Performance first, practical solutions

## Data Flow

```
RawBeacon (raw data) -> BeaconParser (parse) -> Network (structured data)
         ↑                      ↑
    Scanner trait         Pure functions, stateless
         ↑
    ┌────┴────┬─────────┬─────────┐
    │         │         │         │
 Airport   Libpcap   WlanApi   Nl80211
 (macOS)   (macOS)  (Windows)  (Linux)
```

## Core Components

### Scanner Trait

The only interface for WiFi scanning:

```rust
pub trait Scanner: Send + Sync {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError>;
    fn current(&self) -> Result<Option<RawBeacon>, ScanError>;
    fn name(&self) -> &'static str;
}
```

### RawBeacon

Minimal common data that all platforms can provide:

```rust
pub struct RawBeacon {
    pub ssid: Option<Vec<u8>>,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub band: Band,
    pub signal_dbm: i16,
    pub noise_dbm: i16,
    pub ie_data: Vec<u8>,
    pub beacon_interval: u16,
    pub timestamp: u64,
    pub connected: bool,
}
```

### Parser

Pure functions for data transformation:

```rust
pub fn parse_beacon(raw: &RawBeacon) -> Network
pub fn parse_all_ies(ie_data: &[u8]) -> IEDetails
```

## Platform Implementations

| Platform | File | Method |
|----------|------|--------|
| macOS | `scanner/airport.rs` | airport CLI |
| macOS | `scanner/libpcap.rs` | libpcap (planned) |
| Windows | `scanner/wlanapi.rs` | WlanApi DLL |
| Linux | `scanner/nl80211.rs` | iw command |

## Features

- `gui` - Tauri GUI (default)
- `cli` - Command-line interface

## Building

```bash
# GUI mode (default)
cargo build

# CLI mode
cargo build --features cli

# Both
cargo build --all-features
```
