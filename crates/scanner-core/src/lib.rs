//! Scanner Core - Plugin Interface for WiFi Scanning
//!
//! This crate provides the core trait and types for WiFi scanner plugins.
//! Each platform-specific scanner implements the `Scanner` trait.

mod types;
mod registry;
mod error;

pub use types::*;
pub use error::*;
pub use registry::{ScannerRegistry, ScannerInfo};

/// Platform identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
}

impl Platform {
    pub fn current() -> Platform {
        #[cfg(target_os = "macos")]
        { Platform::MacOS }
        #[cfg(target_os = "windows")]
        { Platform::Windows }
        #[cfg(target_os = "linux")]
        { Platform::Linux }
    }
}

/// Scanner capabilities
#[derive(Debug, Clone, Default)]
pub struct ScannerCapabilities {
    /// Can extract IE (Information Element) data
    pub has_ie_data: bool,
    /// Can get BSSID
    pub has_bssid: bool,
    /// Can get signal strength
    pub has_signal: bool,
    /// Can get security info
    pub has_security: bool,
    /// Compatible with App Store
    pub app_store_compatible: bool,
}

/// Scanner trait - implemented by all scanner plugins
pub trait Scanner: Send + Sync {
    /// Unique scanner name
    fn name(&self) -> &'static str;
    
    /// Human-readable description
    fn description(&self) -> &'static str {
        self.name()
    }
    
    /// Scan for WiFi networks
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError>;
    
    /// Get currently connected network
    fn current(&self) -> Result<Option<RawBeacon>, ScanError>;
    
    /// Supported platforms
    fn platforms(&self) -> &'static [Platform];
    
    /// Scanner capabilities
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities::default()
    }
    
    /// Requires root/admin privileges
    fn requires_privilege(&self) -> bool {
        false
    }
    
    /// Check if scanner is available on current system
    fn is_available(&self) -> bool {
        self.platforms().contains(&Platform::current())
    }
    
    /// Get scanner info as struct
    fn info(&self) -> ScannerInfo {
        ScannerInfo {
            name: self.name().to_string(),
            description: self.description().to_string(),
            platforms: self.platforms().to_vec(),
            capabilities: self.capabilities(),
            requires_privilege: self.requires_privilege(),
            available: self.is_available(),
        }
    }
}

/// Plugin entry point trait
pub trait ScannerPlugin: Send + Sync {
    /// Create scanner instance
    fn create(&self) -> Box<dyn Scanner>;
    
    /// Plugin name
    fn name(&self) -> &'static str;
    
    /// Plugin version
    fn version(&self) -> &'static str;
}

/// Type alias for plugin constructor
pub type PluginConstructor = fn() -> Box<dyn Scanner>;
