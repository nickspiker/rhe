//! Loads brief and suffix tables from compile-time const arrays.

use crate::briefs_data::BRIEFS;
use crate::suffixes_data::SUFFIXES;
use crate::chord_map::{BriefTable, ChordKey};

/// Build the brief table from the compile-time BRIEFS and SUFFIXES arrays.
///
/// Regular briefs store "word " (with trailing space).
/// Suffixes store "\x01suffix " — the \x01 prefix signals the interpreter
/// to backspace the previous trailing space before appending.
pub fn load_briefs() -> BriefTable {
    let mut table = BriefTable::new();

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

    eprintln!("rhe: loaded {} briefs + {} suffixes", BRIEFS.len(), SUFFIXES.len());
    table
}
