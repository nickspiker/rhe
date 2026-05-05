//! Suffix briefs for Māori — empty stub.
//!
//! Te reo morphology is mostly prefix-based (whaka- causative, kā- past, etc.) and uses reduplication; English's suffix system (-s plural, -ed past, -ing progressive) doesn't transfer. v0.1.2 ships with no Māori suffix/affix system; the engine still works, the user just types affixed words via the phoneme path. Designing a Māori prefix/reduplication chord system is deferred for a native-speaker pass.

/// Format kept identical to `en/suffixes.rs::SUFFIXES` so consumers (`briefs.rs`, `tutor::drill`) read either language with one type signature: `(left_4bits, "suffix_text")`.
pub const SUFFIXES: &[(u8, &str)] = &[];
