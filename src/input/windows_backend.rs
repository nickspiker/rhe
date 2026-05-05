//! Windows keyboard input grab via rdev's WH_KEYBOARD_LL hook.
//!
//! Mirrors evdev/cgevent backend semantics: when `enabled` is true, home-row chord keys are intercepted and forwarded to the chord state machine; the Win key (mode selector) and unrelated keys pass through. Esc fires a `HidEvent::Quit` so the tutor can shut down cleanly.
//!
//! Caps Lock is fully intercepted — the OS never sees it — and pressing-and-releasing Caps with no other key live toggles the `enabled` flag (rhe ↔ keyboard passthrough). Caps held while another key is pressed cancels the toggle so chorded shortcuts like Caps+Esc (= Quit) still work.

use crate::hand::{KeyDirection, KeyEvent};
use crate::input::HidEvent;
use crate::scan;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// Quit-signal selector matching the evdev backend's API. On Windows caps lock is always intercepted (we never let it reach the OS), so the only choice the variants make is *what quits*: Esc alone, caps+Esc only, or either.
#[derive(Clone, Copy)]
pub enum QuitTrigger {
    EscAlone,
    CapsLockPlusEsc,
    EscOrCapsPlusEsc,
}

pub type ToggleHook = Arc<dyn Fn() + Send + Sync + 'static>;

pub struct WindowsInput {
    pub rx: mpsc::Receiver<HidEvent>,
}

impl WindowsInput {
    pub fn start_grab(
        enabled: Arc<AtomicBool>,
        quit: QuitTrigger,
        on_toggle: Option<ToggleHook>,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            // rdev::grab callback is `Fn`, not `FnMut`, so caps-solo
            // bookkeeping lives in shared atomics. caps_held = caps
            // physically pressed; caps_solo = no other key seen since
            // caps went down (cleared on any non-caps press).
            let caps_held = Arc::new(AtomicBool::new(false));
            let caps_solo = Arc::new(AtomicBool::new(false));
            let caps_held_cb = caps_held.clone();
            let caps_solo_cb = caps_solo.clone();
            let enabled_cb = enabled.clone();
            let tx_cb = tx.clone();

            let result = rdev::grab(move |event| {
                use rdev::{EventType, Key};

                // Caps Lock — always intercepted. Solo press toggles enabled;
                // OS never sees the key so its CAPS state never flips.
                match event.event_type {
                    EventType::KeyPress(Key::CapsLock) => {
                        caps_held_cb.store(true, Ordering::Relaxed);
                        caps_solo_cb.store(true, Ordering::Relaxed);
                        return None;
                    }
                    EventType::KeyRelease(Key::CapsLock) => {
                        caps_held_cb.store(false, Ordering::Relaxed);
                        if caps_solo_cb.swap(false, Ordering::Relaxed) {
                            // Solo press → toggle rhe on/off
                            let was = enabled_cb.fetch_xor(true, Ordering::Relaxed);
                            let _ = was;
                            if let Some(hook) = on_toggle.as_ref() {
                                hook();
                            }
                        }
                        return None;
                    }
                    _ => {}
                }

                // Esc — quit gesture. Whether plain Esc quits depends
                // on the QuitTrigger; caps+Esc always does.
                if matches!(event.event_type, EventType::KeyPress(Key::Escape)) {
                    // Any non-caps press breaks the solo flag.
                    caps_solo_cb.store(false, Ordering::Relaxed);
                    let caps_now = caps_held_cb.load(Ordering::Relaxed);
                    let should_quit = match quit {
                        QuitTrigger::EscAlone => true,
                        QuitTrigger::CapsLockPlusEsc => caps_now,
                        QuitTrigger::EscOrCapsPlusEsc => true,
                    };
                    if should_quit {
                        let _ = tx_cb.send(HidEvent::Quit);
                        return None;
                    }
                }

                // Any other key-down breaks the caps-solo tracking so a
                // chorded press of caps+anything doesn't double as a
                // solo-toggle on release.
                if matches!(event.event_type, EventType::KeyPress(_)) {
                    caps_solo_cb.store(false, Ordering::Relaxed);
                }

                if !enabled_cb.load(Ordering::Relaxed) {
                    return Some(event);
                }

                if let Some(key_event) = convert(&event) {
                    let _ = tx_cb.send(HidEvent::Key(key_event));
                    // Win key (WORD) passes through so OS shortcuts
                    // like Win+L still work; finger keys and space
                    // are swallowed.
                    match event.event_type {
                        EventType::KeyPress(Key::MetaLeft | Key::MetaRight)
                        | EventType::KeyRelease(Key::MetaLeft | Key::MetaRight) => Some(event),
                        _ => None,
                    }
                } else {
                    Some(event)
                }
            });
            if let Err(e) = result {
                crate::rerror!("rdev::grab failed: {:?}", e);
            }
        });

        Ok(Self { rx })
    }
}

fn convert(event: &rdev::Event) -> Option<KeyEvent> {
    let (rdev_key, direction) = match event.event_type {
        rdev::EventType::KeyPress(key) => (key, KeyDirection::Down),
        rdev::EventType::KeyRelease(key) => (key, KeyDirection::Up),
        _ => return None,
    };
    let scan = rdev_to_scan(rdev_key)?;
    Some(KeyEvent { scan, direction })
}

fn rdev_to_scan(key: rdev::Key) -> Option<u8> {
    match key {
        rdev::Key::KeyA => Some(scan::L_PINKY),
        rdev::Key::KeyS => Some(scan::L_RING),
        rdev::Key::KeyD => Some(scan::L_MID),
        rdev::Key::KeyF => Some(scan::L_IDX),
        rdev::Key::KeyJ => Some(scan::R_IDX),
        rdev::Key::KeyK => Some(scan::R_MID),
        rdev::Key::KeyL => Some(scan::R_RING),
        rdev::Key::SemiColon => Some(scan::R_PINKY),
        rdev::Key::Space => Some(scan::R_THUMB),
        // Left Win key as the WORD/mode-selector, mirroring macOS's
        // left ⌘. A solo press still opens Start; chord usage
        // (Win+letter) emits a chord event then suppresses Start.
        rdev::Key::MetaLeft => Some(scan::WORD),
        _ => None,
    }
}
