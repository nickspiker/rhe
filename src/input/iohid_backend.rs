//! IOHIDManager-based keyboard input for macOS.
//!
//! Seizes the keyboard at the HID level. One event per press, one per release.
//! No OS key repeat. No rdev.

use super::HidEvent;
use crate::hand::{KeyDirection, KeyEvent};
use crate::scan;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

// IOKit / CoreFoundation FFI — only what we need.
mod ffi {
    #![allow(non_upper_case_globals, non_camel_case_types, dead_code)]
    use std::ffi::c_void;

    pub type IOHIDManagerRef = *mut c_void;
    pub type IOHIDDeviceRef = *mut c_void;
    pub type IOHIDValueRef = *mut c_void;
    pub type IOHIDElementRef = *mut c_void;
    pub type CFRunLoopRef = *mut c_void;
    pub type CFStringRef = *const c_void;
    pub type CFDictionaryRef = *const c_void;
    pub type CFMutableDictionaryRef = *mut c_void;
    pub type CFNumberRef = *const c_void;
    pub type CFAllocatorRef = *const c_void;
    pub type CFIndex = isize;
    pub type IOReturn = i32;
    pub type IOOptionBits = u32;
    pub type CFNumberType = u32;

    pub const kCFAllocatorDefault: CFAllocatorRef = std::ptr::null();
    pub const kIOHIDOptionsTypeSeizeDevice: IOOptionBits = 0x01;
    pub const kCFNumberSInt32Type: CFNumberType = 3;

    // HID usage page & usage for keyboards
    pub const kHIDPage_GenericDesktop: u32 = 0x01;
    pub const kHIDUsage_GD_Keyboard: u32 = 0x06;

    // HID usage page for keyboard keys
    pub const kHIDPage_KeyboardOrKeypad: u32 = 0x07;

    // HID usage codes for our keys (USB HID spec)
    pub const kHIDUsage_KeyboardA: u32 = 0x04;
    pub const kHIDUsage_KeyboardS: u32 = 0x16;
    pub const kHIDUsage_KeyboardD: u32 = 0x07;
    pub const kHIDUsage_KeyboardF: u32 = 0x09;
    pub const kHIDUsage_KeyboardG: u32 = 0x0A;
    pub const kHIDUsage_KeyboardH: u32 = 0x0B;
    pub const kHIDUsage_KeyboardJ: u32 = 0x0D;
    pub const kHIDUsage_KeyboardK: u32 = 0x0E;
    pub const kHIDUsage_KeyboardL: u32 = 0x0F;
    pub const kHIDUsage_KeyboardSemicolon: u32 = 0x33;
    pub const kHIDUsage_KeyboardSpacebar: u32 = 0x2C;
    pub const kHIDUsage_KeyboardLeftGUI: u32 = 0xE3;
    pub const kHIDUsage_KeyboardEscape: u32 = 0x29;
    pub const kHIDUsage_KeyboardCapsLock: u32 = 0x39;

    // CGEvent for key passthrough
    pub type CGEventRef = *mut c_void;
    pub type CGEventSourceRef = *mut c_void;
    pub type CGEventFlags = u64;
    pub const kCGEventSourceStateHIDSystemState: i32 = 1;
    pub const kCGSessionEventTap: i32 = 1;

    // Modifier flags for CGEvent
    pub const kCGEventFlagMaskShift: CGEventFlags = 0x00020000;
    pub const kCGEventFlagMaskControl: CGEventFlags = 0x00040000;
    pub const kCGEventFlagMaskAlternate: CGEventFlags = 0x00080000;
    pub const kCGEventFlagMaskCommand: CGEventFlags = 0x00100000;

    pub type IOHIDValueCallback = extern "C" fn(
        context: *mut c_void,
        result: IOReturn,
        sender: IOHIDManagerRef,
        value: IOHIDValueRef,
    );

