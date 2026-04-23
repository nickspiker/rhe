//! Number-mode chord → character lookup.
//!
//! Number mode is a sub-session of word-held: entered by tapping the
//! mod key (right thumb) while word is held, exited when word is
//! released (with a trailing space emitted).
//!
//! Single-finger chord → digit. Ten positions laid out right-to-left
//! from R-pinky:
//!
//! ```text
//!   position:  0    1    2    3    4         5         6    7    8    9
//!   key:     R-Pky R-Rg R-Md R-Ix R-Idx-In  L-Idx-In L-Ix L-Md L-Rg L-Pky
//!   digit:    0    1    2    3    4         5         6    7    8    9
//! ```
//!
//! The inner-index keys (QWERTY G and H) only participate in number
//! mode — they're silent in normal phoneme/brief typing.
//!
//! Mod-held + single-finger chord → symbol. Same ten positions, same
//! layout, different output:
//!
//! ```text
//!   0: -   1: /   2: *   3: +   4: )   5: (   6: =   7: %   8: ^   9: ,
//! ```
//!
//! Right hand = basic arithmetic, left hand = comparison / grouping /
//! separators. The matched parentheses sit on the two inner-index
//! positions, mirrored visually across the keyboard's centerline.
//!
//! Multi-finger chords, chords with no fingers (mod alone), and any
//! scancode outside the ten positions return `None`. The interpreter
//! treats a `None` as a silent no-op in number mode.

use crate::chord_map::ChordKey;
use crate::key_mask::KeyMask;
use crate::scan;

/// Which of the ten positions (0..=9) a single-finger chord occupies,
/// after optionally ignoring the mod/thumb bit. Returns `None` if the
/// chord isn't a single-finger press on one of the ten number-mode
/// positions.
fn position(key: ChordKey, ignore_mod: bool) -> Option<u8> {
    let mut mask = key.mask();
    if ignore_mod {
        let mut clear = KeyMask::EMPTY;
        clear.set(scan::R_THUMB);
        mask &= !clear;
    }
    if mask.count_ones() != 1 {
        return None;
    }
    let bit = mask.iter().next()?;
    Some(match bit {
        scan::R_PINKY => 0,
        scan::R_RING => 1,
        scan::R_MID => 2,
        scan::R_IDX => 3,
        scan::R_IDX_INNER => 4,
        scan::L_IDX_INNER => 5,
        scan::L_IDX => 6,
        scan::L_MID => 7,
        scan::L_RING => 8,
        scan::L_PINKY => 9,
        _ => return None,
    })
}

/// Digit for a single-finger chord with no mod. Returns `None` for
/// multi-finger, mod-inclusive, or out-of-range chords.
pub fn chord_to_digit(key: ChordKey) -> Option<char> {
    if key.has_mod() {
        return None;
    }
    let pos = position(key, false)?;
    Some(DIGITS[pos as usize])
}

/// Symbol for a single-finger chord with mod held. Returns `None` if
/// the chord lacks the mod bit, has zero or multi-finger bits beyond
/// that, or falls outside the ten positions.
pub fn chord_to_symbol(key: ChordKey) -> Option<char> {
    if !key.has_mod() {
        return None;
    }
    let pos = position(key, true)?;
    Some(SYMBOLS[pos as usize])
}

const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
const SYMBOLS: [char; 10] = ['-', '/', '*', '+', ')', '(', '=', '%', '^', ','];

#[cfg(test)]
mod tests {
    use super::*;

    fn single(code: u8) -> ChordKey {
        ChordKey::from_mask(KeyMask::EMPTY.with(code))
    }

    fn with_mod(code: u8) -> ChordKey {
        ChordKey::from_mask(KeyMask::EMPTY.with(code).with(scan::R_THUMB))
    }

    #[test]
    fn digit_all_ten_positions() {
        assert_eq!(chord_to_digit(single(scan::R_PINKY)), Some('0'));
        assert_eq!(chord_to_digit(single(scan::R_RING)), Some('1'));
        assert_eq!(chord_to_digit(single(scan::R_MID)), Some('2'));
        assert_eq!(chord_to_digit(single(scan::R_IDX)), Some('3'));
        assert_eq!(chord_to_digit(single(scan::R_IDX_INNER)), Some('4'));
        assert_eq!(chord_to_digit(single(scan::L_IDX_INNER)), Some('5'));
        assert_eq!(chord_to_digit(single(scan::L_IDX)), Some('6'));
        assert_eq!(chord_to_digit(single(scan::L_MID)), Some('7'));
        assert_eq!(chord_to_digit(single(scan::L_RING)), Some('8'));
        assert_eq!(chord_to_digit(single(scan::L_PINKY)), Some('9'));
    }

    #[test]
    fn digit_multi_finger_none() {
        let mut m = KeyMask::EMPTY;
        m.set(scan::R_PINKY);
        m.set(scan::R_RING);
        assert_eq!(chord_to_digit(ChordKey::from_mask(m)), None);
    }

    #[test]
    fn digit_mod_none() {
        // Mod held turns this into a symbol, not a digit.
        assert_eq!(chord_to_digit(with_mod(scan::R_IDX)), None);
    }

    #[test]
    fn digit_thumb_alone_none() {
        assert_eq!(chord_to_digit(single(scan::R_THUMB)), None);
    }

    #[test]
    fn symbol_all_ten_positions() {
        assert_eq!(chord_to_symbol(with_mod(scan::R_PINKY)), Some('-'));
        assert_eq!(chord_to_symbol(with_mod(scan::R_RING)), Some('/'));
        assert_eq!(chord_to_symbol(with_mod(scan::R_MID)), Some('*'));
        assert_eq!(chord_to_symbol(with_mod(scan::R_IDX)), Some('+'));
        assert_eq!(chord_to_symbol(with_mod(scan::R_IDX_INNER)), Some(')'));
        assert_eq!(chord_to_symbol(with_mod(scan::L_IDX_INNER)), Some('('));
        assert_eq!(chord_to_symbol(with_mod(scan::L_IDX)), Some('='));
        assert_eq!(chord_to_symbol(with_mod(scan::L_MID)), Some('%'));
        assert_eq!(chord_to_symbol(with_mod(scan::L_RING)), Some('^'));
        assert_eq!(chord_to_symbol(with_mod(scan::L_PINKY)), Some(','));
    }

    #[test]
    fn symbol_without_mod_none() {
        // Plain finger press is a digit, not a symbol.
        assert_eq!(chord_to_symbol(single(scan::R_IDX)), None);
    }

    #[test]
    fn symbol_thumb_only_none() {
        // Thumb alone is the entry/decimal gesture, not a symbol chord.
        assert_eq!(chord_to_symbol(single(scan::R_THUMB)), None);
    }

    #[test]
    fn symbol_multi_finger_none() {
        // Mod + two fingers isn't a registered operator.
        let mut m = KeyMask::EMPTY;
        m.set(scan::R_THUMB);
        m.set(scan::R_IDX);
        m.set(scan::R_MID);
        assert_eq!(chord_to_symbol(ChordKey::from_mask(m)), None);
    }
}
