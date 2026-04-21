//! IOHIDManager-based keyboard input for macOS.
//!
//! Seizes the keyboard at the HID level. One event per press, one per release.
//! No OS key repeat. No rdev.

use super::HidEvent;
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent, PhysicalKey};
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
    pub const kHIDUsage_KeyboardJ: u32 = 0x0D;
    pub const kHIDUsage_KeyboardK: u32 = 0x0E;
    pub const kHIDUsage_KeyboardL: u32 = 0x0F;
    pub const kHIDUsage_KeyboardSemicolon: u32 = 0x33;
    pub const kHIDUsage_KeyboardSpacebar: u32 = 0x2C;
    pub const kHIDUsage_KeyboardLeftGUI: u32 = 0xE3;
    pub const kHIDUsage_KeyboardEscape: u32 = 0x29;

    // CGEvent for key passthrough
    pub type CGEventRef = *mut c_void;
    pub type CGEventSourceRef = *mut c_void;
    pub const kCGEventSourceStateHIDSystemState: i32 = 1;
    pub const kCGSessionEventTap: i32 = 1;

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

        // IOHIDValue
        pub fn IOHIDValueGetElement(value: IOHIDValueRef) -> IOHIDElementRef;
        pub fn IOHIDValueGetIntegerValue(value: IOHIDValueRef) -> CFIndex;

        // IOHIDElement
        pub fn IOHIDElementGetUsagePage(element: IOHIDElementRef) -> u32;
        pub fn IOHIDElementGetUsage(element: IOHIDElementRef) -> u32;

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
        pub fn CFRelease(cf: *const c_void);
    }

    pub const kCFStringEncodingUTF8: u32 = 0x08000100;
}

/// Context passed to the HID callback.
struct CallbackContext {
    tx: mpsc::Sender<HidEvent>,
    enabled: Arc<AtomicBool>,
}

pub struct IoHidInput {
    pub rx: mpsc::Receiver<HidEvent>,
}

impl IoHidInput {
    pub fn start_grab(enabled: Arc<AtomicBool>) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            unsafe {
                let manager = ffi::IOHIDManagerCreate(ffi::kCFAllocatorDefault, 0);
                if manager.is_null() {
                    panic!("IOHIDManagerCreate failed");
                }

                // Match keyboard devices
                let matching = create_keyboard_matching();
                ffi::IOHIDManagerSetDeviceMatching(manager, matching as ffi::CFDictionaryRef);

                // Register callback
                let ctx = Box::into_raw(Box::new(CallbackContext { tx, enabled }));
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

                // Run forever
                ffi::CFRunLoopRun();
            }
        });

        Ok(Self { rx })
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

        if !ctx.enabled.load(Ordering::Relaxed) {
            return;
        }

        // Only real keyboard keys (skip rollover/sentinel elements)
        if usage_page != ffi::kHIDPage_KeyboardOrKeypad || usage <= 0x03 || usage == 0xffffffff {
            return;
        }

        let direction = if pressed != 0 {
            KeyDirection::Down
        } else {
            KeyDirection::Up
        };

        // Escape → quit
        if usage == ffi::kHIDUsage_KeyboardEscape && pressed != 0 {
            let _ = ctx.tx.send(HidEvent::Quit);
            return;
        }

        if let Some(physical) = hid_usage_to_physical(usage) {
            let _ = ctx.tx.send(HidEvent::Key(KeyEvent {
                key: physical,
                direction,
            }));
        } else {
            // Not our key — re-inject to OS so other apps see it
            if let Some(vk) = hid_usage_to_virtual_keycode(usage) {
                let source =
                    ffi::CGEventSourceCreate(ffi::kCGEventSourceStateHIDSystemState);
                if !source.is_null() {
                    let event = ffi::CGEventCreateKeyboardEvent(
                        source,
                        vk,
                        pressed != 0,
                    );
                    if !event.is_null() {
                        ffi::CGEventPost(ffi::kCGSessionEventTap, event);
                        ffi::CFRelease(event as *const std::ffi::c_void);
                    }
                    ffi::CFRelease(source as *const std::ffi::c_void);
                }
            }
        }
    }
}

/// Map HID usage to macOS virtual keycode for passthrough re-injection.
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

/// Map HID usage codes to our physical keys.
fn hid_usage_to_physical(usage: u32) -> Option<PhysicalKey> {
    match usage {
        // Left hand home row (QWERTY A S D F)
        ffi::kHIDUsage_KeyboardA => Some(PhysicalKey::Finger(Hand::Left, Finger::Pinky)),
        ffi::kHIDUsage_KeyboardS => Some(PhysicalKey::Finger(Hand::Left, Finger::Ring)),
        ffi::kHIDUsage_KeyboardD => Some(PhysicalKey::Finger(Hand::Left, Finger::Middle)),
        ffi::kHIDUsage_KeyboardF => Some(PhysicalKey::Finger(Hand::Left, Finger::Index)),

        // Right hand home row (QWERTY J K L ;) + spacebar as 5th bit
        ffi::kHIDUsage_KeyboardJ => Some(PhysicalKey::Finger(Hand::Right, Finger::Index)),
        ffi::kHIDUsage_KeyboardK => Some(PhysicalKey::Finger(Hand::Right, Finger::Middle)),
        ffi::kHIDUsage_KeyboardL => Some(PhysicalKey::Finger(Hand::Right, Finger::Ring)),
        ffi::kHIDUsage_KeyboardSemicolon => Some(PhysicalKey::Finger(Hand::Right, Finger::Pinky)),
        ffi::kHIDUsage_KeyboardSpacebar => Some(PhysicalKey::Finger(Hand::Right, Finger::Thumb)),

        // Word boundary (left ⌘)
        ffi::kHIDUsage_KeyboardLeftGUI => Some(PhysicalKey::Word),

        _ => None,
    }
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
