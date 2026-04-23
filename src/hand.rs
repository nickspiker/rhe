//! Raw key events for the chord pipeline.
//!
//! Input backends translate their platform-specific key identifiers into
//! canonical scancodes (see `src/scan.rs`) and emit `KeyEvent { scan, direction }`.
//! From that point on, the pipeline never sees hand/finger or OS keycodes
//! again — everything speaks in scancode-space.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// Canonical scancode — bit index into `KeyMask`. See `src/scan.rs`.
    pub scan: u8,
    pub direction: KeyDirection,
}
