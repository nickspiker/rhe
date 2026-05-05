//! Te reo Māori layout: phoneme inventory and brief table.
//!
//! 20 phonemes total: 5 short vowels (a, e, i, o, u) + 5 long vowels (ā, ē, ī, ō, ū, via L-IDX-INNER modifier on the short chord) + 10 consonants (h, k, m, n, ng, p, r, t, w, wh). Selected at compile time via the `lang-mri` Cargo feature; see `src/layout/mod.rs` for the dispatch.
//!
//! First cut scope (per the v0.1.2 plan): phoneme table + brief table. Suffix system, ordered briefs, number forms, and three-path digit-word drill are skipped for now — Māori morphology and number constructions are different enough that porting them needs a native-speaker design pass.

pub mod briefs;
pub mod chords;
pub mod number_forms;
pub mod ordered_briefs;
pub mod suffixes;
