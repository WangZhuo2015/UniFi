# Contributing to WiFi Tool

Thank you for your interest in contributing! This document provides guidelines for contributions.

## Development Setup

### Prerequisites

- Node.js >= 18
- pnpm >= 8
- Rust >= 1.75
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Windows**: Visual Studio Build Tools with C++ workload
  - **Linux**: `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev`

### Getting Started

```bash
# Clone the repository
git clone https://github.com/WangZhuo2015/UniFi.git
cd UniFi

# Install dependencies
pnpm install

# Start development server
pnpm tauri dev
```

## Project Architecture

### Scanner Trait

The core abstraction for WiFi scanning. Each platform implements this trait:

```rust
pub trait Scanner: Send + Sync {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError>;
    fn current(&self) -> Result<Option<RawBeacon>, ScanError>;
    fn name(&self) -> &'static str;
}
```

### Adding a New Platform

1. Create a new file in `src-tauri/src/scanner/`
2. Implement the `Scanner` trait
3. Add the module to `scanner/mod.rs`
4. Update `get_scanner()` function

### IE Parser

The parser is a pure function that transforms `RawBeacon` into `Network`:

```rust
pub fn parse_beacon(raw: &RawBeacon) -> Network
```

All IE parsing logic is in `parser/ie.rs`.

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- Run clippy before committing (`cargo clippy`)
- Document public APIs with doc comments

### TypeScript/Svelte

- Use TypeScript for all new code
- Follow existing component patterns
- Use Tailwind CSS for styling

## Commit Messages

Follow conventional commits:

```
feat: add support for WiFi 7 MLO detection
fix: correct EHT capabilities parsing
docs: update README with Linux instructions
refactor: extract IE parsing to separate module
test: add unit tests for IE parser
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and linting
5. Commit your changes
6. Push to your fork
7. Open a Pull Request

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## Questions?

Open an issue for any questions or discussions.
