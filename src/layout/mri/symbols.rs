//! Symbol-mode chord ↔ glyph mappings for te reo Māori. First-cut stub returning `None` for everything — Māori symbol-mode layout is deferred to a future native-speaker design pass. Exists so the lang-mri build compiles against the same `crate::layout::symbols::{chord_to_glyph, glyph_to_chord}` re-export the engine and tutor consume.

pub fn chord_to_glyph(_key: crate::layout::chord_key::ChordKey) -> Option<&'static str> {
    None
}

pub fn glyph_to_chord(_c: char) -> Option<(u8, u8, bool)> {
    None
}
