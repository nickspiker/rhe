//! Platform-agnostic text output trait.

/// Trait for injecting text into the focused application.
pub trait TextOutput {
    fn emit(&self, text: &str);
    fn backspace(&self, count: usize);
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

/// No-op output backend for platforms where text injection isn't wired up. Kept as a placeholder + reference impl of the `TextOutput` trait; no current call site constructs it.
#[allow(dead_code)]
pub struct NullOutput;

#[allow(dead_code)]
impl NullOutput {
    pub fn new() -> Self {
        Self
    }
}

impl TextOutput for NullOutput {
    fn emit(&self, _text: &str) {}
    fn backspace(&self, _count: usize) {}
}
