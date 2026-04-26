//! CGEventTap-based keyboard input for macOS.
//!
//! Selective key suppression — only home-row keys are grabbed, everything
//! else (media keys, F-keys, arrows, etc.) passes through natively.
//! No IOHIDManager seize needed. Key repeats detected via kCGKeyRepeat.
//!
//! Requires Accessibility permission (System Settings → Privacy → Accessibility)
//! or Input Monitoring.

use crate::hand::{KeyDirection, KeyEvent};
use crate::input::HidEvent;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

#[allow(non_upper_case_globals, dead_code)]
mod ffi {
    use std::ffi::c_void;

    pub type CGEventRef = *mut c_void;
    pub type CGEventTapProxy = *mut c_void;
    pub type CFMachPortRef = *mut c_void;
    pub type CFRunLoopSourceRef = *mut c_void;
    pub type CFRunLoopRef = *mut c_void;
    pub type CFAllocatorRef = *const c_void;
    pub type CFStringRef = *const c_void;
    pub type CGEventType = u32;
    pub type CGEventMask = u64;
    pub type CGEventField = u32;

    pub const kCFAllocatorDefault: CFAllocatorRef = std::ptr::null();

    // Event tap locations
    pub const kCGSessionEventTap: u32 = 1;
    pub const kCGHIDEventTap: u32 = 0;

    // Event tap placement
    pub const kCGHeadInsertEventTap: u32 = 0;

    // Event types
    pub const kCGEventKeyDown: CGEventType = 10;
    pub const kCGEventKeyUp: CGEventType = 11;
    pub const kCGEventFlagsChanged: CGEventType = 12;
    pub const kCGEventTapDisabledByTimeout: CGEventType = 0xFFFFFFFE;
    pub const kCGEventTapDisabledByUserInput: CGEventType = 0xFFFFFFFF;

    // Event fields
    pub const kCGKeyboardEventKeycode: CGEventField = 9;
    pub const kCGKeyboardEventAutorepeat: CGEventField = 8;

    // Event masks
    pub const kCGEventMaskForAllEvents: CGEventMask = !0;

