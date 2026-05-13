//! Chord-shape ergonomic ranking — single source of truth for "which finger combination is easiest".
//!
//! The 15 single-hand chord shapes (1-4 fingers from {Index, Middle, Ring, Pinky}) ranked from easiest to hardest by measured user effort. Used by every layout module that maps elements to chord shapes by priority: phoneme placement (frequent phoneme → easy chord), symbol-mode glyph placement, future brief layout work.
//!
//! Bit encoding: bit 0 = Index, bit 1 = Middle, bit 2 = Ring, bit 3 = Pinky. Bits 4+ (thumb / inner-index) are not part of this ranking — those keys carry distinct ergonomic costs and aren't interchangeable with the home-row four. Mirror-symmetric across hands — the same finger combo on the left or right hand has the same effort score.

/// Chord shapes from easiest (index 0) to hardest (index 14). Empirical ranking, hand-tuned during the original phoneme layout pass; see the assignments in `layout/en/chords.rs::Phoneme::chord_key` for the result.
pub const RANKING: [u8; 15] = [
    0b0001, //  0  I       (index alone)
    0b0100, //  1  R       (ring alone)
    0b1000, //  2  P       (pinky alone)
    0b0010, //  3  M       (middle alone)
    0b1111, //  4  all4    (full-hand slap)
    0b0110, //  5  M+R
    0b0011, //  6  I+M
    0b0111, //  7  I+M+R
    0b1001, //  8  I+P
    0b0101, //  9  I+R
    0b1100, // 10  R+P
    0b1110, // 11  M+R+P
    0b1010, // 12  M+P
    0b1101, // 13  I+R+P
    0b1011, // 14  I+M+P
];

/// Position of a chord shape in the effort ranking (0 = easiest, 14 = hardest). Returns `None` if `bits` isn't one of the 15 single-hand 1-4-finger combos.
pub fn rank(bits: u8) -> Option<usize> {
    RANKING.iter().position(|&b| b == bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 15 entries are unique — no duplicates would mean two slots map to the same effort, which breaks frequency-by-effort placement.
    #[test]
    fn all_unique() {
        let mut sorted = RANKING.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 15);
    }

    /// Every entry has between 1 and 4 fingers set (bits 0-3). No empty (zero-finger) chord, no bits outside the home-row four.
    #[test]
    fn entries_are_home_row_combos() {
        for &bits in &RANKING {
            let pop = bits.count_ones();
            assert!(pop >= 1 && pop <= 4, "bad popcount for {:#06b}", bits);
            assert_eq!(bits & !0b1111, 0, "non-home-row bits set in {:#06b}", bits);
        }
    }

    #[test]
    fn rank_lookup() {
        assert_eq!(rank(0b0001), Some(0)); // I = easiest
        assert_eq!(rank(0b1011), Some(14)); // I+M+P = hardest
        assert_eq!(rank(0b1111), Some(4)); // all4 = 5th-easiest
        assert_eq!(rank(0b0000), None); // empty
        assert_eq!(rank(0b10000), None); // bit 4 (thumb) not part of ranking
    }
}