    unsafe extern "C" {
        // IOHIDManager
        pub fn IOHIDManagerCreate(
            allocator: CFAllocatorRef,
            options: IOOptionBits,
        ) -> IOHIDManagerRef;
        pub fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: CFDictionaryRef);
        pub fn IOHIDManagerRegisterInputValueCallback(
            manager: IOHIDManagerRef,
            callback: IOHIDValueCallback,
            context: *mut c_void,
        );
        pub fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: IOOptionBits) -> IOReturn;
        pub fn IOHIDManagerScheduleWithRunLoop(
            manager: IOHIDManagerRef,
            run_loop: CFRunLoopRef,
            mode: CFStringRef,
        );

        // IOHIDDevice — for LED control
        pub fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> *const c_void; // CFSetRef
        pub fn CFSetGetCount(set: *const c_void) -> CFIndex;
        pub fn CFSetGetValues(set: *const c_void, values: *mut *const c_void);
        pub fn IOHIDDeviceSetValue(
            device: IOHIDDeviceRef,
            element: IOHIDElementRef,
            value: IOHIDValueRef,
        ) -> IOReturn;
        pub fn IOHIDDeviceCopyMatchingElements(
            device: IOHIDDeviceRef,
            matching: CFDictionaryRef,
            options: IOOptionBits,
        ) -> *const c_void; // CFArrayRef
        pub fn CFArrayGetCount(array: *const c_void) -> CFIndex;
        pub fn CFArrayGetValueAtIndex(array: *const c_void, idx: CFIndex) -> *const c_void;
        pub fn IOHIDValueCreateWithIntegerValue(
            allocator: CFAllocatorRef,
            element: IOHIDElementRef,
            timestamp: u64,
            value: CFIndex,
        ) -> IOHIDValueRef;

        // HID element info
        pub fn IOHIDElementGetUsagePage(element: IOHIDElementRef) -> u32;
        pub fn IOHIDElementGetUsage(element: IOHIDElementRef) -> u32;

        // IOHIDValue
        pub fn IOHIDValueGetElement(value: IOHIDValueRef) -> IOHIDElementRef;
        pub fn IOHIDValueGetIntegerValue(value: IOHIDValueRef) -> CFIndex;

        // CoreFoundation
        pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        pub fn CFRunLoopRun();
        pub fn CFRunLoopStop(rl: CFRunLoopRef);

        pub static kCFRunLoopDefaultMode: CFStringRef;

        // Dictionary building
        pub fn CFDictionaryCreateMutable(
            allocator: CFAllocatorRef,
            capacity: CFIndex,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFMutableDictionaryRef;
        pub fn CFDictionarySetValue(
            dict: CFMutableDictionaryRef,
            key: *const c_void,
            value: *const c_void,
        );
        pub static kCFTypeDictionaryKeyCallBacks: c_void;
        pub static kCFTypeDictionaryValueCallBacks: c_void;

        pub fn CFNumberCreate(
            allocator: CFAllocatorRef,
            the_type: CFNumberType,
            value_ptr: *const c_void,
        ) -> CFNumberRef;

        pub fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            c_str: *const u8,
            encoding: u32,
        ) -> CFStringRef;

        // CGEvent — for re-injecting non-rhe keys to OS
        pub fn CGEventSourceCreate(state_id: i32) -> CGEventSourceRef;
        pub fn CGEventCreateKeyboardEvent(
            source: CGEventSourceRef,
            virtual_key: u16,
            key_down: bool,
        ) -> CGEventRef;
        pub fn CGEventPost(tap: i32, event: CGEventRef);
        pub fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
        pub fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
        pub fn CFRelease(cf: *const c_void);
    }

    pub const kCFStringEncodingUTF8: u32 = 0x08000100;
}

/// Auto-switch state: HID usages of currently-held chord keys. We
/// track the raw usages (not just tallies) so that on a switch we
/// can release keys from whichever side of the pipeline didn't see
/// their down — preventing stuck keys when rhe grabs or releases
/// the keyboard mid-press.
#[derive(Default)]
struct AutoSwitchState {
    held: Vec<u32>,
}

