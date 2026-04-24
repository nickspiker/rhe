//! Physical-key → rhe-role mapping, per layout.
//!
//! Six standard layouts cover the common hand-position variations:
//!
//! - **Narrow-R**: right-dominant, hands close together, right-thumb on
//!   space. The original rhe layout.
//! - **Medium-R**: right hand shifted one column right (index on K).
//!   Small stance widening with no edge-key exposure.
//! - **Wide-R**: right hand shifted two columns right so the pinky
//!   lands on Enter. Left thumb on space is the word key, right-Alt
//!   is the mod. Right-Shift synthesizes Enter so the user can still
//!   emit newlines.
//! - **Narrow-L** / **Medium-L** / **Wide-L**: mirror images for
//!   left-dominant typists. All rhe internals stay the same — only
//!   the physical scancodes swap sides.
//!
//! `CURRENT` picks which layout the binary uses. Change it here and
//! recompile; the file is both human-readable text and the single
//! source of truth the compiler sees, so there's no config drift and
//! no runtime dispatch cost (the match on `CURRENT` folds away).
//!
//! ## Rollover caveat (choosing a layout)
//!
//! Most keyboards — Mac, PC, laptop, membrane, or otherwise — only
//! guarantee full N-key rollover across the main alpha block (roughly
//! the 30 keys around home row). Keys outside that zone (`'`, Enter,
//! right-Shift, the numpad) typically share matrix rows with other
//! keys and drop to 2-key rollover under simultaneous press, causing
//! chord "ghosting" where the hardware silently suppresses keys rhe
//! needs. rhe chord use pushes up to six simultaneous keys on one
//! hand, so:
//!
//! - **Narrow-R / Narrow-L** live entirely inside the main alpha
//!   block and work on any keyboard. This is the safe default.
//! - **Medium-R / Medium-L** step one column into the edge zone
//!   (apostrophe) — usually fine, some budget boards will complain.
//! - **Wide-R / Wide-L** use Enter as a chord key and will ghost on
//!   most keyboards. Only viable on hardware with advertised full
//!   NKRO (gaming mechs, most aftermarket boards).
//!
//! When in doubt, stay narrow. Upgrade to medium/wide once you've
//! verified your keyboard's rollover across the target keys.

// The two platforms each consume one half of this module's surface;
// the other half is "dead" from that platform's perspective but the
// code still needs to compile and test. Silence the cross-platform
// warnings rather than cfg-gate every other definition.
#![allow(dead_code)]

use crate::scan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    NarrowR,
    MediumR,
    WideR,
    NarrowL,
    MediumL,
    WideL,
}

/// The layout this build will use. Edit this single line and recompile
/// to change rhe's key mapping. Narrow-R is the default — it stays
/// inside the main alpha block where almost every keyboard provides
/// full N-key rollover. See the rollover caveat in the module docs
/// before switching to Medium or Wide.
pub const CURRENT: Layout = Layout::NarrowR;

/// When enabled, the input backends auto-flip rhe on/off based on
/// the pattern of physical keys the user presses:
///
/// - **Enable** (off → on) when the user simultaneously holds 3+
///   home-row chord keys, OR 2+ chord keys alongside WORD or thumb.
/// - **Disable** (on → off) the moment a non-home-row *letter* key
///   goes down (Q-row or Z-row positions). Numbers, symbols,
///   punctuation, modifiers, and navigation keys don't trigger —
///   users need to type `$` or arrow around while inside rhe.
///
/// **Default: off.** Fast typists routinely hold 3+ home-row keys
/// briefly during rolled keystrokes, which triggers false auto-
/// enables. Caps-lock is the universally-reliable toggle; this
/// is strictly an opt-in convenience for users who find they
/// forget which mode rhe is in. Set to `true` here to enable.
///
/// Stuck-key handling on the transition is implemented either
/// way — flipping mid-press synthesizes the proper releases so
/// nothing gets latched.
pub const AUTO_SWITCH: bool = false;

