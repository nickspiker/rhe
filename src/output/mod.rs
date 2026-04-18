/// Trait for injecting text into the focused application.
pub trait TextOutput {
    fn emit(&self, text: &str);
    fn backspace(&self, count: usize);
}

#[cfg(target_os = "macos")]
pub mod macos;
