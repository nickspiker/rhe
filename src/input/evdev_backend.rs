//! evdev-based keyboard input for Linux.
//!
//! Opens every keyboard device under `/dev/input/event*`, `EVIOCGRAB`s each
//! one exclusively, and reads raw scancodes directly from the kernel — this
//! sits below xkb, so the user's active layout (Dvorak/Colemak/etc) does
//! not remap the stream. One thread per grabbed device feeds a shared mpsc
//! channel. Purely event-driven: each `read()` blocks until the kernel
//! delivers an `input_event`.
//!
//! Requires the running user to have read access to `/dev/input/event*`
//! (typically membership in the `input` group).

use super::HidEvent;
use crate::hand::{KeyDirection, KeyEvent};
use crate::layout;
use std::ffi::CString;
use std::fs;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

// evdev event-type codes
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_LED: u16 = 0x11;
const SYN_REPORT: u16 = 0x00;
// LED codes: 0 = num-lock, 1 = caps-lock, 2 = scroll-lock.
// All are potentially reconciled by xkb against its own state; scroll-lock
// is usually the least-managed, but whichever works on the user's box.
const LED_CODE: u16 = 0x00; // LED_NUML
const BUS_VIRTUAL: u16 = 0x06;

// Positional scancodes from linux/input-event-codes.h — these are
// pre-xkb hardware positions, identical whether the user is on QWERTY,
// Dvorak, Colemak, or anything else. The layout-specific chord key
// mappings live in `crate::layout`; only non-layout keys are kept here.
const KEY_ESC: u16 = 1;
const KEY_CAPSLOCK: u16 = 58;
/// Keyboard-detection probe. Any standard keyboard advertises KEY_A in
/// its EV_KEY bitmap; mice don't. The role this position plays in rhe
/// depends on the layout — see `layout::linux_to_role`.
const KEY_A: u16 = 30;
const KEY_ENTER: u16 = 28;

// EVIOCGRAB = _IOW('E', 0x90, int) — stable Linux ABI, x86-64 generic _IOC layout.
const EVIOCGRAB: libc::c_ulong = 0x40044590;

// uinput ioctls (type 'U' = 0x55). Stable Linux ABI.
const UI_SET_EVBIT: libc::c_ulong = 0x40045564; // _IOW('U', 100, int)
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565; // _IOW('U', 101, int)
const UI_DEV_CREATE: libc::c_ulong = 0x5501; // _IO ('U',   1)
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503; // _IOW('U',   3, struct uinput_setup)

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

