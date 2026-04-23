//! Loads brief and suffix tables from compile-time const arrays.

use crate::briefs_data::BRIEFS;
use crate::chord_map::{BriefTable, ChordKey};
use crate::ordered_briefs_data::ORDERED_BRIEFS;
use crate::suffixes_data::SUFFIXES;

/// Build the brief table from the compile-time BRIEFS / ORDERED_BRIEFS /
/// SUFFIXES arrays.
///
/// Regular briefs store "word " (with trailing space).
/// Suffixes store "\x01suffix " — the \x01 prefix signals the interpreter
/// to backspace the previous trailing space before appending.
///
/// Ordered briefs load **first** so the chord-slot lockout is active
/// by the time unordered briefs try to insert. A BRIEFS entry aimed at
/// an ordered-claimed chord is silently dropped by `BriefTable::insert`.
pub fn load_briefs() -> BriefTable {
    let mut table = BriefTable::new();

    for &(left, right, first_down, word) in ORDERED_BRIEFS {
        let has_mod = right & (1 << 4) != 0;
        let fingers = right & 0xF;
        let key = ChordKey::from_packed(fingers, left, has_mod);
        table.insert_ordered(key, first_down, format!("{} ", word));
    }

    for &(left, right, word) in BRIEFS {
        let has_mod = right & (1 << 4) != 0;
        let fingers = right & 0xF;
        let key = ChordKey::from_packed(fingers, left, has_mod);
        table.insert(key, format!("{} ", word));
    }

    // Suffixes: left-hand only (right=0), marked with \x01 prefix
    for &(left, suffix) in SUFFIXES {
        let key = ChordKey::from_packed(0, left, false);
        table.insert(key, format!("\x01{} ", suffix));
    }

    eprintln!(
        "rhe: loaded {} briefs + {} ordered + {} suffixes",
        BRIEFS.len(),
        ORDERED_BRIEFS.len(),
        SUFFIXES.len()
    );
    table
}
