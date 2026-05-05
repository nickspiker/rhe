//! Te reo Māori phoneme inventory and chord-to-phoneme mapping.
//!
//! 20 phonemes total: 5 short vowels (a, e, i, o, u) + 5 long vowels (ā, ē, ī, ō, ū) + 10 consonants (h, k, m, n, ng, p, r, t, w, wh). Long vowels share the short vowel's left-hand chord and add **L-IDX-INNER** (inner-index key, "G" position) as a length modifier — the same-hand mechanical equivalent of English's R-thumb-as-voicing-modifier. Cross-hand thumb-modifier doesn't combine cleanly under the state machine's per-hand fire when word is held, so the modifier moves to the same hand as the vowel.
//!
//! Specific finger assignments are first-cut estimates; will be re-ranked once the TeHikuMedia Hansard frequency table is wired in. `ChordKey` and `BriefTable` are re-exported from the language-neutral `crate::layout::chord_key` so call sites that say `crate::layout::chords::ChordKey` continue to resolve regardless of active language.

pub use crate::layout::chord_key::{BriefTable, ChordKey};

use crate::key_mask::KeyMask;
use crate::scan;

/// The 20 te reo Māori phonemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phoneme {
    // Consonants (right hand) — 10 total
    H,
    K,
    M,
    N,
    Ng,
    P,
    R,
    T,
    W,
    Wh,

    // Short vowels (left hand) — 5
    A,
    E,
    I,
    O,
    U,

    // Long vowels (left hand + L-IDX-INNER) — 5
    ALong,
    ELong,
    ILong,
    OLong,
    ULong,
}

impl Phoneme {
    /// IPA representation. Māori phonemes are close to their grapheme spelling; macrons mark vowel length.
    pub fn to_ipa(self) -> &'static str {
        use Phoneme::*;
        match self {
            H => "h",
            K => "k",
            M => "m",
            N => "n",
            Ng => "ŋ",
            P => "p",
            R => "ɾ",
            T => "t",
            W => "w",
            Wh => "ɸ",
            A => "a",
            E => "e",
            I => "i",
            O => "o",
            U => "u",
            ALong => "aː",
            ELong => "eː",
            ILong => "iː",
            OLong => "oː",
            ULong => "uː",
        }
    }

    /// Grapheme spelling (the one the engine emits to the focused app). Native te reo orthography is 1:1 with the phoneme inventory; long vowels carry macrons.
    pub fn to_grapheme(self) -> &'static str {
        use Phoneme::*;
        match self {
            H => "h",
            K => "k",
            M => "m",
            N => "n",
            Ng => "ng",
            P => "p",
            R => "r",
            T => "t",
            W => "w",
            Wh => "wh",
            A => "a",
            E => "e",
            I => "i",
            O => "o",
            U => "u",
            ALong => "ā",
            ELong => "ē",
            ILong => "ī",
            OLong => "ō",
            ULong => "ū",
        }
    }

    /// The `ChordKey` for this phoneme.
    ///
    /// Right hand (consonants, 4-bit): assigned by rough frequency × chord effort. Same effort ranking as English (I < R < P < M < all4 < M+R < I+M < I+M+R < I+P < I+R …). 5 spare slots on the 4-bit grid plus all 16 thumb-paired slots are unused — chord space is wildly over-provisioned for Māori. First-cut frequency order is heuristic; revisit once Hansard corpus data is in.
    ///
    /// Left hand (vowels): the 5 short vowels claim 5 chords; long vowels share the same chord + L-IDX-INNER (the inner-index "G" key on the left hand) as a length modifier. Single-hand fire because the modifier and vowel are both left-hand, so the state machine's per-hand fire delivers the long-vowel chord intact when word is held.
    pub fn chord_key(self) -> ChordKey {
        use Phoneme::*;
        match self {
            // Consonants — right hand 4-bit. Ordered roughly by frequency × effort.
            T => ChordKey::from_packed(0b0001, 0, false), // R-IDX
            N => ChordKey::from_packed(0b0100, 0, false), // R-RING
            K => ChordKey::from_packed(0b1000, 0, false), // R-PINKY
            R => ChordKey::from_packed(0b0010, 0, false), // R-MID
            H => ChordKey::from_packed(0b1111, 0, false), // all four
            M => ChordKey::from_packed(0b0110, 0, false), // M+R
            P => ChordKey::from_packed(0b0011, 0, false), // I+M
            Ng => ChordKey::from_packed(0b0111, 0, false), // I+M+R
            W => ChordKey::from_packed(0b1001, 0, false), // I+P
            Wh => ChordKey::from_packed(0b0101, 0, false), // I+R

            // Short vowels — left hand 4-bit (no L-IDX-INNER).
            A => left_chord(0b0001, false), // L-IDX
            E => left_chord(0b0010, false), // L-MID
            I => left_chord(0b0100, false), // L-RING
            O => left_chord(0b1000, false), // L-PINKY
            U => left_chord(0b1111, false), // all four

            // Long vowels — same short chord + L-IDX-INNER (length modifier).
            ALong => left_chord(0b0001, true),
            ELong => left_chord(0b0010, true),
            ILong => left_chord(0b0100, true),
            OLong => left_chord(0b1000, true),
            ULong => left_chord(0b1111, true),
        }
    }
}

