//! Scanner registry for plugin management

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{Platform, Scanner, ScannerCapabilities};

/// Scanner information
#[derive(Debug, Clone)]
pub struct ScannerInfo {
    /// Unique scanner name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Supported platforms
    pub platforms: Vec<Platform>,
    /// Scanner capabilities
    pub capabilities: ScannerCapabilities,
    /// Requires elevated privileges
    pub requires_privilege: bool,
    /// Is available on current system
    pub available: bool,
}

/// Scanner registry for managing plugins
pub struct ScannerRegistry {
    scanners: RwLock<HashMap<String, Arc<Box<dyn Scanner>>>>,
    order: RwLock<Vec<String>>,
}

impl Default for ScannerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScannerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            scanners: RwLock::new(HashMap::new()),
            order: RwLock::new(Vec::new()),
        }
    }
    
    /// Register a scanner
    pub fn register(&self, scanner: Box<dyn Scanner>) {
        let name = scanner.name().to_string();
        let info = scanner.info();
        
        // Update default scanner preference order
        if info.capabilities.has_ie_data && !info.requires_privilege {
            // Prefer scanners with IE data that don't require privileges
            let mut order = self.order.write().unwrap();
            order.insert(0, name.clone());
        } else {
            let mut order = self.order.write().unwrap();
            order.push(name.clone());
        }
        
        let mut scanners = self.scanners.write().unwrap();
        scanners.insert(name, Arc::new(scanner));
    }
    
    /// Get a scanner by name
    pub fn get(&self, name: &str) -> Option<Arc<Box<dyn Scanner>>> {
        let scanners = self.scanners.read().unwrap();
        scanners.get(name).cloned()
    }
    
    /// Get the default scanner for the current platform
    pub fn get_default(&self) -> Option<Arc<Box<dyn Scanner>>> {
        let order = self.order.read().unwrap();
        let scanners = self.scanners.read().unwrap();
        
        for name in order.iter() {
            if let Some(scanner) = scanners.get(name) {
                if scanner.is_available() {
                    return Some(scanner.clone());
                }
            }
        }
        
        None
    }
    
    /// Get the best available scanner (prefers IE data)
    pub fn get_best(&self) -> Option<Arc<Box<dyn Scanner>>> {
        let scanners = self.scanners.read().unwrap();
        
        // First, try to find a scanner with IE data that's available
        let best = scanners.values()
            .filter(|s| s.is_available())
            .max_by(|a, b| {
                let a_score = Self::score_scanner(a);
                let b_score = Self::score_scanner(b);
                a_score.cmp(&b_score)
            });
        
        best.cloned()
    }
    
    fn score_scanner(scanner: &Box<dyn Scanner>) -> u32 {
        let caps = scanner.capabilities();
        let mut score = 0u32;
        
        // Prefer scanners with IE data
        if caps.has_ie_data { score += 100; }
        // Prefer scanners with BSSID
        if caps.has_bssid { score += 50; }
        // Prefer scanners with signal strength
        if caps.has_signal { score += 30; }
        // Prefer scanners that don't require privileges
        if !scanner.requires_privilege() { score += 70; }
        // Prefer App Store compatible scanners for macOS
        if caps.app_store_compatible { score += 20; }
        
        score
    }
    
    /// List all registered scanners
    pub fn list(&self) -> Vec<ScannerInfo> {
        let scanners = self.scanners.read().unwrap();
        scanners.values().map(|s| s.info()).collect()
    }
    
    /// List available scanners on current platform
    pub fn list_available(&self) -> Vec<ScannerInfo> {
        let scanners = self.scanners.read().unwrap();
        scanners.values()
            .filter(|s| s.is_available())
            .map(|s| s.info())
            .collect()
    }
    
    /// Get scanner count
    pub fn count(&self) -> usize {
        self.scanners.read().unwrap().len()
    }
    
    /// Clear all registered scanners
    pub fn clear(&self) {
        let mut scanners = self.scanners.write().unwrap();
        let mut order = self.order.write().unwrap();
        scanners.clear();
        order.clear();
    }
    
    /// Unregister a scanner by name
    pub fn unregister(&self, name: &str) -> bool {
        let mut scanners = self.scanners.write().unwrap();
        let mut order = self.order.write().unwrap();
        
        order.retain(|n| n != name);
        scanners.remove(name).is_some()
    }
}

