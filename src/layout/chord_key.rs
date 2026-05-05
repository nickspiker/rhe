//! Language-neutral chord-key encoding and the per-language `BriefTable`.
//!
//! `ChordKey` is just a packed bitmask of held physical keys; it carries no language assumption. `BriefTable` maps `ChordKey` → word string and supports both unordered and ordered (first-key-down disambiguated) entries — the same shape every language uses.
//!
//! Both types live here so `layout::en::chords` and `layout::mri::chords` can share the implementation. Each language's `chords.rs` re-exports `ChordKey` and `BriefTable` so call sites that say `crate::layout::chords::ChordKey` continue to resolve.

use crate::key_mask::KeyMask;
use crate::scan;

/// A chord key — the set of physical keys that fire this chord.
///
/// Internally a `KeyMask` (256-bit, one bit per HID scancode), so it can represent any physical keyboard chord. For now rhe uses only 9 of those bits (4 right fingers + 4 left fingers + right thumb / "mod"), but the wider representation is what lets future features bind to inner-index keys, function row, etc.
///
/// Backward-compatible packed-bit accessors (`right_bits`, `left_bits`, `has_mod`) translate back to the legacy 9-bit layout for display/legacy-storage purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChordKey(KeyMask);

impl ChordKey {
    /// Empty chord (no keys pressed).
    pub const EMPTY: Self = Self(KeyMask::EMPTY);

    /// Build directly from a `KeyMask`. This is the path the state machine uses — no packed-bit round-trip, no hand/finger detour.
    pub fn from_mask(mask: KeyMask) -> Self {
        Self(mask)
    }

    /// Build from the legacy packed representation: 4 right-finger bits, 4 left-finger bits, plus the modkey flag. This is how `briefs_data.rs`, `suffixes_data.rs`, and `Phoneme::chord_key` still express chords — kept so those data tables don't have to change format yet.
    pub fn from_packed(right_fingers: u8, left_fingers: u8, has_mod: bool) -> Self {
        let mut mask = KeyMask::EMPTY;
        const LEFT: [u8; 4] = [scan::L_IDX, scan::L_MID, scan::L_RING, scan::L_PINKY];
        const RIGHT: [u8; 4] = [scan::R_IDX, scan::R_MID, scan::R_RING, scan::R_PINKY];
        for (bit, code) in LEFT.iter().enumerate() {
            if left_fingers & (1 << bit) != 0 {
                mask.set(*code);
            }
        }
        for (bit, code) in RIGHT.iter().enumerate() {
            if right_fingers & (1 << bit) != 0 {
                mask.set(*code);
            }
        }
        if has_mod {
            mask.set(scan::R_THUMB);
        }
        Self(mask)
    }

    /// Legacy u16 construction (used by some callers that round-trip a packed encoding). Bits 0-3 = right, bits 4-7 = left, bit 8 = mod.
    pub fn from_packed_u16(packed: u16) -> Self {
        Self::from_packed(
            (packed & 0xF) as u8,
            ((packed >> 4) & 0xF) as u8,
            packed & (1 << 8) != 0,
        )
    }

    /// The underlying 256-bit mask.
    pub fn mask(self) -> KeyMask {
        self.0
    }

    /// 4-bit packed right-finger bits (index=bit0, middle=bit1, ring=bit2, pinky=bit3).
    pub fn right_bits(self) -> u8 {
        let mut bits = 0u8;
        if self.0.test(scan::R_IDX) {
            bits |= 1 << 0;
        }
        if self.0.test(scan::R_MID) {
            bits |= 1 << 1;
        }
        if self.0.test(scan::R_RING) {
            bits |= 1 << 2;
        }
        if self.0.test(scan::R_PINKY) {
            bits |= 1 << 3;
        }
        bits
    }

    /// 4-bit packed left-finger bits (index=bit0, middle=bit1, ring=bit2, pinky=bit3).
    pub fn left_bits(self) -> u8 {
        let mut bits = 0u8;
        if self.0.test(scan::L_IDX) {
            bits |= 1 << 0;
        }
        if self.0.test(scan::L_MID) {
            bits |= 1 << 1;
        }
        if self.0.test(scan::L_RING) {
            bits |= 1 << 2;
        }
        if self.0.test(scan::L_PINKY) {
            bits |= 1 << 3;
        }
        bits
    }

