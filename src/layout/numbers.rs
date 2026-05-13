//! Number-mode chord → character lookup.
//!
//! Number mode is a sub-session of word-held: entered by pressing-and-releasing the mod key (right thumb) alone while word is held, exited when word is released (with a trailing space emitted).
//!
//! Single-finger chord → digit. Ten positions laid out right-to-left from R-pinky:
//!
//! ```text
//!   position:  0    1    2    3    4         5         6    7    8    9
//!   key:     R-Pky R-Rg R-Md R-Ix R-Idx-In  L-Idx-In L-Ix L-Md L-Rg L-Pky
//!   digit:    0    1    2    3    4         5         6    7    8    9
//! ```
//!
//! The inner-index keys (QWERTY G and H) only participate in number mode — they're silent in normal phoneme/brief typing.
//!
//! Mod-held + single-finger chord → symbol. Same ten positions, same layout, different output:
//!
//! ```text
//!   0: -   1: /   2: *   3: +   4: )   5: (   6: =   7: %   8: ^   9: ,
//! ```
//!
//! Right hand = basic arithmetic, left hand = comparison / grouping / separators. The matched parentheses sit on the two inner-index positions, mirrored visually across the keyboard's centerline.
//!
//! The mod bit can arrive two ways, distinguished by `first_down`:
//! - mod pressed *first*, then the finger → symbol (above table).
//! - finger pressed *first*, then mod → **spelled** digit ("five"), useful for prose where you want the word form without leaving number mode.
//!
//! Multi-finger chords, chords with no fingers (mod alone), and any scancode outside the ten positions return `None`. The interpreter treats a `None` as a silent no-op in number mode.

use super::chords::ChordKey;
use crate::key_mask::KeyMask;
use crate::scan;

/// Which of the ten positions (0..=9) a chord occupies, after optionally ignoring the mod/thumb bit. Returns `None` if the chord isn't a recognized number-mode chord shape.
///
/// Two chord shapes per position for digits 4 and 5: the inner-index stretch keys (QWERTY G/H) for standard keyboards that have those rows, plus an all-4-home-row-fingers chord (matching the M phoneme on the right and the Uw phoneme on the left) for stripped-down "rheboard" hardware that omits the inner-index keys. Both shapes coexist in software — software users get the choice; rheboard users get a complete number layout without the extra physical keys. Single-finger chords map to the other eight positions.
fn position(key: ChordKey, ignore_mod: bool) -> Option<u8> {
    let mut mask = key.mask();
    if ignore_mod {
        let mut clear = KeyMask::EMPTY;
        clear.set(scan::R_THUMB);
        mask &= !clear;
    }
    // All-4-home-row chords for digits 4 and 5 (rheboard layout). Checked before the single-finger fallback so multi-finger doesn't get rejected by `count_ones() != 1`.
    let all_four_right = KeyMask::EMPTY
        .with(scan::R_IDX)
        .with(scan::R_MID)
        .with(scan::R_RING)
        .with(scan::R_PINKY);
    if mask == all_four_right {
        return Some(4);
    }
    let all_four_left = KeyMask::EMPTY
        .with(scan::L_IDX)
        .with(scan::L_MID)
        .with(scan::L_RING)
        .with(scan::L_PINKY);
    if mask == all_four_left {
        return Some(5);
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

/// Digit for a single-finger chord with no mod. Returns `None` for multi-finger, mod-inclusive, or out-of-range chords.
pub fn chord_to_digit(key: ChordKey) -> Option<char> {
    if key.has_mod() {
        return None;
    }
    let pos = position(key, false)?;
    Some(DIGITS[pos as usize])
}

/// Symbol for a single-finger chord with mod held. Returns `None` if the chord lacks the mod bit, has zero or multi-finger bits beyond that, or falls outside the ten positions.
pub fn chord_to_symbol(key: ChordKey) -> Option<char> {
    if !key.has_mod() {
        return None;
    }
    let pos = position(key, true)?;
    Some(SYMBOLS[pos as usize])
}

/// Spelled word for a single-finger chord with mod held. Same position map as `chord_to_digit`; the caller discriminates symbol vs word by inspecting `first_down` on the incoming Chord event (mod-first = symbol, finger-first = word).
pub fn chord_to_digit_word(key: ChordKey) -> Option<&'static str> {
    if !key.has_mod() {
        return None;
    }
    let pos = position(key, true)?;
    Some(DIGIT_WORDS[pos as usize])
}

/// Glyph for a multi-finger single-hand chord (no mod) inside number mode. Right-hand chords mirror the lowercase Greek consonants from `crate::layout::symbols::chord_to_glyph` for effort ranks 5-14 — the chord shape per Greek letter is identical across symbol and number mode, so muscle memory transfers. The five most-frequent Greek consonants (λ β μ ρ π) sit on symbol-mode effort ranks 0-4 (single-finger or all-4), which are digits in number mode — those letters are reachable only via symbol mode.
///
/// Left-hand chords carry brackets and math operators that don't fit on the single-finger+mod operator slots; placement is independent of symbol mode (different glyph set).
///
/// All-4-home-row chords are excluded from both halves because they're claimed by digits 4 (right) and 5 (left) under the rheboard-friendly position map.
///
/// Returns `None` for single-finger chords (those are digits), mod-bearing chords (those are arithmetic operators or spelled digits), cross-hand rolls (ambiguous), and any chord shape not in the static table below.
pub fn chord_to_multi_glyph(key: ChordKey) -> Option<&'static str> {
    if key.has_mod() {
        return None;
    }
    let right = key.right_bits();
    let left = key.left_bits();
    if right != 0 && left != 0 {
        return None;
    }
    if right.count_ones() <= 1 && left.count_ones() <= 1 {
        return None;
    }
    Some(match (right, left) {
        // Right-hand Greek consonants (effort 5-14, matching `symbols::RIGHT_BY_EFFORT`).
        (0b0110, 0) => "σ", // 5  M+R
        (0b0011, 0) => "ω", // 6  I+M
        // 7 (I+M+R, 0b0111) — None in symbol-mode table; left empty here too.
        (0b1001, 0) => "τ", // 8  I+P
        (0b0101, 0) => "δ", // 9  I+R
        (0b1100, 0) => "η", // 10 R+P
        (0b1110, 0) => "ψ", // 11 M+R+P
        (0b1010, 0) => "κ", // 12 M+P
        (0b1101, 0) => "φ", // 13 I+R+P
        (0b1011, 0) => "ζ", // 14 I+M+P
        // Left-hand brackets and math operators. Independent placement (different glyph set, no symbol-mode parallel).
        (0, 0b0011) => "[",
        (0, 0b0110) => "]",
        (0, 0b0101) => "{",
        (0, 0b1001) => "}",
        (0, 0b1100) => "<",
        (0, 0b1010) => ">",
        (0, 0b0111) => "√",
        (0, 0b1110) => "∞",
        (0, 0b1011) => "∂",
        (0, 0b1101) => "∫",
        _ => return None,
    })
}

