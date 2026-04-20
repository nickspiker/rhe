use core_graphics::event::{CGEvent, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use foreign_types::ForeignType;

pub struct MacOSOutput;

impl MacOSOutput {
    pub fn new() -> Self {
        Self
    }
}

impl super::TextOutput for MacOSOutput {
    fn emit(&self, text: &str) {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .expect("failed to create event source");

        let utf16: Vec<u16> = text.encode_utf16().collect();

        let event_down = CGEvent::new_keyboard_event(source.clone(), 0, true)
            .expect("failed to create key event");

        unsafe {
            CGEventKeyboardSetUnicodeString(
                event_down.as_ptr() as *mut _,
                utf16.len() as u64,
                utf16.as_ptr(),
            );
        }

        let event_up =
            CGEvent::new_keyboard_event(source, 0, false).expect("failed to create key event");

        // Post to Session level, not HID — avoids our own grab re-catching these
        event_down.post(CGEventTapLocation::Session);
        event_up.post(CGEventTapLocation::Session);
    }

    fn backspace(&self, count: usize) {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .expect("failed to create event source");

        for _ in 0..count {
            let down = CGEvent::new_keyboard_event(source.clone(), 51, true)
                .expect("failed to create key event");
            let up = CGEvent::new_keyboard_event(source.clone(), 51, false)
                .expect("failed to create key event");
            down.post(CGEventTapLocation::Session);
            up.post(CGEventTapLocation::Session);
        }
    }
}

unsafe extern "C" {
    fn CGEventKeyboardSetUnicodeString(
        event: *mut core_graphics::sys::CGEvent,
        length: u64,
        string: *const u16,
    );
}