/// Context passed to the HID callback.
struct CallbackContext {
    tx: mpsc::Sender<HidEvent>,
    enabled: Arc<AtomicBool>, // shared with tray — caps lock toggles this
    manager: ffi::IOHIDManagerRef,
    /// Tracked modifier state for passthrough re-injection
    modifier_flags: std::sync::Mutex<u64>,
    /// If true, Escape sends Quit. If false, Escape passes through.
    esc_quits: bool,
    /// Chord-key tallies for the auto-switch heuristic.
    auto_switch: std::sync::Mutex<AutoSwitchState>,
}

pub struct IoHidInput {
    pub rx: mpsc::Receiver<HidEvent>,
    manager: ffi::IOHIDManagerRef,
}

// Safety: the IOHIDManagerRef is thread-safe (CFRunLoop-based).
unsafe impl Send for IoHidInput {}

impl Drop for IoHidInput {
    fn drop(&mut self) {
        // Reset caps lock LED to off on shutdown
        unsafe {
            set_caps_lock_led(self.manager, false);
        }
    }
}

impl IoHidInput {
    pub fn start_grab(enabled: Arc<AtomicBool>, esc_quits: bool) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        // Wrapper to send raw pointer across thread boundary
        struct SendPtr(*mut std::ffi::c_void);
        unsafe impl Send for SendPtr {}
        let (mgr_tx, mgr_rx) = mpsc::channel::<SendPtr>();

        std::thread::spawn(move || {
            unsafe {
                let manager = ffi::IOHIDManagerCreate(ffi::kCFAllocatorDefault, 0);
                if manager.is_null() {
                    panic!("IOHIDManagerCreate failed");
                }
                // Send manager ref back so Drop can reset the LED
                let _ = mgr_tx.send(SendPtr(manager));

                // Match keyboard devices
                let matching = create_keyboard_matching();
                ffi::IOHIDManagerSetDeviceMatching(manager, matching as ffi::CFDictionaryRef);

                // Register callback
                let enabled_for_led = enabled.clone();
                let ctx = Box::into_raw(Box::new(CallbackContext {
                    tx, enabled, manager,
                    modifier_flags: std::sync::Mutex::new(0),
                    esc_quits,
                    auto_switch: std::sync::Mutex::new(AutoSwitchState::default()),
                }));
                ffi::IOHIDManagerRegisterInputValueCallback(
                    manager,
                    hid_callback,
                    ctx as *mut std::ffi::c_void,
                );

                // Schedule with run loop
                let rl = ffi::CFRunLoopGetCurrent();
                ffi::IOHIDManagerScheduleWithRunLoop(manager, rl, ffi::kCFRunLoopDefaultMode);

                // Open with seize — grabs the keyboard exclusively
                let result = ffi::IOHIDManagerOpen(manager, ffi::kIOHIDOptionsTypeSeizeDevice);
                if result != 0 {
                    panic!(
                        "IOHIDManagerOpen failed: {} — check Input Monitoring permissions",
                        result
                    );
                }

                // Set caps lock LED to match initial state:
                // rhe active (enabled=true) → LED OFF
                // keyboard mode (enabled=false) → LED ON
                let initial_enabled = enabled_for_led.load(Ordering::Relaxed);
                set_caps_lock_led(manager, !initial_enabled);

                // Run forever
                ffi::CFRunLoopRun();
            }
        });

        let SendPtr(manager) = mgr_rx.recv().map_err(|_| "failed to get manager ref".to_string())?;
        Ok(Self { rx, manager })
    }
}

impl super::KeyInput for IoHidInput {
    fn next_event(&mut self) -> Option<KeyEvent> {
        loop {
            match self.rx.recv().ok()? {
                HidEvent::Key(ev) => return Some(ev),
                HidEvent::Quit => return None,
            }
        }
    }
}

