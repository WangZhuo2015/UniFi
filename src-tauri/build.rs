fn main() {
    // Check target OS at build time via environment variable
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        // Link CoreWLAN framework for WiFi scanning
        println!("cargo:rustc-link-lib=framework=CoreWLAN");
        println!("cargo:rustc-link-lib=framework=Security");
    }

    #[cfg(feature = "gui")]
    tauri_build::build()
}
