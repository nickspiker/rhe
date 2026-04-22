//! Phoneme-to-chord mapping, `ChordKey` encoding, `PhonemeTable` and `BriefTable`.

use crate::chord_state::Chord;
use crate::key_mask::KeyMask;
use crate::scan;

/// A chord key — the set of physical keys that fire this chord.
///
/// Internally a `KeyMask` (256-bit, one bit per HID scancode), so it can
/// represent any physical keyboard chord. For now rhe uses only 9 of
/// those bits (4 right fingers + 4 left fingers + right thumb / "mod"),
/// but the wider representation is what lets future features bind to
/// inner-index keys, function row, etc.
///
/// Backward-compatible packed-bit accessors (`right_bits`, `left_bits`,
/// `has_mod`) translate back to the legacy 9-bit layout for display/
/// legacy-storage purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChordKey(KeyMask);

impl ChordKey {
    /// Empty chord (no keys pressed).
    pub const EMPTY: Self = Self(KeyMask::EMPTY);

    /// Build from a state-machine-level `Chord`.
    pub fn from_chord(chord: &Chord) -> Self {
        Self::from_packed(chord.right.0 & 0xF, chord.left.0 & 0xF, chord.modkey)
    }

    /// Build from the legacy packed representation: 4 right-finger bits,
    /// 4 left-finger bits, plus the modkey flag. This is how
    /// `briefs_data.rs`, `suffixes_data.rs`, and `Phoneme::chord_key`
    /// still express chords — kept so those data tables don't have to
    /// change format yet.
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

    /// Legacy u16 construction (used by some callers that round-trip
    /// a packed encoding). Bits 0-3 = right, bits 4-7 = left, bit 8 = mod.
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
        if self.0.test(scan::R_IDX) { bits |= 1 << 0; }
        if self.0.test(scan::R_MID) { bits |= 1 << 1; }
        if self.0.test(scan::R_RING) { bits |= 1 << 2; }
        if self.0.test(scan::R_PINKY) { bits |= 1 << 3; }
        bits
    }

    /// 4-bit packed left-finger bits (index=bit0, middle=bit1, ring=bit2, pinky=bit3).
    pub fn left_bits(self) -> u8 {
        let mut bits = 0u8;
        if self.0.test(scan::L_IDX) { bits |= 1 << 0; }
        if self.0.test(scan::L_MID) { bits |= 1 << 1; }
        if self.0.test(scan::L_RING) { bits |= 1 << 2; }
        if self.0.test(scan::L_PINKY) { bits |= 1 << 3; }
        bits
    }

    /// Is the mod bit (right thumb / spacebar) part of this chord?
    pub fn has_mod(self) -> bool {
        self.0.test(scan::R_THUMB)
    }
}

/// The 39 English phonemes, split into consonants (right hand) and vowels (left hand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phoneme {
    // Consonants (right hand) — 24 total
    T,
    D,
    S,
    Z,
    K,
    G,
    P,
    B,
    N,
    M,
    R,
    Dh, // ð
    L,
    H,
    F,
    V,
    W,
    Th, // θ
    Sh,
    Zh, // ʃ ʒ
    Ch,
    Jh, // tʃ dʒ
    Ng, // ŋ
    Y,  // j

    // Vowels (left hand) — 15 total
    Ah,
    Ih,
    Eh,
    Ae, // ʌ/ə ɪ ɛ æ
    Iy,
    Aa,
    Ey,
    Er, // iː ɑ eɪ ɝ
    Ay,
    Ow,
    Ao, // aɪ oʊ ɔ
    Uw,
    Aw,
    Uh,
    Oy, // uː aʊ ʊ ɔɪ
}

impl Phoneme {
    /// Convert ARPABET symbol (from CMU dict) to Phoneme.
    pub fn from_arpabet(s: &str) -> Option<Self> {
        match s {
            "T" => Some(Self::T),
            "D" => Some(Self::D),
            "S" => Some(Self::S),
            "Z" => Some(Self::Z),
            "K" => Some(Self::K),
            "G" => Some(Self::G),
            "P" => Some(Self::P),
            "B" => Some(Self::B),
            "N" => Some(Self::N),
            "M" => Some(Self::M),
            "R" => Some(Self::R),
            "DH" => Some(Self::Dh),
            "L" => Some(Self::L),
            "HH" => Some(Self::H),
            "F" => Some(Self::F),
            "V" => Some(Self::V),
            "W" => Some(Self::W),
            "TH" => Some(Self::Th),
            "SH" => Some(Self::Sh),
            "ZH" => Some(Self::Zh),
            "CH" => Some(Self::Ch),
            "JH" => Some(Self::Jh),
            "NG" => Some(Self::Ng),
            "Y" => Some(Self::Y),

            "AH" => Some(Self::Ah),
            "IH" => Some(Self::Ih),
            "EH" => Some(Self::Eh),
            "AE" => Some(Self::Ae),
            "IY" => Some(Self::Iy),
            "AA" => Some(Self::Aa),
            "EY" => Some(Self::Ey),
            "ER" => Some(Self::Er),
            "AY" => Some(Self::Ay),
            "OW" => Some(Self::Ow),
            "AO" => Some(Self::Ao),
            "UW" => Some(Self::Uw),
            "AW" => Some(Self::Aw),
            "UH" => Some(Self::Uh),
            "OY" => Some(Self::Oy),

            _ => None,
        }
    }