// EVIOCGBIT(ev, len) = _IOC(_IOC_READ=2, type='E', nr=0x20+ev, size=len).
// Generic _IOC layout: (dir << 30) | (size << 16) | (type << 8) | nr.
fn eviocgbit(ev: u16, len: u32) -> libc::c_ulong {
    let len = len & 0x3FFF;
    ((2u32 << 30) | (len << 16) | (0x45u32 << 8) | (0x20 + ev as u32)) as libc::c_ulong
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

/// How the user signals "quit rhe" and whether caps lock toggles the grab.
///
/// In both `CapsLockPlusEsc` and `EscOrCapsPlusEsc`, Caps Lock is
/// intercepted and a solo tap toggles the `enabled` flag (passthrough on/
/// off). `EscAlone` leaves caps alone — for unit tests, not typical use.
#[derive(Clone, Copy)]
pub enum QuitTrigger {
    /// Esc alone quits. Caps lock passes through to OS, doesn't toggle.
    EscAlone,
    /// Caps+Esc quits, Esc alone passes through. Caps taps toggle enabled.
    /// Used by `rhe run` where Esc is needed by the focused app (vim, etc).
    CapsLockPlusEsc,
    /// Either Esc alone OR Caps+Esc quits. Caps taps toggle enabled.
    /// Used by the tutor so Esc still exits traditionally, *and* the user
    /// can tap caps to pause the tutor and type normally for a moment.
    EscOrCapsPlusEsc,
}

/// Callback fired when a caps-lock solo-tap toggles the `enabled` flag.
/// Lets the tray (or any other UI) refresh itself without polling.
pub type ToggleHook = Arc<dyn Fn() + Send + Sync + 'static>;

pub struct EvdevInput {
    pub rx: mpsc::Receiver<HidEvent>,
}

impl EvdevInput {
    pub fn start_grab(
        enabled: Arc<AtomicBool>,
        quit: QuitTrigger,
        on_toggle: Option<ToggleHook>,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        let devices =
            find_keyboards().map_err(|e| format!("evdev: failed to scan /dev/input: {}", e))?;
        if devices.is_empty() {
            return Err(
                "evdev: no keyboard devices found — is the user in the `input` group?".into(),
            );
        }

        let mut grabbed = Vec::new();
        for path in &devices {
            match open_and_grab(path) {
                Ok(fd) => grabbed.push(fd),
                Err(e) => eprintln!("evdev: skipping {}: {}", path, e),
            }
        }
        if grabbed.is_empty() {
            return Err("evdev: could not grab any keyboard device".into());
        }

        eprintln!("evdev: grabbed {} keyboard device(s)", grabbed.len());

        // Open a uinput virtual keyboard for passing non-rhe keys back to the OS.
        // If this fails (no permission on /dev/uinput) we still run, we just
        // can't forward — ESC will quit the grab.
        let uinput_fd = match open_uinput() {
            Ok(fd) => Some(fd),
            Err(e) => {
                eprintln!("evdev: passthrough disabled ({}); non-rhe keys will be swallowed", e);
                None
            }
        };

        // In caps-is-rhe-key mode, set the scroll-lock LED to mirror the
        // initial passthrough state. "LED on = typing normally" → so LED
        // is off when rhe starts enabled.
        let caps_controls_led = matches!(
            quit,
            QuitTrigger::CapsLockPlusEsc | QuitTrigger::EscOrCapsPlusEsc
        );
        if caps_controls_led {
            let led_on = !enabled.load(Ordering::Relaxed);
            for &fd in &grabbed {
                set_scroll_led(fd, led_on);
            }
        }

        for fd in grabbed {
            let tx = tx.clone();
            let enabled = enabled.clone();
            let on_toggle = on_toggle.clone();
            std::thread::spawn(move || reader_loop(fd, uinput_fd, tx, enabled, quit, on_toggle));
        }

        Ok(Self { rx })
    }
}

fn reader_loop(
    fd: RawFd,
    uinput_fd: Option<RawFd>,
    tx: mpsc::Sender<HidEvent>,
    enabled: Arc<AtomicBool>,
    quit: QuitTrigger,
    on_toggle: Option<ToggleHook>,
) {
    const BATCH: usize = 32;
    let mut buf: [InputEvent; BATCH] = unsafe { std::mem::zeroed() };
    let buf_bytes = std::mem::size_of::<[InputEvent; BATCH]>();
    let ev_size = std::mem::size_of::<InputEvent>();

    // Caps-lock state tracking:
    //   caps_held: key physically down right now (used to gate caps+Esc quit).
    //   caps_solo: true since last caps-down, cleared by any other key-down.
    //              On caps-up, if still true, it was a solo tap → toggle rhe.
    // In CapsLockPlusEsc mode we intercept caps entirely (OS never sees it);
    // in EscAlone mode caps passes through and doesn't toggle.
    let mut caps_held = false;
    let mut caps_solo = false;
    let caps_is_rhe_key = matches!(
        quit,
        QuitTrigger::CapsLockPlusEsc | QuitTrigger::EscOrCapsPlusEsc
    );

    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf_bytes) };
        if n <= 0 {
            break;
        }
        let count = n as usize / ev_size;
        for ev in &buf[..count] {
            if ev.type_ != EV_KEY {
                continue;
            }

            // Caps-lock handling (intercept in run mode, passthrough in tutor).
            if ev.code == KEY_CAPSLOCK {
                match ev.value {
                    1 => {
                        caps_held = true;
                        caps_solo = true;
                    }
                    0 => {
                        caps_held = false;
                        if caps_is_rhe_key && caps_solo {
                            let was = enabled.fetch_xor(true, Ordering::Relaxed);
                            let now_enabled = !was;
                            eprintln!(
                                "rhe: {} (caps-lock tap)",
                                if now_enabled { "enabled" } else { "disabled" }
                            );
                            set_scroll_led(fd, !now_enabled);
                            if let Some(hook) = on_toggle.as_ref() {
                                hook();
                            }
                        }
                        caps_solo = false;
                    }
                    _ => {} // autorepeat — no state change
                }
                if caps_is_rhe_key {
                    continue; // intercept caps; OS doesn't see it
                }
                // tutor mode: fall through to passthrough
            } else if ev.value == 1 {
                // Any non-caps key-down breaks solo-caps tracking.
                caps_solo = false;
            }

            // Wide layouts put a chord key on physical Enter, so one
            // of the shift keys is remapped to synthesize Enter (else
            // the user loses newline). Runs before everything else —
            // the synth is a passthrough with a rewritten code, not a
            // chord event, and it bypasses both rhe and caps-lock.
            if let Some(synth_key) = layout::linux_enter_synth_key() {
                if ev.code == synth_key {
                    if let Some(ufd) = uinput_fd {
                        forward_key(ufd, KEY_ENTER, ev.value);
                    }
                    continue;
                }
            }

            let rhe_scan = layout::linux_to_role(ev.code);
            let active = enabled.load(Ordering::Relaxed);

            // Quit gesture. Esc down only; Esc up + autorepeat ignored.
            if ev.code == KEY_ESC && ev.value == 1 {
                let should_quit = match quit {
                    QuitTrigger::EscAlone => active,
                    QuitTrigger::CapsLockPlusEsc => caps_held,
                    // Tutor: Esc always quits, regardless of enabled state.
                    // If caps-toggled rhe off and user hits Esc, they still
                    // expect the tutor to exit.
                    QuitTrigger::EscOrCapsPlusEsc => true,
                };
                if should_quit {
                    let _ = tx.send(HidEvent::Quit);
                    continue;
                }
                // else: fall through to passthrough so Esc reaches the app.
            }

            // rhe's own keys, when active: consume, don't forward.
            if active {
                if let Some(scan) = rhe_scan {
                    let dir = match ev.value {
                        0 => KeyDirection::Up,
                        1 => KeyDirection::Down,
                        _ => continue, // skip autorepeat on chord keys
                    };
                    let _ = tx.send(HidEvent::Key(KeyEvent { scan, direction: dir }));
                    continue;
                }
            }

            // Anything else → re-inject into the OS via uinput.
            if let Some(ufd) = uinput_fd {
                forward_key(ufd, ev.code, ev.value);
            }
        }
    }
    unsafe {
        libc::ioctl(fd, EVIOCGRAB, 0i32);
        libc::close(fd);
    }
}

