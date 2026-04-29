//! Windows text output via SendInput with KEYEVENTF_UNICODE.
//!
//! Sends Unicode codepoints directly so the OS handles whatever keyboard
//! layout is active — no per-layout reverse map needed. Surrogate pairs
//! (codepoints above U+FFFF) get split into two UTF-16 code units, each
//! sent as a separate keystroke pair. Backspace uses VK_BACK.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
    VIRTUAL_KEY, VK_BACK,
};

pub struct WindowsOutput;

impl WindowsOutput {
    pub fn new() -> Self {
        Self
    }
}

impl super::TextOutput for WindowsOutput {
    fn emit(&self, text: &str) {
        let mut units: Vec<u16> = Vec::with_capacity(text.len());
        units.extend(text.encode_utf16());
        // Each code unit becomes a down/up pair. Surrogates are
        // sent as two separate units; Windows reassembles them
        // into the original codepoint at the destination app.
        let mut inputs: Vec<INPUT> = Vec::with_capacity(units.len() * 2);
        for &u in &units {
            inputs.push(unicode_input(u, false));
            inputs.push(unicode_input(u, true));
        }
        if inputs.is_empty() {
            return;
        }
        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn backspace(&self, count: usize) {
        if count == 0 {
            return;
        }
        let mut inputs: Vec<INPUT> = Vec::with_capacity(count * 2);
        for _ in 0..count {
            inputs.push(vk_input(VK_BACK, false));
            inputs.push(vk_input(VK_BACK, true));
        }
        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

fn unicode_input(code_unit: u16, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_UNICODE
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: code_unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        Default::default()
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
