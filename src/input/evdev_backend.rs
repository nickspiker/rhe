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
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent, PhysicalKey, Thumb};
use std::ffi::CString;
use std::fs;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

// evdev event-type codes
const EV_KEY: u16 = 0x01;

// Positional scancodes from linux/input-event-codes.h — these are
// pre-xkb hardware positions, identical whether the user is on QWERTY,
// Dvorak, Colemak, or anything else.
const KEY_ESC: u16 = 1;
const KEY_A: u16 = 30;
const KEY_S: u16 = 31;
const KEY_D: u16 = 32;
const KEY_F: u16 = 33;
const KEY_J: u16 = 36;
const KEY_K: u16 = 37;
const KEY_L: u16 = 38;
const KEY_SEMICOLON: u16 = 39;
const KEY_SPACE: u16 = 57;
// PC keyboards put LEFTALT directly under the left thumb next to space
// (the Mac Command/⌘ position is LEFTMETA on those boards, but on a PC
// keyboard LEFTMETA is the Super/Win key one slot further out).
const KEY_LEFTALT: u16 = 56;

// EVIOCGRAB = _IOW('E', 0x90, int) — stable Linux ABI, x86-64 generic _IOC layout.
const EVIOCGRAB: libc::c_ulong = 0x40044590;

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

pub struct EvdevInput {
    pub rx: mpsc::Receiver<HidEvent>,
}

impl EvdevInput {
    pub fn start_grab(enabled: Arc<AtomicBool>) -> Result<Self, String> {
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

        for fd in grabbed {
            let tx = tx.clone();
            let enabled = enabled.clone();
            std::thread::spawn(move || reader_loop(fd, tx, enabled));
        }

        Ok(Self { rx })
    }
}

fn reader_loop(fd: RawFd, tx: mpsc::Sender<HidEvent>, enabled: Arc<AtomicBool>) {
    const BATCH: usize = 32;
    let mut buf: [InputEvent; BATCH] = unsafe { std::mem::zeroed() };
    let buf_bytes = std::mem::size_of::<[InputEvent; BATCH]>();
    let ev_size = std::mem::size_of::<InputEvent>();

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
            // value: 0=up, 1=down, 2=autorepeat — skip autorepeat (we want one press / one release).
            let dir = match ev.value {
                0 => KeyDirection::Up,
                1 => KeyDirection::Down,
                _ => continue,
            };

            if ev.code == KEY_ESC && dir == KeyDirection::Down {
                if enabled.load(Ordering::Relaxed) {
                    let _ = tx.send(HidEvent::Quit);
                }
                continue;
            }

            if !enabled.load(Ordering::Relaxed) {
                continue;
            }

            if let Some(key) = linux_to_physical(ev.code) {
                let _ = tx.send(HidEvent::Key(KeyEvent {
                    key,
                    direction: dir,
                }));
            }
        }
    }
    unsafe {
        libc::ioctl(fd, EVIOCGRAB, 0i32);
        libc::close(fd);
    }
}

fn linux_to_physical(code: u16) -> Option<PhysicalKey> {
    Some(match code {
        KEY_A => PhysicalKey::Finger(Hand::Left, Finger::Pinky),
        KEY_S => PhysicalKey::Finger(Hand::Left, Finger::Ring),
        KEY_D => PhysicalKey::Finger(Hand::Left, Finger::Middle),
        KEY_F => PhysicalKey::Finger(Hand::Left, Finger::Index),
        KEY_J => PhysicalKey::Finger(Hand::Right, Finger::Index),
        KEY_K => PhysicalKey::Finger(Hand::Right, Finger::Middle),
        KEY_L => PhysicalKey::Finger(Hand::Right, Finger::Ring),
        KEY_SEMICOLON => PhysicalKey::Finger(Hand::Right, Finger::Pinky),
        KEY_SPACE => PhysicalKey::Thumb(Thumb::Space),
        KEY_LEFTALT => PhysicalKey::Thumb(Thumb::Mod),
        _ => return None,
    })
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