/// Write the indicator-LED state to a grabbed keyboard. Which physical
/// LED lights up depends on `LED_CODE` (num-lock by default; scroll-lock
/// and caps-lock also possible but often xkb-managed).
fn set_scroll_led(fd: RawFd, on: bool) {
    let led_ev = InputEvent {
        time: libc::timeval { tv_sec: 0, tv_usec: 0 },
        type_: EV_LED,
        code: LED_CODE,
        value: if on { 1 } else { 0 },
    };
    let syn_ev = InputEvent {
        time: libc::timeval { tv_sec: 0, tv_usec: 0 },
        type_: EV_SYN,
        code: SYN_REPORT,
        value: 0,
    };
    let ev_size = std::mem::size_of::<InputEvent>();
    unsafe {
        libc::write(fd, &led_ev as *const _ as *const libc::c_void, ev_size);
        libc::write(fd, &syn_ev as *const _ as *const libc::c_void, ev_size);
    }
}

fn forward_key(uinput_fd: RawFd, code: u16, value: i32) {
    let key_ev = InputEvent {
        time: libc::timeval { tv_sec: 0, tv_usec: 0 },
        type_: EV_KEY,
        code,
        value,
    };
    let syn_ev = InputEvent {
        time: libc::timeval { tv_sec: 0, tv_usec: 0 },
        type_: EV_SYN,
        code: SYN_REPORT,
        value: 0,
    };
    let ev_size = std::mem::size_of::<InputEvent>();
    unsafe {
        libc::write(uinput_fd, &key_ev as *const _ as *const libc::c_void, ev_size);
        libc::write(uinput_fd, &syn_ev as *const _ as *const libc::c_void, ev_size);
    }
}

