//! Te reo Māori brief table (auto-generated).
//!
//! First-cut stub: empty BRIEFS slice. `gen_briefs_mri` (Phase 4) will populate this from a frequency-ranked te reo word list (TeHikuMedia Hansard corpus). For now the engine falls through to phoneme-grapheme assembly for every word, which works for Māori because the orthography is 1:1 with the phoneme inventory.

use crate::layout::chord_key::{BriefTable, ChordKey};

/// Briefs entries: `(left_bits, right_bits, word)`. Empty until gen_briefs_mri runs.
pub const BRIEFS: &[(u8, u8, &str)] = &[];

pub fn load_briefs() -> BriefTable {
    let mut table = BriefTable::new();
    for &(left, right, word) in BRIEFS {
        // Briefs are typed without word held, so has_mod is always false.
        let key = ChordKey::from_packed(right, left, false);
        table.insert(key, format!("{} ", word));
    }
    table
}
