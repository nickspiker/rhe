//! Ordered briefs for Māori — empty stub.
//!
//! v0.1.2 first cut: no ordered briefs. Te reo has fewer homophones than English and the chord space is over-provisioned, so the unordered brief table covers everything we need at this stage. Ordered-brief curation can come later once a native speaker has drilled the basics and identified pairs worth disambiguating by first-down lead.

/// Format kept identical to `en/ordered_briefs.rs::ORDERED_BRIEFS` so consumers (`briefs.rs`, `gen_briefs`) can read either language with one type signature: `(left_4bits, right_5bits, first_down_scancode, "word")`.
pub const ORDERED_BRIEFS: &[(u8, u8, u8, &str)] = &[];
