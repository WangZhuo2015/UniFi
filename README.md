# UniFi

<div align="center">

**Professional WiFi Analysis Tool**

Cross-platform · Lightweight · Open Source

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)](https://github.com/WangZhuo2015/UniFi)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange)](https://www.rust-lang.org/)

</div>

---

## Features

### 🔍 Network Scanning
- Real-time WiFi network scanning
- Support for 2.4GHz / 5GHz / 6GHz bands
- Display signal strength, channel, bandwidth, etc.

### 📊 Professional Analysis
- **WiFi Standard Detection**: Auto-detect WiFi 4/5/6/7
- **IE Parsing**: Complete 802.11 Information Element parsing
- **QAM Detection**: 256-QAM / 1K-QAM / 4K-QAM
- **MLO Support**: WiFi 7 Multi-Link Operation detection

### 🛡️ Security Information
- WPA2/WPA3 security type identification
- Encryption suite detection (CCMP/GCMP)
- 802.11w PMF support detection

### 📡 Protocol Extensions
- **802.11k**: Radio Resource Measurement
- **802.11r**: Fast BSS Transition (Fast Roaming)
- **802.11v**: BSS Transition Management
- **802.11w**: Protected Management Frames

### 📈 Network Load
- Channel utilization monitoring
- Connected device count
- Available bandwidth capacity

## Installation

### Download Pre-built Binaries

Visit the [Releases](https://github.com/WangZhuo2015/UniFi/releases) page to download for your platform.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/WangZhuo2015/UniFi.git
cd UniFi

# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

### CLI Mode

```bash
# Build CLI
cd src-tauri
cargo build --release

# Basic commands
./target/release/unifi scan                    # Scan networks
./target/release/unifi scan --format json      # JSON output
./target/release/unifi current                 # Current connection
./target/release/unifi info AA:BB:CC:DD:EE:FF  # Network details
./target/release/unifi scanners --verbose      # List available scanners
```

See [CLI Usage Guide](./docs/CLI_USAGE.md) for complete documentation.

## Platform Support

| Platform | Scanner | IE Data | WiFi 6/7 | Privilege Required |
|----------|---------|---------|----------|-------------------|
| macOS | airport | ✓ Full | ✓ | None |
| macOS | corewlan | ✗ None | ✗ | None |
| macOS | libpcap | ✓ Full | ✓ | root |
| Windows | WlanAPI | Partial | Partial | None |
| Linux | nl80211 | ✓ Full | ✓ | None |
| Linux | libpcap | ✓ Full | ✓ | root |

## Tech Stack

- **Frontend**: [Svelte 5](https://svelte.dev/) + [TypeScript](https://www.typescriptlang.org/)
- **Backend**: [Rust](https://www.rust-lang.org/) + [Tauri 2.0](https://tauri.app/)
- **Styling**: [Tailwind CSS](https://tailwindcss.com/)
- **Build**: [Vite](https://vitejs.dev/) + [pnpm](https://pnpm.io/)

## Project Structure

```
unifi/
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores.ts       # State management
│   │   └── types.ts        # TypeScript types
│   └── routes/             # Page routes
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── scanner/        # Platform scanners
│   │   ├── parser/         # IE parser
│   │   └── lib.rs          # Main entry
│   └── Cargo.toml
└── docs/                   # Documentation
```

## Development

### Requirements

- Node.js >= 18
- pnpm >= 8
- Rust >= 1.75
- Platform-specific dependencies:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools
  - Linux: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`

### Commands

```bash
# Install dependencies
pnpm install

# Start development server
pnpm tauri dev

# Run lint
pnpm lint

# Build for production
pnpm tauri build
```

## Architecture

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

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for details.

## License

This project is licensed under the [MIT License](./LICENSE).

## Acknowledgments

- [Tauri](https://tauri.app/) - Cross-platform desktop framework
- [IEEE 802.11](https://standards.ieee.org/ieee/802.11/7140/) - WiFi Standard
- [WinFi](https://www.helge-keck.com/wifi-analysis/) - Inspiration

---

<div align="center">

Made with ❤️ by [WangZhuo](https://github.com/WangZhuo2015)

</div>
