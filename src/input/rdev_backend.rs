//! rdev-based keyboard input backend (legacy, macOS/Linux).

use crate::hand::{KeyDirection, KeyEvent};
use crate::scan;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// Cross-platform key event capture using the `rdev` crate.
///
/// Uses `grab` mode to intercept home-row keys when enabled.
/// When disabled, keys pass through to the OS normally.
///
/// On macOS: requires Accessibility permissions.
pub struct RdevInput {
    pub rx: mpsc::Receiver<KeyEvent>,
}

impl RdevInput {
    /// Start in grab mode with an enable/disable flag.
    ///
    /// When `enabled` is true: home-row keys are captured and suppressed.
    /// When `enabled` is false: all keys pass through normally.
    pub fn start_grab(enabled: Arc<AtomicBool>) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            rdev::grab(move |event| {
                if !enabled.load(Ordering::Relaxed) {
                    return Some(event); // pass through when disabled
                }

                if let Some(key_event) = convert(&event) {
                    let _ = tx.send(key_event);

                    // ⌘ always passes through to OS (for ⌘+C, ⌘+Tab, etc.)
                    // We capture the event for chord detection but don't suppress it
                    match event.event_type {
                        rdev::EventType::KeyPress(rdev::Key::MetaLeft | rdev::Key::MetaRight)
                        | rdev::EventType::KeyRelease(rdev::Key::MetaLeft | rdev::Key::MetaRight) =>
                        {
                            Some(event) // pass through
                        }
                        _ => None, // suppress finger keys and space
                    }
                } else {
                    Some(event) // pass through non-home-row keys
                }
            })
            .expect("rdev: failed to grab — check Accessibility permissions");
        });

        Ok(Self { rx })
    }

    /// Start in listen mode (non-suppressing, for debug).
    pub fn start_listen() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            rdev::listen(move |event| {
                if let Some(key_event) = convert(&event) {
                    let _ = tx.send(key_event);
                }
            })
            .expect("rdev: failed to listen — check permissions");
        });

        Ok(Self { rx })
    }
}

impl super::KeyInput for RdevInput {
    fn next_event(&mut self) -> Option<KeyEvent> {
        self.rx.recv().ok()
    }
}

/// Convert an rdev event to our KeyEvent, if it's a key we care about.
fn convert(event: &rdev::Event) -> Option<KeyEvent> {
    let (rdev_key, direction) = match event.event_type {
        rdev::EventType::KeyPress(key) => (key, KeyDirection::Down),
        rdev::EventType::KeyRelease(key) => (key, KeyDirection::Up),
        _ => return None,
    };

    let scan = rdev_to_scan(rdev_key)?;
    Some(KeyEvent { scan, direction })
}

/// Map rdev keys to rhe canonical scancodes (`src/scan.rs`).
fn rdev_to_scan(key: rdev::Key) -> Option<u8> {
    match key {
        // Left hand home row (QWERTY positions A S D F)
        rdev::Key::KeyA => Some(scan::L_PINKY),
        rdev::Key::KeyS => Some(scan::L_RING),
        rdev::Key::KeyD => Some(scan::L_MID),
        rdev::Key::KeyF => Some(scan::L_IDX),

        // Right hand home row (QWERTY positions J K L ;)
        rdev::Key::KeyJ => Some(scan::R_IDX),
        rdev::Key::KeyK => Some(scan::R_MID),
        rdev::Key::KeyL => Some(scan::R_RING),
        rdev::Key::SemiColon => Some(scan::R_PINKY),

        // Spacebar = right hand thumb / mod bit
        rdev::Key::Space => Some(scan::R_THUMB),
        // Left ⌘ = word boundary
        rdev::Key::MetaLeft => Some(scan::WORD),

        _ => None,
    }
}