/// The HID callback — fires once per physical key state change. No repeats.
extern "C" fn hid_callback(
    context: *mut std::ffi::c_void,
    _result: ffi::IOReturn,
    _sender: ffi::IOHIDManagerRef,
    value: ffi::IOHIDValueRef,
) {
    unsafe {
        let ctx = &*(context as *const CallbackContext);

        let element = ffi::IOHIDValueGetElement(value);
        let usage_page = ffi::IOHIDElementGetUsagePage(element);
        let usage = ffi::IOHIDElementGetUsage(element);
        let pressed = ffi::IOHIDValueGetIntegerValue(value);

        // Non-keyboard usage pages (consumer control, etc.) — let them through
        if usage_page != ffi::kHIDPage_KeyboardOrKeypad {
            return; // IOHIDManager doesn't suppress non-keyboard elements
        }
        // Skip rollover/sentinel elements within the keyboard page
        if usage <= 0x03 || usage == 0xffffffff {
            return;
        }

        let direction = if pressed != 0 {
            KeyDirection::Down
        } else {
            KeyDirection::Up
        };

        // Caps lock: only toggle on key-down, ignore key-up
        if usage == ffi::kHIDUsage_KeyboardCapsLock {
            if pressed != 0 {
                let was_enabled = ctx.enabled.load(Ordering::Relaxed);
                let now_enabled = !was_enabled;
                ctx.enabled.store(now_enabled, Ordering::Relaxed);
                set_caps_lock_led(ctx.manager, !now_enabled);
            }
            return;
        }

        // Escape handling
        if usage == ffi::kHIDUsage_KeyboardEscape && pressed != 0 {
            if ctx.esc_quits {
                let _ = ctx.tx.send(HidEvent::Quit);
                return;
            }
            // Not quitting — pass escape through to OS
            if let Some(vk) = hid_usage_to_virtual_keycode(usage) {
                reinject_key(vk, usage, true, &ctx.modifier_flags);
            }
            return;
        }

        // Wide layouts put a chord key on Return, so one shift key
        // gets remapped to synthesize Return (otherwise the user
        // loses newline). Runs before everything else — it's just a
        // passthrough with a rewritten code, not a rhe event.
        if let Some(synth_usage) = crate::layout::hid_enter_synth_usage() {
            if usage == synth_usage {
                // HID 0x28 = Return, virtual keycode 0x24.
                reinject_key(0x24, 0x28, pressed != 0, &ctx.modifier_flags);
                return;
            }
        }

        // Auto-switch: flip `enabled` based on chord-key patterns so
        // rhe self-corrects when the user forgot to toggle it. See
        // `crate::layout::AUTO_SWITCH` for the rule. On a switch,
        // release the held keys from whichever side didn't see their
        // down so they don't stay stuck after the ownership flip.
        let mut auto_switched = false;
        if crate::layout::AUTO_SWITCH {
            let rhe_role = hid_usage_to_scan(usage);
            let mut sw = ctx.auto_switch.lock().unwrap();
            if rhe_role.is_some() {
                if pressed != 0 {
                    if !sw.held.contains(&usage) {
                        sw.held.push(usage);
                    }
                } else {
                    sw.held.retain(|&u| u != usage);
                }
            }

            if pressed != 0 {
                let (finger, word, thumb) = classify_hid_held(&sw.held);

                // Auto-enable.
                if !ctx.enabled.load(Ordering::Relaxed)
                    && (finger >= 3 || (finger >= 2 && (word || thumb)))
                {
                    ctx.enabled.store(true, Ordering::Relaxed);
                    set_caps_lock_led(ctx.manager, false);
                    // Release held keys from OS's view (they were
                    // passing through while rhe was off).
                    for &u in &sw.held {
                        if let Some(vk) = hid_usage_to_virtual_keycode(u) {
                            reinject_key(vk, u, false, &ctx.modifier_flags);
                        }
                    }
                    // Bring rhe's state machine up to date.
                    for &u in &sw.held {
                        if let Some(scan) = hid_usage_to_scan(u) {
                            let _ = ctx.tx.send(HidEvent::Key(KeyEvent {
                                scan,
                                direction: KeyDirection::Down,
                            }));
                        }
                    }
                    auto_switched = true;
                }

                // Auto-disable.
                if !auto_switched
                    && rhe_role.is_none()
                    && crate::layout::hid_is_non_home_row_letter(usage)
                    && ctx.enabled.load(Ordering::Relaxed)
                {
                    ctx.enabled.store(false, Ordering::Relaxed);
                    set_caps_lock_led(ctx.manager, true);
                    // Tell rhe to release what it was tracking.
                    for &u in &sw.held {
                        if let Some(scan) = hid_usage_to_scan(u) {
                            let _ = ctx.tx.send(HidEvent::Key(KeyEvent {
                                scan,
                                direction: KeyDirection::Up,
                            }));
                        }
                    }
                    // Inject matching key-downs to OS.
                    for &u in &sw.held {
                        if let Some(vk) = hid_usage_to_virtual_keycode(u) {
                            reinject_key(vk, u, true, &ctx.modifier_flags);
                        }
                    }
                }
            }
            drop(sw);
        }

        // After auto-enable the triggering event is already queued
        // to rhe as part of the held-set replay; skip the rest of
        // this callback to avoid double-sending it.
        if auto_switched {
            return;
        }

        // If rhe is disabled (caps lock toggled off), pass everything through
        if !ctx.enabled.load(Ordering::Relaxed) {
            if let Some(vk) = hid_usage_to_virtual_keycode(usage) {
                reinject_key(vk, usage, pressed != 0, &ctx.modifier_flags);
            }
            return;
        }

        if let Some(scan) = hid_usage_to_scan(usage) {
            let _ = ctx.tx.send(HidEvent::Key(KeyEvent { scan, direction }));
        } else {
            // Not our key — re-inject to OS so other apps see it
            if let Some(vk) = hid_usage_to_virtual_keycode(usage) {
                reinject_key(vk, usage, pressed != 0, &ctx.modifier_flags);
            }
        }
    }
}