/// Build a left-hand `ChordKey` from 4-bit finger pattern + optional L-IDX-INNER (length modifier).
fn left_chord(left_4bit: u8, long: bool) -> ChordKey {
    let mut mask = KeyMask::EMPTY;
    const LEFT: [u8; 4] = [scan::L_IDX, scan::L_MID, scan::L_RING, scan::L_PINKY];
    for (bit, code) in LEFT.iter().enumerate() {
        if left_4bit & (1 << bit) != 0 {
            mask.set(*code);
        }
    }
    if long {
        mask.set(scan::L_IDX_INNER);
    }
    ChordKey::from_mask(mask)
}

/// Phoneme table: maps `ChordKey` → `Phoneme`. HashMap-backed so the 256-bit keyspace isn't a problem (only actual phoneme chords consume memory).
pub struct PhonemeTable {
    entries: std::collections::HashMap<ChordKey, Phoneme>,
}

impl PhonemeTable {
    pub fn new() -> Self {
        let mut entries = std::collections::HashMap::new();
        let all_phonemes = [
            Phoneme::H,
            Phoneme::K,
            Phoneme::M,
            Phoneme::N,
            Phoneme::Ng,
            Phoneme::P,
            Phoneme::R,
            Phoneme::T,
            Phoneme::W,
            Phoneme::Wh,
            Phoneme::A,
            Phoneme::E,
            Phoneme::I,
            Phoneme::O,
            Phoneme::U,
            Phoneme::ALong,
            Phoneme::ELong,
            Phoneme::ILong,
            Phoneme::OLong,
            Phoneme::ULong,
        ];
        for p in all_phonemes {
            entries.insert(p.chord_key(), p);
        }
        Self { entries }
    }

    pub fn lookup(&self, key: ChordKey) -> Option<Phoneme> {
        self.entries.get(&key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phoneme_table_covers_all_twenty() {
        let table = PhonemeTable::new();
        assert_eq!(table.entries.len(), 20);
    }

    #[test]
    fn no_chord_collisions() {
        // Every phoneme must hash to a unique ChordKey.
        let table = PhonemeTable::new();
        let phonemes = [
            Phoneme::H, Phoneme::K, Phoneme::M, Phoneme::N, Phoneme::Ng,
            Phoneme::P, Phoneme::R, Phoneme::T, Phoneme::W, Phoneme::Wh,
            Phoneme::A, Phoneme::E, Phoneme::I, Phoneme::O, Phoneme::U,
            Phoneme::ALong, Phoneme::ELong, Phoneme::ILong, Phoneme::OLong, Phoneme::ULong,
        ];
        for p in phonemes {
            assert_eq!(table.lookup(p.chord_key()), Some(p), "{:?} round-trip failed", p);
        }
    }

    #[test]
    fn long_vowel_adds_inner_index_to_short() {
        // Long vowel chord = short chord + L-IDX-INNER. Verifying for each pair.
        let pairs = [
            (Phoneme::A, Phoneme::ALong),
            (Phoneme::E, Phoneme::ELong),
            (Phoneme::I, Phoneme::ILong),
            (Phoneme::O, Phoneme::OLong),
            (Phoneme::U, Phoneme::ULong),
        ];
        for (short, long) in pairs {
            let short_mask = short.chord_key().mask();
            let long_mask = long.chord_key().mask();
            assert!(long_mask.test(scan::L_IDX_INNER), "{:?} should include L-IDX-INNER", long);
            assert!(!short_mask.test(scan::L_IDX_INNER), "{:?} should not include L-IDX-INNER", short);
            // Removing L-IDX-INNER from the long chord gets back the short chord.
            let mut long_minus_inner = long_mask;
            long_minus_inner.clear(scan::L_IDX_INNER);
            assert_eq!(long_minus_inner, short_mask, "{:?} - L-IDX-INNER should equal {:?}", long, short);
        }
    }

    #[test]
    fn consonants_are_right_hand_only() {
        for p in [Phoneme::T, Phoneme::N, Phoneme::K, Phoneme::Ng, Phoneme::Wh] {
            let key = p.chord_key();
            assert!(key.right_bits() != 0, "{:?} should be right hand", p);
            assert_eq!(key.left_bits(), 0, "{:?} should have no left hand", p);
            assert!(!key.has_mod(), "{:?} should not use thumb modifier", p);
        }
    }

    #[test]
    fn vowels_are_left_hand_only() {
        for p in [Phoneme::A, Phoneme::E, Phoneme::I, Phoneme::O, Phoneme::U] {
            let key = p.chord_key();
            assert!(key.left_bits() != 0, "{:?} should be left hand", p);
            assert_eq!(key.right_bits(), 0, "{:?} should have no right hand", p);
            assert!(!key.has_mod(), "{:?} should not use R-thumb", p);
        }
    }
}
