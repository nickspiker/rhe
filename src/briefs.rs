use crate::briefs_data::BRIEFS;
use crate::chord_map::{BriefTable, ChordKey};

/// Build the brief table from the compile-time BRIEFS array.
pub fn load_briefs() -> BriefTable {
    let mut table = BriefTable::new();

    for &(right, left, word) in BRIEFS {
        let has_mod = right & (1 << 4) != 0;
        let fingers = right & 0xF;
        let key = fingers as u16 | (left as u16) << 4 | if has_mod { 1u16 << 8 } else { 0 };
        table.insert(ChordKey(key), format!("{} ", word));
    }

    eprintln!("rhe: loaded {} briefs", BRIEFS.len());
    table
}