    /// IPA representation.
    pub fn to_ipa(self) -> &'static str {
        match self {
            Self::T => "t",
            Self::D => "d",
            Self::S => "s",
            Self::Z => "z",
            Self::K => "k",
            Self::G => "g",
            Self::P => "p",
            Self::B => "b",
            Self::N => "n",
            Self::M => "m",
            Self::R => "ɹ",
            Self::Dh => "ð",
            Self::L => "l",
            Self::H => "h",
            Self::F => "f",
            Self::V => "v",
            Self::W => "w",
            Self::Th => "θ",
            Self::Sh => "ʃ",
            Self::Zh => "ʒ",
            Self::Ch => "tʃ",
            Self::Jh => "dʒ",
            Self::Ng => "ŋ",
            Self::Y => "j",

            Self::Ah => "ʌ",
            Self::Ih => "ɪ",
            Self::Eh => "ɛ",
            Self::Ae => "æ",
            Self::Iy => "iː",
            Self::Aa => "ɑ",
            Self::Ey => "eɪ",
            Self::Er => "ɝ",
            Self::Ay => "aɪ",
            Self::Ow => "oʊ",
            Self::Ao => "ɔ",
            Self::Uw => "uː",
            Self::Aw => "aʊ",
            Self::Uh => "ʊ",
            Self::Oy => "ɔɪ",
        }
    }

    /// The ChordKey for this phoneme.
    ///
    /// Mapped by frequency × measured chord effort from bench data.
    /// Each finger combo appears twice: without mod (top 15) and with mod (next 9).
    /// Easiest combo = most frequent phoneme.
    ///
    /// Effort ranking (measured): I < R < P < M < all4 < M+R < I+M < I+M+R
    ///   < I+P < I+R < R+P < M+R+P < M+P < I+R+P < I+M+P
    pub fn chord_key(self) -> ChordKey {
        use Phoneme::*;
        let (right, left, modkey) = match self {
            // Consonants: right hand, no mod (top 15 by frequency)
            T  => (0b0001, 0, false), // rank 1, 165M, index
            S  => (0b0100, 0, false), // rank 3, 110M, ring
            D  => (0b1000, 0, false), // rank 5, 87M, pinky
            R  => (0b0010, 0, false), // rank 4, 91M, middle (swapped w/ D by freq pair)
            M  => (0b1111, 0, false), // rank 7 (but paired), all4
            L  => (0b0110, 0, false), // rank 6 (paired), middle+ring
            K  => (0b0011, 0, false), // rank 8 (paired), index+middle
            Dh => (0b0111, 0, false), // rank 9, index+middle+ring
            W  => (0b1001, 0, false), // rank 10, index+pinky
            Z  => (0b0101, 0, false), // rank 11, index+ring
            Y  => (0b1100, 0, false), // rank 12, ring+pinky
            H  => (0b1110, 0, false), // rank 13, middle+ring+pinky
            F  => (0b1010, 0, false), // rank 15 (paired), middle+pinky — spare area starts
            B  => (0b1101, 0, false), // rank 14 (paired), index+ring+pinky
            P  => (0b1011, 0, false), // rank 16 (paired), index+middle+pinky
            // Consonants: right hand, with mod (next 9 by frequency)
            N  => (0b0001, 0, true),  // rank 2, 141M, index+mod
            V  => (0b0100, 0, true),  // rank 17, ring+mod
            Ng => (0b1000, 0, true),  // rank 18, pinky+mod
            G  => (0b0010, 0, true),  // rank 19, middle+mod
            Sh => (0b1111, 0, true),  // rank 20, all4+mod
            Th => (0b0110, 0, true),  // rank 21, middle+ring+mod
            Jh => (0b0011, 0, true),  // rank 22, index+middle+mod
            Ch => (0b0111, 0, true),  // rank 23, index+middle+ring+mod
            Zh => (0b1001, 0, true),  // rank 24, index+pinky+mod
            // Vowels: left hand, no mod (all 15 by frequency)
            Ah => (0, 0b0001, false), // rank 1, 182M, index
            Ih => (0, 0b0100, false), // rank 2, 126M, ring
            Iy => (0, 0b1000, false), // rank 3, 85M, pinky
            Eh => (0, 0b0010, false), // rank 4, 77M, middle
            Uw => (0, 0b1111, false), // rank 5, 68M, all4
            Ay => (0, 0b0110, false), // rank 6, 64M, middle+ring
            Ae => (0, 0b0011, false), // rank 7, 59M, index+middle
            Aa => (0, 0b0111, false), // rank 8, 52M, index+middle+ring
            Er => (0, 0b1001, false), // rank 9, 44M, index+pinky
            Ow => (0, 0b0101, false), // rank 10, 40M, index+ring
            Ey => (0, 0b1100, false), // rank 11, 36M, ring+pinky
            Ao => (0, 0b1110, false), // rank 12, 36M, middle+ring+pinky
            Aw => (0, 0b1010, false), // rank 13, 17M, middle+pinky
            Uh => (0, 0b1101, false), // rank 14, 11M, index+ring+pinky
            Oy => (0, 0b1011, false), // rank 15, 2M, index+middle+pinky
        };
        ChordKey::from_packed(right, left, modkey)
    }
}

