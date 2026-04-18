use crate::hand::{KeyDirection, KeyEvent, PhysicalKey, Hand, Finger, Thumb};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// Cross-platform key event capture using the `rdev` crate.
///
/// Uses `grab` mode to intercept home-row keys when enabled.
/// When disabled, keys pass through to the OS normally.
///
/// On macOS: requires Accessibility permissions.
pub struct RdevInput {
    rx: mpsc::Receiver<KeyEvent>,
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
                    None // suppress when enabled
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
    let (rdev_key, dir) = match event.event_type {
        rdev::EventType::KeyPress(key) => (key, KeyDirection::Down),
        rdev::EventType::KeyRelease(key) => (key, KeyDirection::Up),
        _ => return None,
    };

    let physical = rdev_to_physical(rdev_key)?;
    Some(KeyEvent {
        key: physical,
        direction: dir,
    })
}

/// Map rdev keys to our physical key model.
fn rdev_to_physical(key: rdev::Key) -> Option<PhysicalKey> {
    match key {
        // Left hand home row (QWERTY positions A S D F)
        rdev::Key::KeyA => Some(PhysicalKey::Finger(Hand::Left, Finger::Pinky)),
        rdev::Key::KeyS => Some(PhysicalKey::Finger(Hand::Left, Finger::Ring)),
        rdev::Key::KeyD => Some(PhysicalKey::Finger(Hand::Left, Finger::Middle)),
        rdev::Key::KeyF => Some(PhysicalKey::Finger(Hand::Left, Finger::Index)),

        // Right hand home row (QWERTY positions J K L ;)
        rdev::Key::KeyJ => Some(PhysicalKey::Finger(Hand::Right, Finger::Index)),
        rdev::Key::KeyK => Some(PhysicalKey::Finger(Hand::Right, Finger::Middle)),
        rdev::Key::KeyL => Some(PhysicalKey::Finger(Hand::Right, Finger::Ring)),
        rdev::Key::SemiColon => Some(PhysicalKey::Finger(Hand::Right, Finger::Pinky)),

        // Thumbs
        rdev::Key::Space => Some(PhysicalKey::Thumb(Thumb::Space)),
        rdev::Key::ControlLeft => Some(PhysicalKey::Thumb(Thumb::Ctrl)),

        _ => None,
    }
}