/// True for Linux evdev scancodes that correspond to the 17
/// non-home-row letter positions (Q-W-E-R-T-Y-U-I-O-P and Z-X-C-V-
/// B-N-M in a QWERTY physical layout — the *positions* are what
/// matter, the actual letter the user's keymap produces is
/// irrelevant). Pressing one of these is a strong signal that the
/// user means to type letters, not chord.
pub const fn linux_is_non_home_row_letter(code: u16) -> bool {
    // Top letter row: 16..=25 inclusive (KEY_Q through KEY_P).
    // Bottom letter row: 44..=50 inclusive (KEY_Z through KEY_M).
    matches!(code, 16..=25 | 44..=50)
}

/// Mac equivalent: HID keyboard usage codes for the same 17
/// non-home-row letter positions.
pub const fn hid_is_non_home_row_letter(usage: u32) -> bool {
    // Top row Q-P (USB HID usage table).
    // Bottom row Z-M.
    matches!(
        usage,
        0x14 | 0x1A | 0x08 | 0x15 | 0x17 | 0x1C | 0x18 | 0x0C | 0x12 | 0x13
            | 0x1D | 0x1B | 0x06 | 0x19 | 0x05 | 0x11 | 0x10
    )
}

// ─── Linux evdev scancode constants ───
// A compile-time-only module so backends don't have to redeclare these.

pub mod linux {
    pub const KEY_ENTER: u16 = 28;
    pub const KEY_A: u16 = 30;
    pub const KEY_S: u16 = 31;
    pub const KEY_D: u16 = 32;
    pub const KEY_F: u16 = 33;
    pub const KEY_G: u16 = 34;
    pub const KEY_H: u16 = 35;
    pub const KEY_J: u16 = 36;
    pub const KEY_K: u16 = 37;
    pub const KEY_L: u16 = 38;
    pub const KEY_SEMICOLON: u16 = 39;
    pub const KEY_APOSTROPHE: u16 = 40;
    pub const KEY_LEFTSHIFT: u16 = 42;
    pub const KEY_RIGHTSHIFT: u16 = 54;
    pub const KEY_LEFTALT: u16 = 56;
    pub const KEY_SPACE: u16 = 57;
    pub const KEY_RIGHTALT: u16 = 100;
}

// ─── HID usage constants (macOS) ───

pub mod hid {
    pub const A: u32 = 0x04;
    pub const D: u32 = 0x07;
    pub const F: u32 = 0x09;
    pub const G: u32 = 0x0A;
    pub const H: u32 = 0x0B;
    pub const J: u32 = 0x0D;
    pub const K: u32 = 0x0E;
    pub const L: u32 = 0x0F;
    pub const S: u32 = 0x16;
    pub const RETURN: u32 = 0x28;
    pub const SPACEBAR: u32 = 0x2C;
    pub const SEMICOLON: u32 = 0x33;
    pub const APOSTROPHE: u32 = 0x34;
    pub const LEFT_SHIFT: u32 = 0xE1;
    pub const LEFT_ALT: u32 = 0xE2;
    pub const LEFT_GUI: u32 = 0xE3;
    pub const RIGHT_SHIFT: u32 = 0xE5;
    pub const RIGHT_ALT: u32 = 0xE6;
    pub const RIGHT_GUI: u32 = 0xE7;
}

// ─── Linux mappings ───

pub const fn linux_to_role(code: u16) -> Option<u8> {
    match CURRENT {
        Layout::NarrowR => linux_narrow_r(code),
        Layout::MediumR => linux_medium_r(code),
        Layout::WideR => linux_wide_r(code),
        Layout::NarrowL => linux_narrow_l(code),
        Layout::MediumL => linux_medium_l(code),
        Layout::WideL => linux_wide_l(code),
    }
}

const fn linux_narrow_r(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        KEY_A => Some(scan::L_PINKY),
        KEY_S => Some(scan::L_RING),
        KEY_D => Some(scan::L_MID),
        KEY_F => Some(scan::L_IDX),
        KEY_G => Some(scan::L_IDX_INNER),
        KEY_H => Some(scan::R_IDX_INNER),
        KEY_J => Some(scan::R_IDX),
        KEY_K => Some(scan::R_MID),
        KEY_L => Some(scan::R_RING),
        KEY_SEMICOLON => Some(scan::R_PINKY),
        KEY_SPACE => Some(scan::R_THUMB),
        KEY_LEFTALT => Some(scan::WORD),
        _ => None,
    }
}

