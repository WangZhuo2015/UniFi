fn main() {
    #[cfg(target_os = "macos")]
    {
        // Link CoreWLAN framework for WiFi scanning
        println!("cargo:rustc-link-lib=framework=CoreWLAN");
        println!("cargo:rustc-link-lib=framework=Security");
    }

    #[cfg(feature = "gui")]
    tauri_build::build()
}
