//! Linux text output via uinput + libxkbcommon reverse-mapping.
//!
//! Creates a dedicated virtual keyboard for emitting rhe's chord output,
//! and maps Rust `char`s back to the scancode+modifier sequences that will
//! produce them *under the user's active xkb layout*. So a Dvorak user
//! gets proper Dvorak output; a Colemak user gets Colemak; etc.
//!
//! Characters not representable in the active layout (IPA, emoji, etc.)
//! fall back to `Ctrl+Shift+U <hex> Enter` — GTK/Qt/IBus unicode input —
//! which works in most modern GUI apps.
//!
//! The output virtual keyboard is created AFTER the evdev backend has
//! scanned for real keyboards, so it won't be self-grabbed.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::unix::io::RawFd;

// evdev scancodes we emit directly (not keycode+8, these are the kernel values).
const KEY_BACKSPACE: u16 = 14;
const KEY_ENTER: u16 = 28;
const KEY_U: u16 = 22;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTALT: u16 = 100; // AltGr

// evdev event types.
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const SYN_REPORT: u16 = 0x00;

// uinput ioctls (type 'U' = 0x55). Stable Linux ABI.
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503;

const BUS_VIRTUAL: u16 = 0x06;

// Modifier mask bits used by the reverse lookup table.
const MOD_SHIFT: u8 = 0b001;
const MOD_ALTGR: u8 = 0b010;

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

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct LinuxOutput {
    fd: RawFd,
    /// char → (kernel scancode, modifier mask). Produces the character
    /// when the modifiers are held and the key is tapped.
    reverse_map: HashMap<char, (u16, u8)>,
}

impl LinuxOutput {
    pub fn new() -> Self {
        let fd = open_uinput().unwrap_or_else(|e| {
            eprintln!(
                "rhe: /dev/uinput unavailable ({}); output disabled.\n\
                 fix: sudo tee /etc/udev/rules.d/99-uinput.rules <<<\\\n\
                     'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"'",
                e
            );
            -1
        });

        let reverse_map = match build_reverse_map() {
            Ok(m) => {
                eprintln!("rhe: xkb reverse-map built ({} chars)", m.len());
                m
            }
            Err(e) => {
                eprintln!(
                    "rhe: could not build xkb keymap ({}); unicode fallback only",
                    e
                );
                HashMap::new()
            }
        };

        Self { fd, reverse_map }
    }

    fn emit_key(&self, code: u16, value: i32) {
        if self.fd < 0 {
            return;
        }
        write_event(self.fd, EV_KEY, code, value);
        write_event(self.fd, EV_SYN, SYN_REPORT, 0);
    }

    fn tap(&self, code: u16) {
        self.emit_key(code, 1);
        self.emit_key(code, 0);
    }

    /// Emit a single char via scancode synthesis (reverse map) or via
    /// Ctrl+Shift+U <hex> Enter fallback for chars outside the keymap.
    fn emit_char(&self, c: char) {
        if let Some(&(code, mods)) = self.reverse_map.get(&c) {
            if mods & MOD_SHIFT != 0 {
                self.emit_key(KEY_LEFTSHIFT, 1);
            }
            if mods & MOD_ALTGR != 0 {
                self.emit_key(KEY_RIGHTALT, 1);
            }
            self.tap(code);
            if mods & MOD_ALTGR != 0 {
                self.emit_key(KEY_RIGHTALT, 0);
            }
            if mods & MOD_SHIFT != 0 {
                self.emit_key(KEY_LEFTSHIFT, 0);
            }
        } else {
            self.unicode_fallback(c);
        }
    }

    /// GTK/Qt/IBus unicode input: Ctrl+Shift+U (release), then hex digits,
    /// then Enter to commit. Works in most modern GUI apps.
    fn unicode_fallback(&self, c: char) {
        let codepoint = c as u32;
        // Start: Ctrl+Shift held, tap U, release Shift+Ctrl.
        self.emit_key(KEY_LEFTCTRL, 1);
        self.emit_key(KEY_LEFTSHIFT, 1);
        self.tap(KEY_U);
        self.emit_key(KEY_LEFTSHIFT, 0);
        self.emit_key(KEY_LEFTCTRL, 0);

        // Hex digits. Look each up in the reverse map; if missing, bail.
        for digit in format!("{:x}", codepoint).chars() {
            if let Some(&(code, mods)) = self.reverse_map.get(&digit) {
                if mods & MOD_SHIFT != 0 {
                    self.emit_key(KEY_LEFTSHIFT, 1);
                }
                self.tap(code);
                if mods & MOD_SHIFT != 0 {
                    self.emit_key(KEY_LEFTSHIFT, 0);
                }
            }
        }

        // Enter commits the unicode input.
        self.tap(KEY_ENTER);
    }
}

impl super::TextOutput for LinuxOutput {
    fn emit(&self, text: &str) {
        for c in text.chars() {
            self.emit_char(c);
        }
    }

    fn backspace(&self, count: usize) {
        for _ in 0..count {
            self.tap(KEY_BACKSPACE);
        }
    }
}