/// Create a virtual keyboard via `/dev/uinput`. Returns its fd, which we
/// write `input_event` records to when forwarding non-rhe keys back to the OS.
///
/// Requires write access to `/dev/uinput` (commonly root; can be granted
/// to the `input` group via a udev rule:
/// `KERNEL=="uinput", GROUP="input", MODE="0660"`).
fn open_uinput() -> Result<RawFd, String> {
    let path = CString::new("/dev/uinput").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
    if fd < 0 {
        return Err(format!("open /dev/uinput: {}", std::io::Error::last_os_error()));
    }

    unsafe {
        libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as i32);
        libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as i32);
        // Register every key code up through KEY_MICMUTE (0xF8). Covers the
        // full standard keyboard range without needing per-key logic.
        for code in 1..=0xF8i32 {
            libc::ioctl(fd, UI_SET_KEYBIT, code);
        }
    }

    let mut setup: UinputSetup = unsafe { std::mem::zeroed() };
    setup.id.bustype = BUS_VIRTUAL;
    setup.id.vendor = 0x7268; // "rh"
    setup.id.product = 0x6500; // "e\0"
    setup.id.version = 1;
    let name = b"rhe passthrough";
    setup.name[..name.len()].copy_from_slice(name);

    let rc = unsafe {
        libc::ioctl(fd, UI_DEV_SETUP, &setup as *const UinputSetup as *const libc::c_void)
    };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(format!("UI_DEV_SETUP: {}", err));
    }

    let rc = unsafe { libc::ioctl(fd, UI_DEV_CREATE) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(format!("UI_DEV_CREATE: {}", err));
    }

    Ok(fd)
}


fn find_keyboards() -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in fs::read_dir("/dev/input")? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("event") {
            continue;
        }
        // Skip our own uinput devices — grabbing them creates a feedback loop
        // where every char we emit comes right back in as input.
        let dev_name = std::fs::read_to_string(format!("/sys/class/input/{}/device/name", name))
            .unwrap_or_default();
        if dev_name.trim().starts_with("rhe ") {
            continue;
        }
        let path = format!("/dev/input/{}", name);
        if is_keyboard(&path) {
            out.push(path);
        }
    }
    Ok(out)
}

/// A device is considered a keyboard if it advertises KEY_A in its
/// EV_KEY capability bitmap. Mice don't claim KEY_A (their button codes
/// live at BTN_LEFT=0x110+), so this cleanly filters pointers out.
fn is_keyboard(path: &str) -> bool {
    let cpath = match CString::new(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK) };
    if fd < 0 {
        return false;
    }

    let mut bits = [0u8; 32];
    let rc = unsafe {
        libc::ioctl(
            fd,
            eviocgbit(EV_KEY, bits.len() as u32),
            bits.as_mut_ptr() as *mut libc::c_void,
        )
    };
    unsafe { libc::close(fd) };
    if rc < 0 {
        return false;
    }
    let byte = (KEY_A / 8) as usize;
    let bit = KEY_A % 8;
    bits[byte] & (1 << bit) != 0
}

fn open_and_grab(path: &str) -> Result<RawFd, String> {
    let cpath = CString::new(path).map_err(|e| e.to_string())?;
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err(format!("open: {}", std::io::Error::last_os_error()));
    }
    let rc = unsafe { libc::ioctl(fd, EVIOCGRAB, 1i32) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(format!("EVIOCGRAB: {}", err));
    }
    Ok(fd)
}
