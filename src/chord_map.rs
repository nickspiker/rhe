use crate::chord_state::{Chord, FingerChord, Mode, ThumbState};

/// A chord key: the unique index into the syllable table.
///
/// Encoding: 11 bits total
///   bits 0-3:  right hand finger chord (4 bits, 0-15)
///   bits 4-7:  left hand finger chord (4 bits, 0-15)
///   bits 8-9:  mode (2 bits, 0-3)
///   bit 10:    ctrl state (1 bit)
///
/// Total: 2^11 = 2,048 possible chord keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChordKey(pub u16);

impl ChordKey {
    pub const MAX: u16 = 2048;

    pub fn from_chord(chord: &Chord) -> Self {
        let right = chord.right.0 as u16 & 0xF;
        let left = (chord.left.0 as u16 & 0xF) << 4;
        let mode = (mode_to_bits(chord.mode) as u16) << 8;
        let ctrl = if chord.thumbs.has_ctrl() { 1u16 << 10 } else { 0 };
        Self(right | left | mode | ctrl)
    }

    pub fn right_fingers(self) -> FingerChord {
        FingerChord((self.0 & 0xF) as u8)
    }

    pub fn left_fingers(self) -> FingerChord {
        FingerChord(((self.0 >> 4) & 0xF) as u8)
    }

    pub fn mode(self) -> Mode {
        mode_from_bits(((self.0 >> 8) & 0x3) as u8)
    }

    pub fn has_ctrl(self) -> bool {
        self.0 & (1 << 10) != 0
    }
}

fn mode_to_bits(mode: Mode) -> u8 {
    match mode {
        Mode::Mode1 => 0,
        Mode::Mode2 => 1,
        Mode::Mode3 => 2,
        Mode::Mode4 => 3,
    }
}

fn mode_from_bits(bits: u8) -> Mode {
    match bits & 0x3 {
        0 => Mode::Mode1,
        1 => Mode::Mode2,
        2 => Mode::Mode3,
        _ => Mode::Mode4,
    }
}

/// The syllable table: maps ChordKey → syllable string.
///
/// This is the core lookup. Generated at build time from CMU dict +
/// frequency data. At runtime it's a flat array indexed by ChordKey.
///
/// Entries are `Option<&str>` — None means the chord is unassigned.
pub struct SyllableTable {
    entries: Vec<Option<String>>,
}

impl SyllableTable {
    pub fn new() -> Self {
        Self {
            entries: vec![None; ChordKey::MAX as usize],
        }
    }

    pub fn insert(&mut self, key: ChordKey, syllable: String) {
        self.entries[key.0 as usize] = Some(syllable);
    }

    pub fn lookup(&self, key: ChordKey) -> Option<&str> {
        self.entries[key.0 as usize].as_deref()
    }

    pub fn assigned_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_key_roundtrip() {
        let chord = Chord {
            mode: Mode::Mode2,
            right: FingerChord(0b0101),  // index+ring
            left: FingerChord(0b0011),   // index+middle
            thumbs: ThumbState(0b01),    // ctrl only
        };

        let key = ChordKey::from_chord(&chord);
        assert_eq!(key.right_fingers(), FingerChord(0b0101));
        assert_eq!(key.left_fingers(), FingerChord(0b0011));
        assert_eq!(key.mode(), Mode::Mode2);
        assert!(key.has_ctrl());
    }

    #[test]
    fn chord_key_no_ctrl() {
        let chord = Chord {
            mode: Mode::Mode1,
            right: FingerChord(0b1010),
            left: FingerChord(0b0000),
            thumbs: ThumbState::NONE,
        };

        let key = ChordKey::from_chord(&chord);
        assert!(!key.has_ctrl());
        assert_eq!(key.mode(), Mode::Mode1);
    }

    #[test]
    fn chord_key_max_value() {
        let chord = Chord {
            mode: Mode::Mode4,
            right: FingerChord(0b1111),
            left: FingerChord(0b1111),
            thumbs: ThumbState(0b01), // ctrl
        };

        let key = ChordKey::from_chord(&chord);
        assert!(key.0 < ChordKey::MAX);
    }

    #[test]
    fn syllable_table_basic() {
        let mut table = SyllableTable::new();
        let key = ChordKey(42);

        assert!(table.lookup(key).is_none());

        table.insert(key, "cat".to_string());
        assert_eq!(table.lookup(key), Some("cat"));
    }

    #[test]
    fn all_keys_unique() {
        // Verify no two different chord inputs produce the same key
        use std::collections::HashSet;
        let mut seen = HashSet::new();

        for right in 0..16u8 {
            for left in 0..16u8 {
                for mode in 0..4u8 {
                    for ctrl in [false, true] {
                        let chord = Chord {
                            mode: mode_from_bits(mode),
                            right: FingerChord(right),
                            left: FingerChord(left),
                            thumbs: if ctrl { ThumbState(0b01) } else { ThumbState::NONE },
                        };
                        let key = ChordKey::from_chord(&chord);
                        assert!(seen.insert(key.0), "Duplicate key {}", key.0);
                    }
                }
            }
        }

        assert_eq!(seen.len(), 2048);
    }
}
