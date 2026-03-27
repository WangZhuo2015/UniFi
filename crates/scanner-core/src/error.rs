//! Scanner error types

use thiserror::Error;

/// Scanner error type
#[derive(Debug, Error)]
pub enum ScanError {
    /// Scanner not available on this platform
    #[error("Scanner '{0}' is not available on this platform")]
    NotAvailable(String),
    
    /// Permission denied (requires root/admin)
    #[error("Permission denied: {0}. This scanner requires elevated privileges.")]
    PermissionDenied(String),
    
    /// Interface not found
    #[error("Network interface not found: {0}")]
    InterfaceNotFound(String),
    
    /// Interface not in monitor mode
    #[error("Interface '{0}' is not in monitor mode")]
    NotMonitorMode(String),
    
    /// Scan timeout
    #[error("Scan timed out after {0}ms")]
    Timeout(u64),
    
    /// No networks found
    #[error("No networks found")]
    NoNetworks,
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// System error
    #[error("System error: {0}")]
    System(String),
    
    /// Platform-specific error
    #[error("{0}: {1}")]
    Platform(String, String),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    Config(String),
    
    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl ScanError {
    /// Create a platform-specific error
    pub fn platform(platform: &str, msg: impl Into<String>) -> Self {
        ScanError::Platform(platform.to_string(), msg.into())
    }
    
    /// Check if error is due to permissions
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, ScanError::PermissionDenied(_))
    }
    
    /// Check if error is due to timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, ScanError::Timeout(_))
    }
    
    /// Check if scanner is not available
    pub fn is_not_available(&self) -> bool {
        matches!(self, ScanError::NotAvailable(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ScanError::PermissionDenied("libpcap requires root".to_string());
        assert!(err.to_string().contains("Permission denied"));
        
        let err = ScanError::Timeout(5000);
        assert!(err.to_string().contains("5000ms"));
    }
    
    #[test]
    fn test_error_checks() {
        let err = ScanError::PermissionDenied("test".to_string());
        assert!(err.is_permission_denied());
        assert!(!err.is_timeout());
        
        let err = ScanError::Timeout(1000);
        assert!(err.is_timeout());
        assert!(!err.is_permission_denied());
        
        let err = ScanError::NotAvailable("test".to_string());
        assert!(err.is_not_available());
    }
    
    #[test]
    fn test_platform_error() {
        let err = ScanError::platform("macOS", "airport not found");
        assert!(err.to_string().contains("macOS"));
        assert!(err.to_string().contains("airport not found"));
    }
}