/// Map HID usage to macOS virtual keycode for passthrough re-injection.
/// Re-inject a key event to the OS with proper modifier flags.
/// Tracks modifier state so all events carry the correct flags.
unsafe fn reinject_key(vk: u16, usage: u32, key_down: bool, modifier_flags: &std::sync::Mutex<u64>) {
    let source = ffi::CGEventSourceCreate(ffi::kCGEventSourceStateHIDSystemState);
    if source.is_null() { return; }

    // Update tracked modifier state
    let modifier_flag = match usage {
        0xE0 | 0xE4 => Some(ffi::kCGEventFlagMaskControl),
        0xE1 | 0xE5 => Some(ffi::kCGEventFlagMaskShift),
        0xE2 | 0xE6 => Some(ffi::kCGEventFlagMaskAlternate),
        0xE3 | 0xE7 => Some(ffi::kCGEventFlagMaskCommand),
        _ => None,
    };

    if let Some(flag) = modifier_flag {
        let mut flags = modifier_flags.lock().unwrap();
        if key_down {
            *flags |= flag;
        } else {
            *flags &= !flag;
        }
    }

    let event = ffi::CGEventCreateKeyboardEvent(source, vk, key_down);
    if !event.is_null() {
        // Apply current modifier state to ALL events
        let flags = *modifier_flags.lock().unwrap();
        ffi::CGEventSetFlags(event, flags);

        ffi::CGEventPost(ffi::kCGSessionEventTap, event);
        ffi::CFRelease(event as *const std::ffi::c_void);
    }
    ffi::CFRelease(source as *const std::ffi::c_void);
}

