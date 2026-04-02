// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "gui")]
    unifi_lib::run();

    #[cfg(not(feature = "gui"))]
    unifi_lib::cli::run();
}