    pub type CGEventTapCallBack = extern "C" fn(
        proxy: CGEventTapProxy,
        event_type: CGEventType,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef;

    unsafe extern "C" {
        pub fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: CGEventMask,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;

        pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

        pub fn CFMachPortCreateRunLoopSource(
            allocator: CFAllocatorRef,
            port: CFMachPortRef,
            order: i64,
        ) -> CFRunLoopSourceRef;

        pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        pub fn CFRunLoopAddSource(
            rl: CFRunLoopRef,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        pub fn CFRunLoopRun();

        pub static kCFRunLoopCommonModes: CFStringRef;

        pub fn CGEventGetIntegerValueField(
            event: CGEventRef,
            field: CGEventField,
        ) -> i64;

        pub fn CGEventGetFlags(event: CGEventRef) -> u64;
        pub fn CGEventSetFlags(event: CGEventRef, flags: u64);

        pub fn CGEventSetIntegerValueField(
            event: CGEventRef,
            field: CGEventField,
            value: i64,
        );
    }
}

/// macOS virtual keycode → rhe scan code (only for our keys).
/// Returns None for keys we don't care about (they pass through).
fn vk_to_scan(vk: u16) -> Option<u8> {
    use crate::scan;
    // Use the layout module to map — virtual keycodes are positional
    // (QWERTY-based regardless of OS layout).
    crate::preferences::layout::vk_to_role(vk)
}

/// Is this virtual keycode the caps lock key?
fn is_caps_lock(vk: u16) -> bool {
    vk == 0x39
}

/// Is this virtual keycode escape?
fn is_escape(vk: u16) -> bool {
    vk == 0x35
}

/// Is this virtual keycode the word key? (left command = 0x37, or whatever the layout says)
fn is_word_key(vk: u16) -> bool {
    vk_to_scan(vk).map_or(false, |s| s == crate::scan::WORD)
}

/// Is this a key rhe cares about?
fn is_rhe_key(vk: u16) -> bool {
    vk_to_scan(vk).is_some()
}

struct CallbackContext {
    tx: mpsc::Sender<HidEvent>,
    enabled: Arc<AtomicBool>,
    esc_quits: bool,
    /// IOHIDManager opened WITHOUT seize, just for caps lock LED control
    led_manager: *mut c_void,
    /// Proxy to wake the tray event loop on state changes
    tray_proxy: Option<crate::tray::TrayProxy>,
}

pub struct CgEventInput {
    pub rx: mpsc::Receiver<HidEvent>,
}

impl CgEventInput {
    pub fn start_grab(
        enabled: Arc<AtomicBool>,
        esc_quits: bool,
        tray_proxy: Option<crate::tray::TrayProxy>,
    ) -> Result<Self, String> {
        // Disable caps lock firmware debounce
        let _ = std::process::Command::new("hidutil")
            .args(["property", "--set", r#"{"CapsLockDelayOverride":0}"#])
            .output();

        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            unsafe {
                let ctx = Box::into_raw(Box::new(CallbackContext {
                    tx,
                    enabled,
                    esc_quits,
                    led_manager: std::ptr::null_mut(),
                    tray_proxy,
                }));

                let mask = (1u64 << ffi::kCGEventKeyDown)
                    | (1u64 << ffi::kCGEventKeyUp)
                    | (1u64 << ffi::kCGEventFlagsChanged);

                let tap = ffi::CGEventTapCreate(
                    ffi::kCGHIDEventTap,
                    ffi::kCGHeadInsertEventTap,
                    0, // active tap (can modify/suppress)
                    mask,
                    event_callback,
                    ctx as *mut c_void,
                );

                if tap.is_null() {
                    panic!(
                        "CGEventTapCreate failed — check Accessibility or Input Monitoring permissions"
                    );
                }

                let source = ffi::CFMachPortCreateRunLoopSource(
                    ffi::kCFAllocatorDefault,
                    tap,
                    0,
                );
                let rl = ffi::CFRunLoopGetCurrent();
                ffi::CFRunLoopAddSource(rl, source, ffi::kCFRunLoopCommonModes);
                ffi::CGEventTapEnable(tap, true);

                ffi::CFRunLoopRun();
            }
        });

        Ok(Self { rx })
    }
}

impl super::KeyInput for CgEventInput {
    fn next_event(&mut self) -> Option<KeyEvent> {
        loop {
            match self.rx.recv().ok()? {
                HidEvent::Key(ev) => return Some(ev),
                HidEvent::Quit => return None,
            }
        }
    }
}

extern "C" fn event_callback(
    _proxy: ffi::CGEventTapProxy,
    event_type: ffi::CGEventType,
    event: ffi::CGEventRef,
    user_info: *mut c_void,
) -> ffi::CGEventRef {
    unsafe {
        // Re-enable tap if macOS disabled it (timeout protection)
        if event_type == ffi::kCGEventTapDisabledByTimeout
            || event_type == ffi::kCGEventTapDisabledByUserInput
        {
            // Can't re-enable from here — just pass through
            return event;
        }

        let ctx = &*(user_info as *const CallbackContext);

        // Strip caps lock flag from EVERY event — caps lock is rhe's
        // private mode toggle, never the OS text-level caps lock.
        let flags = ffi::CGEventGetFlags(event);
        if flags & 0x10000 != 0 {
            ffi::CGEventSetFlags(event, flags & !0x10000);
        }

        let is_down = event_type == ffi::kCGEventKeyDown;
        let is_up = event_type == ffi::kCGEventKeyUp;
        let is_flags = event_type == ffi::kCGEventFlagsChanged;

        let vk = ffi::CGEventGetIntegerValueField(event, ffi::kCGKeyboardEventKeycode) as u16;

        // Caps lock toggle — one event per press, fully suppressed from OS
        if is_flags && is_caps_lock(vk) {
            let was_enabled = ctx.enabled.load(Ordering::Relaxed);
            let now_enabled = !was_enabled;
            ctx.enabled.store(now_enabled, Ordering::Relaxed);
            if let Some(proxy) = &ctx.tray_proxy {
                let _ = proxy.send_event(crate::tray::TrayEvent::StateChanged);
            }
            return std::ptr::null_mut(); // fully suppress — OS never sees caps lock
        }

        // Escape handling
        if is_down && is_escape(vk) {
            if ctx.esc_quits {
                let _ = ctx.tx.send(HidEvent::Quit);
                return std::ptr::null_mut();
            }
            // Not quitting — pass through
            return event;
        }

        // If rhe is disabled, pass everything through
        if !ctx.enabled.load(Ordering::Relaxed) {
            return event;
        }

        // Not a key down or up? Pass through (modifiers etc)
        if !is_down && !is_up {
            return event;
        }

        // Skip key repeats
        let is_repeat = ffi::CGEventGetIntegerValueField(
            event,
            ffi::kCGKeyboardEventAutorepeat,
        ) != 0;
        if is_repeat {
            return std::ptr::null_mut(); // suppress repeats for our keys
        }

        // Check if it's one of our keys
        if let Some(scan) = vk_to_scan(vk) {
            let direction = if is_down {
                KeyDirection::Down
            } else {
                KeyDirection::Up
            };
            let _ = ctx.tx.send(HidEvent::Key(KeyEvent { scan, direction }));
            // Suppress — rhe owns this key
            return std::ptr::null_mut();
        }

        // Not our key — pass through (media keys, arrows, numbers, etc.)
        event
    }
}
