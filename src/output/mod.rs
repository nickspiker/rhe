/// Trait for injecting text into the focused application.
pub trait TextOutput {
    fn emit(&self, text: &str);
    fn backspace(&self, count: usize);
}

#[cfg(target_os = "macos")]
pub mod macos;

/// No-op output backend for platforms where text injection isn't wired up.
///
/// On Linux, uinput routes through the user's xkb layout (so injected
/// scancodes get re-interpreted by Dvorak/etc). Until we add xkb
/// reverse-mapping or an IME path, the tutor runs without emitting text.
pub struct NullOutput;

impl NullOutput {
    pub fn new() -> Self {
        Self
    }
}

impl TextOutput for NullOutput {
    fn emit(&self, _text: &str) {}
    fn backspace(&self, _count: usize) {}
}
