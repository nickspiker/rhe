//! Number-form transformations for Māori — stubbed.
//!
//! Māori cardinals (tahi, rua, toru, whā, rima, ono, whitu, waru, iwa, tekau) and ordinals (tuatahi, tuarua, …) follow a different construction than English's "twenty-second" / "twice" / "half" forms. v0.1.2 doesn't try to port the form-chord system; users type spelled numbers via the phoneme path. The `Form` enum and helper functions are kept as no-op stubs so consumer code (`tutor::drill`, `interpreter`) compiles without language-specific cfg gates around every call site.

pub use crate::layout::chord_key::ChordKey;

/// Forms a number-mode commit can be transformed into. Variants kept identical to `en/number_forms.rs::Form` so call sites that pattern-match on Form variants compile under either feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    SpelledCardinal,
    Ordinal,
    Multiplier,
    Group,
    Fraction,
    Prefix,
}

impl Form {
    /// Always returns 0 — no Māori number forms in v0.1.2, so no chord triggers any form.
    pub const fn chord_bits(self) -> u8 {
        0
    }

    /// Always returns `None` — no Māori chord triggers a form transform yet.
    pub fn from_chord(_key: ChordKey) -> Option<Self> {
        None
    }
}

/// Always returns `None` — no Māori form transforms in v0.1.2. Caller falls through to the regular emit path.
pub fn apply(_form: Form, _digits: &str) -> Option<String> {
    None
}

// Per-form generator stubs. All return `None` so the drill builder
// (`build_spelled_form_steps`) treats every word as having no spelled
// form, falling through to the phoneme path for words like "tahi" /
// "rua" — which is exactly what we want for v0.1.2.

pub fn spelled_cardinal(_digits: &str) -> Option<String> {
    None
}

pub fn ordinal(_digits: &str) -> Option<String> {
    None
}

pub fn multiplier(_digits: &str) -> Option<String> {
    None
}

pub fn group(_digits: &str) -> Option<String> {
    None
}

pub fn fraction(_digits: &str) -> Option<String> {
    None
}

pub fn prefix(_digits: &str) -> Option<String> {
    None
}
