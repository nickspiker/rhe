//! Canonical scancode constants for the chord model.
//!
//! rhe's internal chord representation (`KeyMask`) indexes bits by
//! scancode. These constants nail down *which* scancodes the home-row
//! chord keys occupy — Linux evdev codes, but the numbers themselves
//! are arbitrary: they just have to be internally consistent between
//! the input backends and the chord/lookup tables.
//!
//! The macOS backend translates HID usage codes into these values
//! before handing events to the state machine.

use crate::key_mask::KeyMask;

// Left hand home row
pub const L_PINKY: u8 = 30; // KEY_A
pub const L_RING: u8 = 31; // KEY_S
pub const L_MID: u8 = 32; // KEY_D
pub const L_IDX: u8 = 33; // KEY_F

// Inner-index keys (stretch positions between the hands).
// Reserved for future digit/number mode. Not in any chord yet.
pub const L_IDX_INNER: u8 = 34; // KEY_G
pub const R_IDX_INNER: u8 = 35; // KEY_H

// Right hand home row
pub const R_IDX: u8 = 36; // KEY_J
pub const R_MID: u8 = 37; // KEY_K
pub const R_RING: u8 = 38; // KEY_L
pub const R_PINKY: u8 = 39; // KEY_SEMICOLON

// Thumb / modifier keys
pub const R_THUMB: u8 = 57; // KEY_SPACE — the chord-level "mod" bit
pub const WORD: u8 = 56; // KEY_LEFTALT (Linux) / KEY_LEFTGUI on Mac — mode selector

/// Bits belonging to the left hand's chord keys. Inner-index (G) is
/// included so it participates in per-hand firing — used by number
/// mode for position 5. Outside number mode it's a harmless no-op
/// because no phoneme/brief is mapped to the inner-index bit.
pub const LEFT_MASK: KeyMask = KeyMask::EMPTY
    .with(L_PINKY)
    .with(L_RING)
    .with(L_MID)
    .with(L_IDX)
    .with(L_IDX_INNER);

/// Bits belonging to the right hand's chord keys, including the
/// thumb/mod bit and the inner-index (H) key for number mode.
pub const RIGHT_MASK: KeyMask = KeyMask::EMPTY
    .with(R_IDX)
    .with(R_MID)
    .with(R_RING)
    .with(R_PINKY)
    .with(R_THUMB)
    .with(R_IDX_INNER);

/// Packed-bit index for a left-hand chord key. Bits 0-3 are the four
/// home fingers (index=0, middle=1, ring=2, pinky=3); bit 4 is the
/// inner-index stretch key (G), only reached in number mode. Returns
/// `None` for scancodes outside the left hand's chord set.
pub fn left_bit(scan: u8) -> Option<u8> {
    match scan {
        L_IDX => Some(0),
        L_MID => Some(1),
        L_RING => Some(2),
        L_PINKY => Some(3),
        L_IDX_INNER => Some(4),
        _ => None,
    }
}

/// Packed-bit index for a right-hand chord key. Bits 0-3 are the four
/// home fingers (index=0, middle=1, ring=2, pinky=3); bit 4 is the
/// thumb/mod key; bit 5 is the inner-index stretch key (H), only
/// reached in number mode. Returns `None` for scancodes outside the
/// right hand's chord set.
pub fn right_bit(scan: u8) -> Option<u8> {
    match scan {
        R_IDX => Some(0),
        R_MID => Some(1),
        R_RING => Some(2),
        R_PINKY => Some(3),
        R_THUMB => Some(4),
        R_IDX_INNER => Some(5),
        _ => None,
    }
}

/// Human-readable label for a chord-key scancode. Used by debug UIs
/// (rollover test, listen output) that want more than a raw number.
#[allow(dead_code)] // only used by macOS rollover_test for now
pub fn label(scan: u8) -> &'static str {
    match scan {
        L_PINKY => "L-pinky",
        L_RING => "L-ring",
        L_MID => "L-mid",
        L_IDX => "L-idx",
        L_IDX_INNER => "L-idx-inner",
        R_IDX_INNER => "R-idx-inner",
        R_IDX => "R-idx",
        R_MID => "R-mid",
        R_RING => "R-ring",
        R_PINKY => "R-pinky",
        R_THUMB => "R-thumb",
        WORD => "WORD",
        _ => "?",
    }
}