/// Phoneme table: maps ChordKey → Phoneme. HashMap-backed so the 256-bit
/// keyspace isn't a problem (only actual phoneme chords consume memory).
pub struct PhonemeTable {
    entries: std::collections::HashMap<ChordKey, Phoneme>,
}

impl PhonemeTable {
    /// Build the table from all phoneme definitions.
    pub fn new() -> Self {
        let mut entries = std::collections::HashMap::new();
        let all_phonemes = [
            Phoneme::T, Phoneme::D, Phoneme::S, Phoneme::Z, Phoneme::K,
            Phoneme::G, Phoneme::P, Phoneme::B, Phoneme::N, Phoneme::M,
            Phoneme::R, Phoneme::Dh, Phoneme::L, Phoneme::H, Phoneme::F,
            Phoneme::V, Phoneme::W, Phoneme::Th, Phoneme::Sh, Phoneme::Zh,
            Phoneme::Ch, Phoneme::Jh, Phoneme::Ng, Phoneme::Y,
            Phoneme::Ah, Phoneme::Ih, Phoneme::Eh, Phoneme::Ae, Phoneme::Iy,
            Phoneme::Aa, Phoneme::Ey, Phoneme::Er, Phoneme::Ay, Phoneme::Ow,
            Phoneme::Ao, Phoneme::Uw, Phoneme::Aw, Phoneme::Uh, Phoneme::Oy,
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

/// Brief table: maps ChordKey → word string (for instant output without space).
pub struct BriefTable {
    entries: std::collections::HashMap<ChordKey, String>,
}

impl BriefTable {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: ChordKey, word: String) {
        self.entries.insert(key, word);
    }

    pub fn lookup(&self, key: ChordKey) -> Option<&str> {
        self.entries.get(&key).map(String::as_str)
    }

    /// Iterate over all (chord, word) entries. Used by the tutor to find
    /// which chord produces a given word (reverse lookup).
    pub fn iter(&self) -> impl Iterator<Item = (&ChordKey, &String)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_state::FingerChord;

    #[test]
    fn chord_key_roundtrip() {
        let chord = Chord {
            right: FingerChord(0b0101),
            left: FingerChord(0b0011),
            modkey: true,
            space_held: false,
        };
        let key = ChordKey::from_chord(&chord);
        assert_eq!(key.right_bits(), 0b0101);
        assert_eq!(key.left_bits(), 0b0011);
        assert!(key.has_mod());
    }

    #[test]
    fn phoneme_table_covers_all() {
        let table = PhonemeTable::new();
        // Every phoneme should be reachable
        assert_eq!(table.lookup(Phoneme::T.chord_key()), Some(Phoneme::T));
        assert_eq!(table.lookup(Phoneme::Ah.chord_key()), Some(Phoneme::Ah));
        assert_eq!(table.lookup(Phoneme::Jh.chord_key()), Some(Phoneme::Jh));
        assert_eq!(table.lookup(Phoneme::Oy.chord_key()), Some(Phoneme::Oy));
    }

    #[test]
    fn no_phoneme_collisions() {
        let table = PhonemeTable::new();
        assert_eq!(table.entries.len(), 39);
    }

    #[test]
    fn consonants_are_right_hand() {
        for p in [Phoneme::T, Phoneme::D, Phoneme::S, Phoneme::N, Phoneme::Ng] {
            let key = p.chord_key();
            assert!(key.right_bits() != 0, "{:?} should be right hand", p);
            assert_eq!(key.left_bits(), 0, "{:?} should have no left hand", p);
        }
    }

    #[test]
    fn vowels_are_left_hand() {
        for p in [Phoneme::Ah, Phoneme::Ih, Phoneme::Ey, Phoneme::Oy] {
            let key = p.chord_key();
            assert_eq!(key.right_bits(), 0, "{:?} should have no right hand", p);
            assert!(key.left_bits() != 0, "{:?} should be left hand", p);
        }
    }
}
