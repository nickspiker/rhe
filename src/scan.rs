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