/// Toggle the caps lock LED on all keyboard devices.
/// LED page = 0x08, usage = 0x02 (Caps Lock).
unsafe fn set_caps_lock_led(manager: ffi::IOHIDManagerRef, on: bool) {
    let devices = ffi::IOHIDManagerCopyDevices(manager);
    if devices.is_null() { return; }

    let count = ffi::CFSetGetCount(devices);
    if count == 0 {
        ffi::CFRelease(devices);
        return;
    }

    let mut device_ptrs: Vec<*const std::ffi::c_void> = vec![std::ptr::null(); count as usize];
    ffi::CFSetGetValues(devices, device_ptrs.as_mut_ptr());

    for &dev_ptr in &device_ptrs {
        let device = dev_ptr as ffi::IOHIDDeviceRef;
        // Find LED elements on this device
        let elements = ffi::IOHIDDeviceCopyMatchingElements(
            device, std::ptr::null(), 0
        );
        if elements.is_null() { continue; }

        let el_count = ffi::CFArrayGetCount(elements);
        for i in 0..el_count {
            let element = ffi::CFArrayGetValueAtIndex(elements, i) as ffi::IOHIDElementRef;
            let page = ffi::IOHIDElementGetUsagePage(element);
            let el_usage = ffi::IOHIDElementGetUsage(element);

            // LED page = 0x08, Caps Lock LED = usage 0x02
            if page == 0x08 && el_usage == 0x02 {
                let value = ffi::IOHIDValueCreateWithIntegerValue(
                    ffi::kCFAllocatorDefault,
                    element,
                    0,
                    if on { 1 } else { 0 },
                );
                if !value.is_null() {
                    ffi::IOHIDDeviceSetValue(device, element, value);
                    ffi::CFRelease(value as *const std::ffi::c_void);
                }
            }
        }
        ffi::CFRelease(elements);
    }
    ffi::CFRelease(devices);
}

fn hid_usage_to_virtual_keycode(usage: u32) -> Option<u16> {
    // Common keys — HID usage page 0x07 → macOS virtual keycodes
    match usage {
        0x04 => Some(0x00), // A
        0x05 => Some(0x0B), // B
        0x06 => Some(0x08), // C
        0x07 => Some(0x02), // D
        0x08 => Some(0x0E), // E
        0x09 => Some(0x03), // F
        0x0A => Some(0x05), // G
        0x0B => Some(0x04), // H
        0x0C => Some(0x22), // I
        0x0D => Some(0x26), // J
        0x0E => Some(0x28), // K
        0x0F => Some(0x25), // L
        0x10 => Some(0x2E), // M
        0x11 => Some(0x2D), // N
        0x12 => Some(0x1F), // O
        0x13 => Some(0x23), // P
        0x14 => Some(0x0C), // Q
        0x15 => Some(0x0F), // R
        0x16 => Some(0x01), // S
        0x17 => Some(0x11), // T
        0x18 => Some(0x20), // U
        0x19 => Some(0x09), // V
        0x1A => Some(0x0D), // W
        0x1B => Some(0x07), // X
        0x1C => Some(0x10), // Y
        0x1D => Some(0x06), // Z
        0x1E => Some(0x12), // 1
        0x1F => Some(0x13), // 2
        0x20 => Some(0x14), // 3
        0x21 => Some(0x15), // 4
        0x22 => Some(0x17), // 5
        0x23 => Some(0x16), // 6
        0x24 => Some(0x1A), // 7
        0x25 => Some(0x1C), // 8
        0x26 => Some(0x19), // 9
        0x27 => Some(0x1D), // 0
        0x28 => Some(0x24), // Return
        0x29 => Some(0x35), // Escape
        0x2A => Some(0x33), // Backspace
        0x2B => Some(0x30), // Tab
        0x2C => Some(0x31), // Space
        0x2D => Some(0x1B), // -
        0x2E => Some(0x18), // =
        0x2F => Some(0x21), // [
        0x30 => Some(0x1E), // ]
        0x31 => Some(0x2A), // backslash
        0x33 => Some(0x29), // ;
        0x34 => Some(0x27), // '
        0x35 => Some(0x32), // `
        0x36 => Some(0x2B), // ,
        0x37 => Some(0x2F), // .
        0x38 => Some(0x2C), // /
        0x39 => Some(0x39), // Caps Lock
        0x3A => Some(0x7A), // F1
        0x3B => Some(0x78), // F2
        0x3C => Some(0x63), // F3
        0x3D => Some(0x76), // F4
        0x3E => Some(0x60), // F5
        0x3F => Some(0x61), // F6
        0x40 => Some(0x62), // F7
        0x41 => Some(0x64), // F8
        0x42 => Some(0x65), // F9
        0x43 => Some(0x6D), // F10
        0x44 => Some(0x67), // F11
        0x45 => Some(0x6F), // F12
        0x4F => Some(0x7C), // Right Arrow
        0x50 => Some(0x7B), // Left Arrow
        0x51 => Some(0x7D), // Down Arrow
        0x52 => Some(0x7E), // Up Arrow
        0xE0 => Some(0x3B), // Left Control
        0xE1 => Some(0x38), // Left Shift
        0xE2 => Some(0x3A), // Left Alt/Option
        0xE3 => Some(0x37), // Left GUI/Command
        0xE4 => Some(0x3E), // Right Control
        0xE5 => Some(0x3C), // Right Shift
        0xE6 => Some(0x3D), // Right Alt/Option
        0xE7 => Some(0x36), // Right GUI/Command
        _ => None,
    }
}

