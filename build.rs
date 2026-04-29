fn main() {
    // CARGO_CFG_TARGET_OS reflects the *target* (cross-compile aware),
    // unlike `#[cfg(target_os = "...")]` inside build.rs which sees
    // the host OS instead. Required so cross-builds from Linux to
    // macOS still emit the IOKit / CoreFoundation framework links.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