/// Open `/dev/uinput` and create a virtual keyboard advertising all the
/// scancodes we'll ever emit. Returns the fd on success.
fn open_uinput() -> Result<RawFd, String> {
    let path = CString::new("/dev/uinput").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
    if fd < 0 {
        return Err(format!("open /dev/uinput: {}", std::io::Error::last_os_error()));
    }

    unsafe {
        libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as i32);
        libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as i32);
        // Advertise every standard keyboard key (1..=KEY_MICMUTE).
        for code in 1..=0xF8i32 {
            libc::ioctl(fd, UI_SET_KEYBIT, code);
        }
    }

    let mut setup: UinputSetup = unsafe { std::mem::zeroed() };
    setup.id.bustype = BUS_VIRTUAL;
    setup.id.vendor = 0x7268; // "rh"
    setup.id.product = 0x6501; // "e\x01" — distinct from the passthrough device
    setup.id.version = 1;
    let name = b"rhe text output";
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

fn write_event(fd: RawFd, type_: u16, code: u16, value: i32) {
    let ev = InputEvent {
        time: libc::timeval { tv_sec: 0, tv_usec: 0 },
        type_,
        code,
        value,
    };
    unsafe {
        libc::write(
            fd,
            &ev as *const InputEvent as *const libc::c_void,
            std::mem::size_of::<InputEvent>(),
        );
    }
}

/// Build `char → (kernel scancode, mod mask)` by walking every keycode in
/// the user's active xkb keymap and asking what character each level
/// produces. Level 0 = base, level 1 = shift, level 2 = altgr, level 3 =
/// shift+altgr. We only keep the lowest-modifier mapping for each char.
/// Best-effort detection of the system's active xkb layout. Order:
///   1. $XKB_DEFAULT_LAYOUT env var (libxkbcommon reads this natively).
///   2. `localectl status` output (systemd, works on most modern distros).
///   3. Compile-time defaults (usually US QWERTY — wrong for Dvorak users).
fn detect_layout() -> (String, String, String) {
    if std::env::var("XKB_DEFAULT_LAYOUT").is_ok() {
        // libxkbcommon will pick up env vars when we pass empty strings.
        return (String::new(), String::new(), String::new());
    }

    if let Ok(out) = std::process::Command::new("localectl").arg("status").output() {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut layout = String::new();
            let mut variant = String::new();
            for line in text.lines() {
                let t = line.trim();
                if let Some(v) = t.strip_prefix("X11 Layout:") {
                    layout = v.trim().to_string();
                }
                if let Some(v) = t.strip_prefix("X11 Variant:") {
                    variant = v.trim().to_string();
                }
            }
            if !layout.is_empty() {
                return ("evdev".to_string(), layout, variant);
            }
        }
    }

    (String::new(), String::new(), String::new())
}

fn build_reverse_map() -> Result<HashMap<char, (u16, u8)>, String> {
    use xkbcommon::xkb::{self, Keycode};

    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let (rules, layout, variant) = detect_layout();
    eprintln!(
        "rhe: xkb layout: rules='{}' layout='{}' variant='{}'",
        if rules.is_empty() { "(default)" } else { &rules },
        if layout.is_empty() { "(default)" } else { &layout },
        if variant.is_empty() { "(default)" } else { &variant },
    );

    let model = String::new();
    let keymap = xkb::Keymap::new_from_names(
        &context,
        &rules,
        &model,
        &layout,
        &variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or("xkb_keymap_new_from_names returned null")?;

    let mut map: HashMap<char, (u16, u8)> = HashMap::new();

    // Level → modifier mask. Standard xkb convention.
    let level_mods = [0u8, MOD_SHIFT, MOD_ALTGR, MOD_SHIFT | MOD_ALTGR];

    let min_raw: u32 = keymap.min_keycode().into();
    let max_raw: u32 = keymap.max_keycode().into();

    for kc_raw in min_raw..=max_raw {
        let keycode: Keycode = kc_raw.into();
        let num_layouts = keymap.num_layouts_for_key(keycode);
        if num_layouts == 0 {
            continue;
        }
        let num_levels = keymap.num_levels_for_key(keycode, 0);
        for level in 0..num_levels.min(4) {
            let syms = keymap.key_get_syms_by_level(keycode, 0, level);
            for sym in syms {
                let codepoint = xkb::keysym_to_utf32(*sym);
                if codepoint == 0 {
                    continue;
                }
                let Some(c) = char::from_u32(codepoint) else {
                    continue;
                };
                // xkb keycode = linux scancode + 8.
                if kc_raw < 8 {
                    continue;
                }
                let scancode = (kc_raw - 8) as u16;
                let mods = level_mods[level as usize];
                // Keep the lowest-mod combination for each char (prefer base over shift).
                map.entry(c)
                    .and_modify(|existing| {
                        if mods < existing.1 {
                            *existing = (scancode, mods);
                        }
                    })
                    .or_insert((scancode, mods));
            }
        }
    }

    Ok(map)
}