const fn linux_medium_r(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        // Left hand unchanged.
        KEY_A => Some(scan::L_PINKY),
        KEY_S => Some(scan::L_RING),
        KEY_D => Some(scan::L_MID),
        KEY_F => Some(scan::L_IDX),
        KEY_G => Some(scan::L_IDX_INNER),
        // Right hand shifted one column right.
        KEY_J => Some(scan::R_IDX_INNER),
        KEY_K => Some(scan::R_IDX),
        KEY_L => Some(scan::R_MID),
        KEY_SEMICOLON => Some(scan::R_RING),
        KEY_APOSTROPHE => Some(scan::R_PINKY),
        // Same thumb roles as wide: space=word, right-alt=mod.
        KEY_SPACE => Some(scan::WORD),
        KEY_RIGHTALT => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn linux_wide_r(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        // Left hand unchanged.
        KEY_A => Some(scan::L_PINKY),
        KEY_S => Some(scan::L_RING),
        KEY_D => Some(scan::L_MID),
        KEY_F => Some(scan::L_IDX),
        KEY_G => Some(scan::L_IDX_INNER),
        // Right hand shifted two columns right.
        KEY_K => Some(scan::R_IDX_INNER),
        KEY_L => Some(scan::R_IDX),
        KEY_SEMICOLON => Some(scan::R_MID),
        KEY_APOSTROPHE => Some(scan::R_RING),
        KEY_ENTER => Some(scan::R_PINKY),
        // Thumbs flip roles: left thumb (space) is word, right-alt is mod.
        KEY_SPACE => Some(scan::WORD),
        KEY_RIGHTALT => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn linux_narrow_l(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        // rhe's "left hand" (vowels) on the physical right side.
        KEY_SEMICOLON => Some(scan::L_PINKY),
        KEY_L => Some(scan::L_RING),
        KEY_K => Some(scan::L_MID),
        KEY_J => Some(scan::L_IDX),
        KEY_H => Some(scan::L_IDX_INNER),
        // rhe's "right hand" (consonants) on the physical left side.
        KEY_G => Some(scan::R_IDX_INNER),
        KEY_F => Some(scan::R_IDX),
        KEY_D => Some(scan::R_MID),
        KEY_S => Some(scan::R_RING),
        KEY_A => Some(scan::R_PINKY),
        // Thumbs: right-alt is the non-dominant "word" key, space is mod.
        KEY_SPACE => Some(scan::R_THUMB),
        KEY_RIGHTALT => Some(scan::WORD),
        _ => None,
    }
}

const fn linux_medium_l(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        // "Left hand" (vowels) on physical right, shifted one col right.
        KEY_APOSTROPHE => Some(scan::L_PINKY),
        KEY_SEMICOLON => Some(scan::L_RING),
        KEY_L => Some(scan::L_MID),
        KEY_K => Some(scan::L_IDX),
        KEY_J => Some(scan::L_IDX_INNER),
        // "Right hand" (consonants) on physical left, unshifted.
        KEY_G => Some(scan::R_IDX_INNER),
        KEY_F => Some(scan::R_IDX),
        KEY_D => Some(scan::R_MID),
        KEY_S => Some(scan::R_RING),
        KEY_A => Some(scan::R_PINKY),
        KEY_SPACE => Some(scan::WORD),
        KEY_LEFTALT => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn linux_wide_l(code: u16) -> Option<u8> {
    use linux::*;
    match code {
        // "Left hand" (vowels) on physical right, shifted two cols right.
        KEY_ENTER => Some(scan::L_PINKY),
        KEY_APOSTROPHE => Some(scan::L_RING),
        KEY_SEMICOLON => Some(scan::L_MID),
        KEY_L => Some(scan::L_IDX),
        KEY_K => Some(scan::L_IDX_INNER),
        // "Right hand" (consonants) on physical left, unshifted.
        KEY_G => Some(scan::R_IDX_INNER),
        KEY_F => Some(scan::R_IDX),
        KEY_D => Some(scan::R_MID),
        KEY_S => Some(scan::R_RING),
        KEY_A => Some(scan::R_PINKY),
        // Right thumb (space) is word, left-alt is mod.
        KEY_SPACE => Some(scan::WORD),
        KEY_LEFTALT => Some(scan::R_THUMB),
        _ => None,
    }
}

/// For layouts that put a chord key on Enter, one of the shift keys
/// is remapped to synthesize a literal Enter keypress so the user can
/// still emit newlines. Returns the scancode to watch for, or `None`
/// if the current layout doesn't need the synth.
pub const fn linux_enter_synth_key() -> Option<u16> {
    match CURRENT {
        Layout::WideR => Some(linux::KEY_RIGHTSHIFT),
        Layout::WideL => Some(linux::KEY_LEFTSHIFT),
        Layout::NarrowR | Layout::NarrowL | Layout::MediumR | Layout::MediumL => None,
    }
}

// ─── HID mappings (macOS) ───

pub const fn hid_to_role(usage: u32) -> Option<u8> {
    match CURRENT {
        Layout::NarrowR => hid_narrow_r(usage),
        Layout::MediumR => hid_medium_r(usage),
        Layout::WideR => hid_wide_r(usage),
        Layout::NarrowL => hid_narrow_l(usage),
        Layout::MediumL => hid_medium_l(usage),
        Layout::WideL => hid_wide_l(usage),
    }
}

const fn hid_narrow_r(usage: u32) -> Option<u8> {
    match usage {
        hid::A => Some(scan::L_PINKY),
        hid::S => Some(scan::L_RING),
        hid::D => Some(scan::L_MID),
        hid::F => Some(scan::L_IDX),
        hid::G => Some(scan::L_IDX_INNER),
        hid::H => Some(scan::R_IDX_INNER),
        hid::J => Some(scan::R_IDX),
        hid::K => Some(scan::R_MID),
        hid::L => Some(scan::R_RING),
        hid::SEMICOLON => Some(scan::R_PINKY),
        hid::SPACEBAR => Some(scan::R_THUMB),
        hid::LEFT_GUI => Some(scan::WORD),
        _ => None,
    }
}

const fn hid_medium_r(usage: u32) -> Option<u8> {
    match usage {
        hid::A => Some(scan::L_PINKY),
        hid::S => Some(scan::L_RING),
        hid::D => Some(scan::L_MID),
        hid::F => Some(scan::L_IDX),
        hid::G => Some(scan::L_IDX_INNER),
        // Right hand shifted one column right
        hid::J => Some(scan::R_IDX_INNER),
        hid::K => Some(scan::R_IDX),
        hid::L => Some(scan::R_MID),
        hid::SEMICOLON => Some(scan::R_RING),
        hid::APOSTROPHE => Some(scan::R_PINKY),
        // Same thumb roles as wide: space=word, right-gui=mod
        hid::SPACEBAR => Some(scan::WORD),
        hid::RIGHT_GUI => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn hid_wide_r(usage: u32) -> Option<u8> {
    match usage {
        hid::A => Some(scan::L_PINKY),
        hid::S => Some(scan::L_RING),
        hid::D => Some(scan::L_MID),
        hid::F => Some(scan::L_IDX),
        hid::G => Some(scan::L_IDX_INNER),
        hid::K => Some(scan::R_IDX_INNER),
        hid::L => Some(scan::R_IDX),
        hid::SEMICOLON => Some(scan::R_MID),
        hid::APOSTROPHE => Some(scan::R_RING),
        hid::RETURN => Some(scan::R_PINKY),
        hid::SPACEBAR => Some(scan::WORD),
        hid::RIGHT_GUI => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn hid_narrow_l(usage: u32) -> Option<u8> {
    match usage {
        hid::SEMICOLON => Some(scan::L_PINKY),
        hid::L => Some(scan::L_RING),
        hid::K => Some(scan::L_MID),
        hid::J => Some(scan::L_IDX),
        hid::H => Some(scan::L_IDX_INNER),
        hid::G => Some(scan::R_IDX_INNER),
        hid::F => Some(scan::R_IDX),
        hid::D => Some(scan::R_MID),
        hid::S => Some(scan::R_RING),
        hid::A => Some(scan::R_PINKY),
        hid::SPACEBAR => Some(scan::R_THUMB),
        hid::RIGHT_GUI => Some(scan::WORD),
        _ => None,
    }
}

const fn hid_medium_l(usage: u32) -> Option<u8> {
    match usage {
        // "Left hand" (vowels) on physical right, shifted one col right
        hid::APOSTROPHE => Some(scan::L_PINKY),
        hid::SEMICOLON => Some(scan::L_RING),
        hid::L => Some(scan::L_MID),
        hid::K => Some(scan::L_IDX),
        hid::J => Some(scan::L_IDX_INNER),
        // "Right hand" (consonants) on physical left, unshifted
        hid::G => Some(scan::R_IDX_INNER),
        hid::F => Some(scan::R_IDX),
        hid::D => Some(scan::R_MID),
        hid::S => Some(scan::R_RING),
        hid::A => Some(scan::R_PINKY),
        hid::SPACEBAR => Some(scan::WORD),
        hid::LEFT_GUI => Some(scan::R_THUMB),
        _ => None,
    }
}

const fn hid_wide_l(usage: u32) -> Option<u8> {
    match usage {
        hid::RETURN => Some(scan::L_PINKY),
        hid::APOSTROPHE => Some(scan::L_RING),
        hid::SEMICOLON => Some(scan::L_MID),
        hid::L => Some(scan::L_IDX),
        hid::K => Some(scan::L_IDX_INNER),
        hid::G => Some(scan::R_IDX_INNER),
        hid::F => Some(scan::R_IDX),
        hid::D => Some(scan::R_MID),
        hid::S => Some(scan::R_RING),
        hid::A => Some(scan::R_PINKY),
        hid::SPACEBAR => Some(scan::WORD),
        hid::LEFT_GUI => Some(scan::R_THUMB),
        _ => None,
    }
}

pub const fn hid_enter_synth_usage() -> Option<u32> {
    match CURRENT {
        Layout::WideR => Some(hid::RIGHT_SHIFT),
        Layout::WideL => Some(hid::LEFT_SHIFT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each layout maps exactly one physical key to each of the 12
    // required roles (10 finger slots + word + mod). A missing or
    // duplicated role would silently kill part of the input surface,
    // so the test below is blunt: enumerate every key the layout
    // recognizes and demand the role set matches the canonical one.
    const REQUIRED_ROLES: &[u8] = &[
        scan::L_PINKY, scan::L_RING, scan::L_MID, scan::L_IDX, scan::L_IDX_INNER,
        scan::R_IDX_INNER, scan::R_IDX, scan::R_MID, scan::R_RING, scan::R_PINKY,
        scan::R_THUMB, scan::WORD,
    ];

    fn collect_roles<F, C>(mapper: F, codes: &[C]) -> Vec<u8>
    where
        F: Fn(C) -> Option<u8>,
        C: Copy,
    {
        let mut roles: Vec<u8> = codes.iter().filter_map(|&c| mapper(c)).collect();
        roles.sort_unstable();
        roles
    }

    fn expected_roles() -> Vec<u8> {
        let mut v: Vec<u8> = REQUIRED_ROLES.iter().copied().collect();
        v.sort_unstable();
        v
    }

    const ALL_LINUX_CODES: &[u16] = &[
        linux::KEY_ENTER, linux::KEY_A, linux::KEY_S, linux::KEY_D, linux::KEY_F,
        linux::KEY_G, linux::KEY_H, linux::KEY_J, linux::KEY_K, linux::KEY_L,
        linux::KEY_SEMICOLON, linux::KEY_APOSTROPHE, linux::KEY_LEFTSHIFT,
        linux::KEY_RIGHTSHIFT, linux::KEY_LEFTALT, linux::KEY_SPACE, linux::KEY_RIGHTALT,
    ];

    const ALL_HID_USAGES: &[u32] = &[
        hid::A, hid::S, hid::D, hid::F, hid::G, hid::H, hid::J, hid::K, hid::L,
        hid::SEMICOLON, hid::APOSTROPHE, hid::RETURN, hid::SPACEBAR,
        hid::LEFT_SHIFT, hid::RIGHT_SHIFT, hid::LEFT_ALT, hid::RIGHT_ALT,
        hid::LEFT_GUI, hid::RIGHT_GUI,
    ];

    #[test]
    fn linux_narrow_r_covers_all_roles() {
        assert_eq!(collect_roles(linux_narrow_r, ALL_LINUX_CODES), expected_roles());
    }

    #[test]
    fn linux_wide_r_covers_all_roles() {
        assert_eq!(collect_roles(linux_wide_r, ALL_LINUX_CODES), expected_roles());
    }

    #[test]
    fn linux_narrow_l_covers_all_roles() {
        assert_eq!(collect_roles(linux_narrow_l, ALL_LINUX_CODES), expected_roles());
    }

    #[test]
    fn linux_wide_l_covers_all_roles() {
        assert_eq!(collect_roles(linux_wide_l, ALL_LINUX_CODES), expected_roles());
    }

    #[test]
    fn hid_narrow_r_covers_all_roles() {
        assert_eq!(collect_roles(hid_narrow_r, ALL_HID_USAGES), expected_roles());
    }

    #[test]
    fn hid_wide_r_covers_all_roles() {
        assert_eq!(collect_roles(hid_wide_r, ALL_HID_USAGES), expected_roles());
    }

    #[test]
    fn hid_narrow_l_covers_all_roles() {
        assert_eq!(collect_roles(hid_narrow_l, ALL_HID_USAGES), expected_roles());
    }

    #[test]
    fn hid_wide_l_covers_all_roles() {
        assert_eq!(collect_roles(hid_wide_l, ALL_HID_USAGES), expected_roles());
    }

    #[test]
    fn non_home_row_letter_detection_linux() {
        // Top-row letter positions (Q-P) and bottom-row letter
        // positions (Z-M) should all be flagged.
        for code in 16..=25u16 {
            assert!(linux_is_non_home_row_letter(code), "code {} should trigger", code);
        }
        for code in 44..=50u16 {
            assert!(linux_is_non_home_row_letter(code), "code {} should trigger", code);
        }
        // Home-row positions must not trigger.
        for code in [linux::KEY_A, linux::KEY_S, linux::KEY_D, linux::KEY_F, linux::KEY_G,
                     linux::KEY_H, linux::KEY_J, linux::KEY_K, linux::KEY_L, linux::KEY_SEMICOLON] {
            assert!(!linux_is_non_home_row_letter(code), "home-row {} triggered", code);
        }
        // Numbers, modifiers, punctuation: shouldn't trigger.
        for code in [2u16, 3, 4, 10, 11, 12, 13, 26, 27, 28, 40, 41, 42, 54, 56, 57, 100] {
            assert!(!linux_is_non_home_row_letter(code), "non-letter {} triggered", code);
        }
    }

    #[test]
    fn non_home_row_letter_detection_hid() {
        // A few spot checks in HID usage space.
        assert!(hid_is_non_home_row_letter(0x14)); // Q
        assert!(hid_is_non_home_row_letter(0x13)); // P
        assert!(hid_is_non_home_row_letter(0x1D)); // Z
        assert!(hid_is_non_home_row_letter(0x10)); // M
        assert!(!hid_is_non_home_row_letter(hid::A));
        assert!(!hid_is_non_home_row_letter(hid::F));
        assert!(!hid_is_non_home_row_letter(hid::SPACEBAR));
        assert!(!hid_is_non_home_row_letter(hid::LEFT_ALT));
    }

    #[test]
    fn wide_layouts_have_enter_synth() {
        // If the active layout is wide, both backends advertise a
        // synth key. Narrow layouts leave it unset.
        if matches!(CURRENT, Layout::WideR | Layout::WideL) {
            assert!(linux_enter_synth_key().is_some());
            assert!(hid_enter_synth_usage().is_some());
        } else {
            assert!(linux_enter_synth_key().is_none());
            assert!(hid_enter_synth_usage().is_none());
        }
    }
}