/// Map HID usage codes to rhe canonical scancodes. The per-layout
/// table lives in `crate::layout`; this is just a thin wrapper so the
/// callers can stay in HID-land.
/// Classify a set of held HID usages into the three tally buckets
/// the auto-switch heuristic cares about. Mirrors `classify_held`
/// on the Linux side.
fn classify_hid_held(held: &[u32]) -> (u8, bool, bool) {
    let mut finger: u8 = 0;
    let mut word = false;
    let mut thumb = false;
    for &u in held {
        if let Some(role) = crate::layout::hid_to_role(u) {
            if role == crate::scan::R_THUMB {
                thumb = true;
            } else if role == crate::scan::WORD {
                word = true;
            } else {
                finger = finger.saturating_add(1);
            }
        }
    }
    (finger, word, thumb)
}

fn hid_usage_to_scan(usage: u32) -> Option<u8> {
    crate::layout::hid_to_role(usage)
}

/// Build a matching dictionary for keyboard devices.
unsafe fn create_keyboard_matching() -> ffi::CFMutableDictionaryRef {
    unsafe {
        let dict = ffi::CFDictionaryCreateMutable(
            ffi::kCFAllocatorDefault,
            2,
            &ffi::kCFTypeDictionaryKeyCallBacks as *const _ as *const std::ffi::c_void,
            &ffi::kCFTypeDictionaryValueCallBacks as *const _ as *const std::ffi::c_void,
        );

        let page: i32 = ffi::kHIDPage_GenericDesktop as i32;
        let usage: i32 = ffi::kHIDUsage_GD_Keyboard as i32;

        let page_num = ffi::CFNumberCreate(
            ffi::kCFAllocatorDefault,
            ffi::kCFNumberSInt32Type,
            &page as *const _ as *const std::ffi::c_void,
        );
        let usage_num = ffi::CFNumberCreate(
            ffi::kCFAllocatorDefault,
            ffi::kCFNumberSInt32Type,
            &usage as *const _ as *const std::ffi::c_void,
        );

        let usage_page_key = ffi::CFStringCreateWithCString(
            ffi::kCFAllocatorDefault,
            b"DeviceUsagePage\0".as_ptr(),
            ffi::kCFStringEncodingUTF8,
        );
        let usage_key = ffi::CFStringCreateWithCString(
            ffi::kCFAllocatorDefault,
            b"DeviceUsage\0".as_ptr(),
            ffi::kCFStringEncodingUTF8,
        );

        ffi::CFDictionarySetValue(
            dict,
            usage_page_key as *const std::ffi::c_void,
            page_num as *const std::ffi::c_void,
        );
        ffi::CFDictionarySetValue(
            dict,
            usage_key as *const std::ffi::c_void,
            usage_num as *const std::ffi::c_void,
        );

        dict
    }
}