const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
const SYMBOLS: [char; 10] = ['-', '/', '*', '+', ')', '(', '=', '%', '^', ','];
const DIGIT_WORDS: [&str; 10] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];

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

    #[test]
    fn digit_word_all_ten_positions() {
        assert_eq!(chord_to_digit_word(with_mod(scan::R_PINKY)), Some("zero"));
        assert_eq!(chord_to_digit_word(with_mod(scan::R_RING)), Some("one"));
        assert_eq!(chord_to_digit_word(with_mod(scan::R_MID)), Some("two"));
        assert_eq!(chord_to_digit_word(with_mod(scan::R_IDX)), Some("three"));
        assert_eq!(
            chord_to_digit_word(with_mod(scan::R_IDX_INNER)),
            Some("four")
        );
        assert_eq!(
            chord_to_digit_word(with_mod(scan::L_IDX_INNER)),
            Some("five")
        );
        assert_eq!(chord_to_digit_word(with_mod(scan::L_IDX)), Some("six"));
        assert_eq!(chord_to_digit_word(with_mod(scan::L_MID)), Some("seven"));
        assert_eq!(chord_to_digit_word(with_mod(scan::L_RING)), Some("eight"));
        assert_eq!(chord_to_digit_word(with_mod(scan::L_PINKY)), Some("nine"));
    }

    #[test]
    fn digit_word_without_mod_none() {
        assert_eq!(chord_to_digit_word(single(scan::R_IDX)), None);
    }

    #[test]
    fn digit_word_thumb_only_none() {
        assert_eq!(chord_to_digit_word(single(scan::R_THUMB)), None);
    }

    fn double(a: u8, b: u8) -> ChordKey {
        ChordKey::from_mask(KeyMask::EMPTY.with(a).with(b))
    }

    #[test]
    fn multi_glyph_right_hand_greek() {
        // R-IDX + R-MID (effort 6, I+M) → ω in the new layout.
        assert_eq!(chord_to_multi_glyph(double(scan::R_IDX, scan::R_MID)), Some("ω"));
        // R-MID + R-RING (effort 5, M+R) → σ.
        assert_eq!(
            chord_to_multi_glyph(double(scan::R_MID, scan::R_RING)),
            Some("σ")
        );
        // R-IDX + R-PINKY (effort 8, I+P) → τ.
        assert_eq!(
            chord_to_multi_glyph(double(scan::R_IDX, scan::R_PINKY)),
            Some("τ")
        );
        // π is on effort rank 4 (all-4-right) in symbol mode; that chord is digit 4 in number mode, so π is NOT reachable here.
    }

    #[test]
    fn multi_glyph_left_hand_brackets() {
        assert_eq!(chord_to_multi_glyph(double(scan::L_IDX, scan::L_MID)), Some("["));
        assert_eq!(chord_to_multi_glyph(double(scan::L_MID, scan::L_RING)), Some("]"));
        assert_eq!(chord_to_multi_glyph(double(scan::L_IDX, scan::L_RING)), Some("{"));
    }

    #[test]
    fn multi_glyph_single_finger_none() {
        // Single-finger chords are digits, not multi-glyphs.
        assert_eq!(chord_to_multi_glyph(single(scan::R_IDX)), None);
        assert_eq!(chord_to_multi_glyph(single(scan::L_PINKY)), None);
    }

    #[test]
    fn multi_glyph_with_mod_none() {
        // Mod-bearing chords are arithmetic ops or spelled words.
        let mut m = KeyMask::EMPTY;
        m.set(scan::R_IDX);
        m.set(scan::R_MID);
        m.set(scan::R_THUMB);
        assert_eq!(chord_to_multi_glyph(ChordKey::from_mask(m)), None);
    }

    #[test]
    fn multi_glyph_cross_hand_none() {
        // Cross-hand rolls aren't currently mapped (ambiguous mnemonic).
        assert_eq!(chord_to_multi_glyph(double(scan::R_IDX, scan::L_IDX)), None);
    }

    fn all_four_right() -> ChordKey {
        ChordKey::from_mask(
            KeyMask::EMPTY
                .with(scan::R_IDX)
                .with(scan::R_MID)
                .with(scan::R_RING)
                .with(scan::R_PINKY),
        )
    }

    fn all_four_left() -> ChordKey {
        ChordKey::from_mask(
            KeyMask::EMPTY
                .with(scan::L_IDX)
                .with(scan::L_MID)
                .with(scan::L_RING)
                .with(scan::L_PINKY),
        )
    }

    fn all_four_right_with_mod() -> ChordKey {
        ChordKey::from_mask(
            KeyMask::EMPTY
                .with(scan::R_IDX)
                .with(scan::R_MID)
                .with(scan::R_RING)
                .with(scan::R_PINKY)
                .with(scan::R_THUMB),
        )
    }

    fn all_four_left_with_mod() -> ChordKey {
        ChordKey::from_mask(
            KeyMask::EMPTY
                .with(scan::L_IDX)
                .with(scan::L_MID)
                .with(scan::L_RING)
                .with(scan::L_PINKY)
                .with(scan::R_THUMB),
        )
    }

    /// Rheboard layout: all-4-right home row = digit 4 (replacing the inner-index stretch that the rheboard hardware doesn't have). Standard keyboards keep R_IDX_INNER as an alternate path — both produce '4'.
    #[test]
    fn digit_4_via_all_four_right() {
        assert_eq!(chord_to_digit(all_four_right()), Some('4'));
        assert_eq!(chord_to_digit(single(scan::R_IDX_INNER)), Some('4'));
    }

    #[test]
    fn digit_5_via_all_four_left() {
        assert_eq!(chord_to_digit(all_four_left()), Some('5'));
        assert_eq!(chord_to_digit(single(scan::L_IDX_INNER)), Some('5'));
    }

    /// All-4 + mod still emits the position-4/5 operator, same as the inner-index + mod path.
    #[test]
    fn symbol_4_5_via_all_four_with_mod() {
        assert_eq!(chord_to_symbol(all_four_right_with_mod()), Some(')'));
        assert_eq!(chord_to_symbol(all_four_left_with_mod()), Some('('));
    }

    /// Spelled-word path: all-4 + mod (finger-first via `first_down`) gives "four" / "five", same as inner-index + mod.
    #[test]
    fn digit_word_4_5_via_all_four_with_mod() {
        assert_eq!(chord_to_digit_word(all_four_right_with_mod()), Some("four"));
        assert_eq!(chord_to_digit_word(all_four_left_with_mod()), Some("five"));
    }

    /// All-4-home-row chords are NOT in the multi-glyph table (they're digits now). Catches future drift if someone re-adds µ or ∇ on those shapes.
    #[test]
    fn multi_glyph_excludes_all_four() {
        assert_eq!(chord_to_multi_glyph(all_four_right()), None);
        assert_eq!(chord_to_multi_glyph(all_four_left()), None);
    }
}