/// Global scanner registry
#[allow(dead_code)]
static REGISTRY: std::sync::OnceLock<ScannerRegistry> = std::sync::OnceLock::new();

/// Get the global scanner registry
#[allow(dead_code)]
pub fn global_registry() -> &'static ScannerRegistry {
    REGISTRY.get_or_init(ScannerRegistry::new)
}

/// Register a scanner with the global registry
#[allow(dead_code)]
pub fn register_scanner(scanner: Box<dyn Scanner>) {
    global_registry().register(scanner);
}

/// Get a scanner from the global registry
#[allow(dead_code)]
pub fn get_scanner(name: &str) -> Option<Arc<Box<dyn Scanner>>> {
    global_registry().get(name)
}

/// Get the default scanner from the global registry
#[allow(dead_code)]
pub fn get_default_scanner() -> Option<Arc<Box<dyn Scanner>>> {
    global_registry().get_default()
}

/// Get the best available scanner
#[allow(dead_code)]
pub fn get_best_scanner() -> Option<Arc<Box<dyn Scanner>>> {
    global_registry().get_best()
}

/// Macro to register a scanner
#[macro_export]
macro_rules! register_scanner {
    ($scanner:expr) => {
        $crate::register_scanner(Box::new($scanner))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RawBeacon, ScanError};
    
    struct MockScanner {
        name: &'static str,
        has_ie: bool,
        needs_privilege: bool,
    }
    
    impl Scanner for MockScanner {
        fn name(&self) -> &'static str { self.name }
        
        fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
            Ok(vec![])
        }
        
        fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
            Ok(None)
        }
        
        fn platforms(&self) -> &'static [Platform] {
            &[Platform::MacOS]
        }
        
        fn capabilities(&self) -> ScannerCapabilities {
            ScannerCapabilities {
                has_ie_data: self.has_ie,
                has_bssid: true,
                has_signal: true,
                has_security: true,
                app_store_compatible: !self.needs_privilege,
            }
        }
        
        fn requires_privilege(&self) -> bool {
            self.needs_privilege
        }
    }
    
    #[test]
    fn test_registry() {
        let registry = ScannerRegistry::new();
        
        registry.register(Box::new(MockScanner {
            name: "test1",
            has_ie: true,
            needs_privilege: false,
        }));
        
        registry.register(Box::new(MockScanner {
            name: "test2",
            has_ie: false,
            needs_privilege: false,
        }));
        
        assert_eq!(registry.count(), 2);
        assert!(registry.get("test1").is_some());
        assert!(registry.get("test2").is_some());
        assert!(registry.get("test3").is_none());
    }
    
    #[test]
    fn test_scanner_scoring() {
        let scanner1: Box<dyn Scanner> = Box::new(MockScanner {
            name: "scanner1",
            has_ie: true,
            needs_privilege: false,
        });
        let scanner2: Box<dyn Scanner> = Box::new(MockScanner {
            name: "scanner2",
            has_ie: false,
            needs_privilege: true,
        });

        let score1 = ScannerRegistry::score_scanner(&scanner1);
        let score2 = ScannerRegistry::score_scanner(&scanner2);
        
        // Scanner with IE data and no privilege requirement should score higher
        assert!(score1 > score2);
    }
    
    #[test]
    fn test_unregister() {
        let registry = ScannerRegistry::new();
        
        registry.register(Box::new(MockScanner {
            name: "test",
            has_ie: true,
            needs_privilege: false,
        }));
        
        assert_eq!(registry.count(), 1);
        
        let removed = registry.unregister("test");
        assert!(removed);
        assert_eq!(registry.count(), 0);
        
        let removed_again = registry.unregister("test");
        assert!(!removed_again);
    }
    
    #[test]
    fn test_list_available() {
        let registry = ScannerRegistry::new();
        
        registry.register(Box::new(MockScanner {
            name: "available",
            has_ie: true,
            needs_privilege: false,
        }));
        
        let available = registry.list_available();
        // On non-macOS, this might be empty since MockScanner only supports macOS
        // But on macOS in tests, it should show up
        #[cfg(target_os = "macos")]
        assert!(!available.is_empty());
    }
}