    /// Is the mod bit (right thumb / spacebar) part of this chord?
    pub fn has_mod(self) -> bool {
        self.0.test(scan::R_THUMB)
    }
}

/// Brief table: maps `ChordKey` → word string.
///
/// Two flavours coexist:
/// - **Unordered briefs**: any down-order fires the entry. Default case, produced by `gen_briefs` from frequency × savings ranking.
/// - **Ordered briefs**: `(ChordKey, first_down_scancode)` → word. When a chord has any ordered entry, the chord becomes "claimed" and its unordered entry is suppressed. Only the scancode that goes down first decides which word fires. Used for homophone splits (to/too) and deliberate gesture vocabulary.
pub struct BriefTable {
    unordered: std::collections::HashMap<ChordKey, String>,
    ordered: std::collections::HashMap<(ChordKey, u8), String>,
    /// Chords with at least one ordered entry. Insertions to these chords via the unordered path are dropped.
    claimed: std::collections::HashSet<ChordKey>,
}

impl BriefTable {
    pub fn new() -> Self {
        Self {
            unordered: std::collections::HashMap::new(),
            ordered: std::collections::HashMap::new(),
            claimed: std::collections::HashSet::new(),
        }
    }

    /// Insert an unordered brief. Silently dropped if the chord is already claimed by an ordered entry — the ordered-first load order makes this a lockout.
    pub fn insert(&mut self, key: ChordKey, word: String) {
        if self.claimed.contains(&key) {
            return;
        }
        self.unordered.insert(key, word);
    }

    /// Insert an ordered brief. Claims the chord — future unordered inserts at the same key are dropped, and any already-inserted unordered entry is removed so lookup stays consistent.
    pub fn insert_ordered(&mut self, key: ChordKey, first_down: u8, word: String) {
        self.claimed.insert(key);
        self.unordered.remove(&key);
        self.ordered.insert((key, first_down), word);
    }

    /// Lookup the word for a chord.
    ///
    /// Claimed chords require `first_down` to match a registered ordered entry — any other starting finger returns `None`. Unclaimed chords fall through to the unordered table and ignore `first_down`.
    pub fn lookup(&self, key: ChordKey, first_down: Option<u8>) -> Option<&str> {
        if self.claimed.contains(&key) {
            let first = first_down?;
            return self.ordered.get(&(key, first)).map(String::as_str);
        }
        self.unordered.get(&key).map(String::as_str)
    }

    /// Iterate every (chord, first_down, word) entry. Unordered briefs yield `first_down = None`; ordered briefs yield the required first-down scancode. Used by the tutor for reverse word→chord lookup, and by `rhe briefs` for display.
    pub fn iter(&self) -> impl Iterator<Item = (&ChordKey, Option<u8>, &String)> {
        self.unordered
            .iter()
            .map(|(k, v)| (k, None, v))
            .chain(self.ordered.iter().map(|((k, fd), v)| (k, Some(*fd), v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_key_roundtrip() {
        let key = ChordKey::from_packed(0b0101, 0b0011, true);
        assert_eq!(key.right_bits(), 0b0101);
        assert_eq!(key.left_bits(), 0b0011);
        assert!(key.has_mod());
    }

    #[test]
    fn brief_table_ordered_semantics() {
        let mut table = BriefTable::new();
        let ab = ChordKey::from_packed(0b0001, 0b0001, false);

        // Ordered entries claim the chord. Two orderings → two words.
        table.insert_ordered(ab, scan::R_IDX, "to ".to_string());
        table.insert_ordered(ab, scan::L_IDX, "too ".to_string());

        // Unordered insert at a claimed chord is silently dropped.
        table.insert(ab, "zzz ".to_string());

        assert_eq!(table.lookup(ab, Some(scan::R_IDX)), Some("to "));
        assert_eq!(table.lookup(ab, Some(scan::L_IDX)), Some("too "));
        // Claimed chord + unrecognized first_down → nothing.
        assert_eq!(table.lookup(ab, Some(scan::R_MID)), None);
        assert_eq!(table.lookup(ab, None), None);

        // Unclaimed chord still works the old way.
        let other = ChordKey::from_packed(0b0010, 0, false);
        table.insert(other, "and ".to_string());
        assert_eq!(table.lookup(other, None), Some("and "));
        assert_eq!(table.lookup(other, Some(scan::R_MID)), Some("and "));
    }
}
