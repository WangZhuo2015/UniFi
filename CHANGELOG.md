# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-03-26

### Added
- Initial release
- WiFi network scanning support for macOS, Windows, and Linux
- WiFi standard detection (WiFi 4/5/6/7)
- Complete IE (Information Element) parsing
- QAM modulation detection (256/1K/4K-QAM)
- MLO (Multi-Link Operation) detection for WiFi 7
- Protocol extension detection (802.11k/r/v/w)
- BSS load monitoring
- Dark mode support
- CLI mode for command-line usage
- Professional UI with network cards and details panel

### Platform Support
- macOS: airport CLI scanner
- Windows: WlanApi scanner
- Linux: nl80211/iw scanner

### Technical
- Modular architecture with Scanner trait
- Pure function IE parser
- Tauri 2.0 + Svelte 5 frontend
- Rust backend

## [Unreleased]

### Planned
- libpcap scanner for macOS (fallback when airport is restricted)
- Signal history graphs
- Channel recommendation
- Network grouping by SSID
- Export scan results
- Multi-language support
