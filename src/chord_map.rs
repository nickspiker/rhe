//! Phoneme-to-chord mapping, `ChordKey` encoding, `PhonemeTable` and `BriefTable`.

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

    /// Build directly from a `KeyMask`. This is the path the state
    /// machine uses — no packed-bit round-trip, no hand/finger detour.
    pub fn from_mask(mask: KeyMask) -> Self {
        Self(mask)
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

    /// Approximate English grapheme spelling for this phoneme.
    /// Used when a phoneme sequence doesn't resolve to a dictionary word
    /// and the user wants ASCII autospell output instead of IPA.
    /// Crude by design — English spelling is irregular — but readable and
    /// representable in any keyboard layout.
    pub fn to_grapheme(self) -> &'static str {
        use Phoneme::*;
        match self {
            // Consonants
            T => "t",   D => "d",   S => "s",   Z => "z",
            K => "k",   G => "g",   P => "p",   B => "b",
            N => "n",   M => "m",   R => "r",   L => "l",
            H => "h",   F => "f",   V => "v",   W => "w",
            Y => "y",
            Th => "th", Dh => "th",
            Sh => "sh", Zh => "zh",
            Ch => "ch", Jh => "j",
            Ng => "ng",
            // Vowels
            Ah => "uh", Ih => "i",  Eh => "e",  Ae => "a",
            Iy => "ee", Aa => "ah", Ey => "ay", Er => "er",
            Ay => "y",  Ow => "o",  Ao => "aw", Uw => "oo",
            Aw => "ow", Uh => "oo", Oy => "oy",
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

/// Brief table: maps `ChordKey` → word string.
///
/// Two flavours coexist:
/// - **Unordered briefs**: any down-order fires the entry. Default case,
///   produced by `gen_briefs` from frequency × savings ranking.
/// - **Ordered briefs**: `(ChordKey, first_down_scancode)` → word. When
///   a chord has any ordered entry, the chord becomes "claimed" and its
///   unordered entry is suppressed. Only the scancode that goes down
///   first decides which word fires. Used for homophone splits
///   (to/too) and deliberate gesture vocabulary.
pub struct BriefTable {
    unordered: std::collections::HashMap<ChordKey, String>,
    ordered: std::collections::HashMap<(ChordKey, u8), String>,
    /// Chords with at least one ordered entry. Insertions to these
    /// chords via the unordered path are dropped.
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

    /// Insert an unordered brief. Silently dropped if the chord is
    /// already claimed by an ordered entry — the ordered-first load
    /// order makes this a lockout.
    pub fn insert(&mut self, key: ChordKey, word: String) {
        if self.claimed.contains(&key) {
            return;
        }
        self.unordered.insert(key, word);
    }

    /// Insert an ordered brief. Claims the chord — future unordered
    /// inserts at the same key are dropped, and any already-inserted
    /// unordered entry is removed so lookup stays consistent.
    pub fn insert_ordered(&mut self, key: ChordKey, first_down: u8, word: String) {
        self.claimed.insert(key);
        self.unordered.remove(&key);
        self.ordered.insert((key, first_down), word);
    }

    /// Lookup the word for a chord.
    ///
    /// Claimed chords require `first_down` to match a registered ordered
    /// entry — any other starting finger returns `None`. Unclaimed
    /// chords fall through to the unordered table and ignore `first_down`.
    pub fn lookup(&self, key: ChordKey, first_down: Option<u8>) -> Option<&str> {
        if self.claimed.contains(&key) {
            let first = first_down?;
            return self.ordered.get(&(key, first)).map(String::as_str);
        }
        self.unordered.get(&key).map(String::as_str)
    }

    /// Iterate every (chord, first_down, word) entry. Unordered briefs
    /// yield `first_down = None`; ordered briefs yield the required
    /// first-down scancode. Used by the tutor for reverse word→chord
    /// lookup, and by `rhe briefs` for display.
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
    fn brief_table_ordered_semantics() {
        let mut table = BriefTable::new();
        let ab = ChordKey::from_packed(0b0001, 0b0001, false);

        // Ordered entries claim the chord. Two orderings → two words.
        table.insert_ordered(ab, crate::scan::R_IDX, "to ".to_string());
        table.insert_ordered(ab, crate::scan::L_IDX, "too ".to_string());

        // Unordered insert at a claimed chord is silently dropped.
        table.insert(ab, "zzz ".to_string());

        assert_eq!(table.lookup(ab, Some(crate::scan::R_IDX)), Some("to "));
        assert_eq!(table.lookup(ab, Some(crate::scan::L_IDX)), Some("too "));
        // Claimed chord + unrecognized first_down → nothing.
        assert_eq!(table.lookup(ab, Some(crate::scan::R_MID)), None);
        assert_eq!(table.lookup(ab, None), None);

        // Unclaimed chord still works the old way.
        let other = ChordKey::from_packed(0b0010, 0, false);
        table.insert(other, "and ".to_string());
        assert_eq!(table.lookup(other, None), Some("and "));
        assert_eq!(table.lookup(other, Some(crate::scan::R_MID)), Some("and "));
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
