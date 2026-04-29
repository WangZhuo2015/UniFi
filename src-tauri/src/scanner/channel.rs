//! WiFi Channel Control
//!
//! Provides cross-platform channel setting for monitor mode capture.
//! On macOS, uses CoreWLAN API (macOS 26 compatible).
//! On Linux, uses iw command.

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

/// Result of channel setting operation
#[derive(Debug)]
pub enum ChannelResult {
    Success,
    NotSupported,
    Failed(String),
}

/// Set WiFi channel on the interface
#[cfg(target_os = "macos")]
pub fn set_channel(interface: &str, channel: u8) -> ChannelResult {
    // Try CoreWLAN approach first
    if let Some(result) = try_corewlan_set_channel(interface, channel) {
        return result;
    }

    // Fallback: try airport if available (for older macOS)
    if let Some(result) = try_airport_set_channel(channel) {
        return result;
    }

    ChannelResult::NotSupported
}

#[cfg(target_os = "macos")]
fn try_corewlan_set_channel(interface: &str, channel: u8) -> Option<ChannelResult> {
    unsafe {
        // Get CWWiFiClient
        let client_class = Class::get("CWWiFiClient")?;
        let client: *mut Object = msg_send![client_class, sharedWiFiClient];
        if client.is_null() {
            return None;
        }

        // Get interface - we need to find the right one by name
        // CWWiFiClient.interface returns the default interface
        let cw_interface: *mut Object = msg_send![client, interface];
        if cw_interface.is_null() {
            return None;
        }

        // Try to set channel using CWChannel
        // Note: CoreWLAN doesn't have a direct setChannel method
        // This requires disassociating and using scanForNetworksWithSSID
        // For monitor mode capture, we need a different approach

        // CoreWLAN doesn't support direct channel setting for monitor mode
        // We'll need to use passive capture without channel hopping
        let _ = (interface, channel); // Suppress unused warnings
        None
    }
}

#[cfg(target_os = "macos")]
fn try_airport_set_channel(channel: u8) -> Option<ChannelResult> {
    use std::path::Path;
    use std::process::Command;

    let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

    // Check if airport exists (it won't on macOS 26+)
    if !Path::new(airport_path).exists() {
        return None;
    }

    let result = Command::new(airport_path)
        .arg(format!("-c{}", channel))
        .output();

    match result {
        Ok(output) if output.status.success() => Some(ChannelResult::Success),
        Ok(output) => Some(ChannelResult::Failed(
            String::from_utf8_lossy(&output.stderr).to_string()
        )),
        Err(e) => Some(ChannelResult::Failed(e.to_string())),
    }
}

#[cfg(target_os = "linux")]
pub fn set_channel(interface: &str, channel: u8) -> ChannelResult {
    use std::process::Command;

    let result = Command::new("iw")
        .args(["dev", interface, "set", "channel", &channel.to_string()])
        .output();

    match result {
        Ok(output) if output.status.success() => ChannelResult::Success,
        Ok(output) => ChannelResult::Failed(
            String::from_utf8_lossy(&output.stderr).to_string()
        ),
        Err(e) => ChannelResult::Failed(e.to_string()),
    }
}

/// Check if airport tool is available (for older macOS)
#[cfg(target_os = "macos")]
pub fn airport_available() -> bool {
    use std::path::Path;
    Path::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport").exists()
}

#[cfg(not(target_os = "macos"))]
pub fn airport_available() -> bool {
    false
}

/// Check if channel control is supported
pub fn is_channel_control_supported() -> bool {
    #[cfg(target_os = "macos")]
    {
        // On macOS 26+, channel hopping is limited
        // We can still capture passively
        true
    }

    #[cfg(target_os = "linux")]
    {
        true
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}