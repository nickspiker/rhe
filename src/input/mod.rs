//! Platform-agnostic input trait and `HidEvent` type.

use crate::hand::KeyEvent;

/// Platform-agnostic input trait.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub trait KeyInput {
    /// Block until the next key event. Returns None if the source is closed.
    fn next_event(&mut self) -> Option<KeyEvent>;
}

/// Events emitted by a grabbing input backend.
///
/// `Quit` is a synthetic escape signal — the grabbing backend swallows Esc
/// so the tutor can still exit cleanly without the keystroke reaching apps.
#[derive(Debug, Clone, Copy)]
pub enum HidEvent {
    Key(KeyEvent),
    Quit,
}

#[cfg(target_os = "macos")]
pub mod rdev_backend;

#[cfg(target_os = "macos")]
pub mod iohid_backend;

#[cfg(target_os = "macos")]
pub mod cgevent_backend;

#[cfg(target_os = "linux")]
pub mod evdev_backend;
