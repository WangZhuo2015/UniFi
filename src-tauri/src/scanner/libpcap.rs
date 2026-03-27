//! macOS Libpcap Scanner (Reserved)
//!
//! Future implementation using libpcap for beacon frame capture.
//! This will be used when airport access is restricted.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::ScanError;

pub struct LibpcapScanner;

impl Scanner for LibpcapScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        Err(ScanError::NotSupported)
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        Err(ScanError::NotSupported)
    }
    
    fn name(&self) -> &'static str {
        "macOS Libpcap (not implemented)"
    }
    
    fn requires_privilege(&self) -> bool {
        true
    }
}
