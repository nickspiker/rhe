//! Symbol-mode chord ↔ glyph mappings for English.
//!
//! Symbol mode is entered by the `+word +mod -word +chord` gesture (see `state_machine.rs::feed`'s pending_zero branch). Once entered, a single chord fires and the interpreter looks the chord shape up here to emit the glyph, then reverts to Mode::Normal — one symbol per gesture, the user re-enters for each subsequent symbol.
//!
//! Layout structure: each hand gets an array of length 15, indexed by `crate::layout::effort::RANKING` position. Index 0 is the easiest chord (I = index alone), index 14 is the hardest (I+M+P). Glyphs are placed in frequency order so the most-used glyph lands on the easiest chord. Two lookup directions (`chord_to_glyph` and `glyph_to_chord`) both read from the same two arrays — no per-shape duplication.
//!
//! Right hand (Greek lowercase consonants): currently follows English-phoneme analog placement (κ on K-chord, π on P-chord, etc.) so the muscle memory from phoneme typing transfers. The Dh slot has no Greek analog — left empty for a future addition. Once we have a Greek-letter-frequency-driven corpus pass (arxiv abstracts), this array gets re-sorted by Greek frequency × effort rank.
//!
//! Left hand (units, currency, ASCII randos, Greek vowels): sorted by frequency × effort. Frequencies for non-ASCII glyphs come from `data/symbol_frequency.csv`; ASCII randos are estimated by typical web/programming usage. Phoneme-sound mnemonics for vowels (α=Ah etc.) intentionally dropped — frequency wins.
//!
//! Mod-bearing chord shapes are unreachable from the current SymbolMode entry path — `accum` is zeroed and seeded with the trigger finger only, so thumb never lands in a fired chord. Inner-index keys are number-mode-only by convention. Both reasons keep the layout strictly to the 8 home-row finger keys.

/// Right-hand glyphs indexed by effort rank. `None` = chord shape is intentionally unmapped (currently the I+M+R slot — reserved for a future addition). Accessed only through `chord_to_glyph` / `glyph_to_chord` below.
///
/// Frequency rankings come from `data/symbol_frequency_arxiv.csv` (900 arXiv math/physics abstracts). The phoneme-analog mnemonic from earlier passes (Greek-on-English-phoneme-shape) was abandoned — it put τ (8th-most-frequent Greek consonant) on the easiest chord and λ / β / μ / π (the actual heavy hitters) on the hardest. arXiv frequencies reflect real Greek-letter use in math/physics writing, which is where symbol mode earns its slot.
const RIGHT_BY_EFFORT: [Option<&str>; 15] = [
    Some("λ"), //  0  I        — lambda    (arxiv rank 1, count 77)
    Some("β"), //  1  R        — beta      (76)
    Some("μ"), //  2  P        — mu        (74)
    Some("ρ"), //  3  M        — rho       (50)
    Some("π"), //  4  all4     — pi        (45)
    Some("σ"), //  5  M+R      — sigma     (26)
    Some("ω"), //  6  I+M      — omega     (26, tied with σ)
    None,      //  7  I+M+R    — reserved (no current Greek lowercase candidate)
    Some("τ"), //  8  I+P      — tau       (18)
    Some("δ"), //  9  I+R      — delta     (16)
    Some("η"), // 10  R+P      — eta       (15)
    Some("ψ"), // 11  M+R+P    — psi       (13)
    Some("κ"), // 12  M+P      — kappa     (12)
    Some("φ"), // 13  I+R+P    — phi       (11)
    Some("ζ"), // 14  I+M+P    — zeta      (10)
];

/// Left-hand glyphs indexed by effort rank. Frequency × effort: most-used glyph on the easiest chord.
const LEFT_BY_EFFORT: [Option<&str>; 15] = [
    Some("°"), //  0  I        — degree (scan rank 2 across runs)
    Some("£"), //  1  R        — pound
    Some("@"), //  2  P        — at-sign
    Some("α"), //  3  M        — alpha (highest-freq Greek vowel)
    Some("#"), //  4  all4     — hash
    Some("$"), //  5  M+R      — dollar
    Some("ο"), //  6  I+M      — omicron
    Some("ι"), //  7  I+M+R    — iota
    Some("~"), //  8  I+P      — tilde
    Some("€"), //  9  I+R      — euro
    Some(";"), // 10  R+P      — semicolon
    Some("ε"), // 11  M+R+P    — epsilon
    Some("υ"), // 12  M+P      — upsilon
    Some("|"), // 13  I+R+P    — pipe
    Some("`"), // 14  I+M+P    — backtick
];

/// Symbol-mode chord → glyph. Returns `None` for any chord shape not in the table; the interpreter treats `None` as a silent no-op. Routes via `effort::rank` so adding a new glyph is a one-line change to `RIGHT_BY_EFFORT` / `LEFT_BY_EFFORT`.
pub fn chord_to_glyph(key: crate::layout::chord_key::ChordKey) -> Option<&'static str> {
    if key.has_mod() {
        return None;
    }
    let r = key.right_bits();
    let l = key.left_bits();
    if r != 0 && l == 0 {
        return crate::layout::effort::rank(r).and_then(|i| RIGHT_BY_EFFORT[i]);
    }
    if l != 0 && r == 0 {
        return crate::layout::effort::rank(l).and_then(|i| LEFT_BY_EFFORT[i]);
    }
    None
}

/// Reverse lookup: glyph char → chord shape `(right_bits, left_bits, has_mod)`. Iterates the per-hand arrays — no separate static reverse-table to keep in sync.
pub fn glyph_to_chord(c: char) -> Option<(u8, u8, bool)> {
    let mut buf = [0u8; 4];
    let s: &str = c.encode_utf8(&mut buf);
    for (i, slot) in RIGHT_BY_EFFORT.iter().enumerate() {
        if let Some(g) = slot {
            if *g == s {
                return Some((crate::layout::effort::RANKING[i], 0, false));
            }
        }
    }
    for (i, slot) in LEFT_BY_EFFORT.iter().enumerate() {
        if let Some(g) = slot {
            if *g == s {
                return Some((0, crate::layout::effort::RANKING[i], false));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both arrays' Some entries match exactly one char (no multi-char glyphs accidentally present), and have no duplicate glyphs across either array. A duplicate would make `glyph_to_chord` non-deterministic.
    #[test]
    fn arrays_well_formed() {
        let mut all_glyphs: Vec<&str> = Vec::new();
        for slot in RIGHT_BY_EFFORT.iter().chain(LEFT_BY_EFFORT.iter()) {
            if let Some(g) = slot {
                assert_eq!(
                    g.chars().count(),
                    1,
                    "multi-char glyph {} not supported by glyph_to_chord lookup",
                    g
                );
                all_glyphs.push(g);
            }
        }
        let unique: std::collections::HashSet<&&str> = all_glyphs.iter().collect();
        assert_eq!(
            unique.len(),
            all_glyphs.len(),
            "duplicate glyph across hands"
        );
    }

    /// Round-trip: every Some-slot glyph in either array maps back to its own chord shape. Catches drift if the arrays are ever desynchronized from `effort::RANKING`.
    #[test]
    fn forward_reverse_roundtrip() {
        for (i, slot) in RIGHT_BY_EFFORT.iter().enumerate() {
            if let Some(g) = slot {
                let c = g.chars().next().unwrap();
                let (r, l, m) = glyph_to_chord(c).expect("missing reverse lookup");
                assert_eq!((r, l, m), (crate::layout::effort::RANKING[i], 0, false));
                let key = crate::layout::chord_key::ChordKey::from_packed(r, l, m);
                assert_eq!(chord_to_glyph(key), Some(*g));
            }
        }
        for (i, slot) in LEFT_BY_EFFORT.iter().enumerate() {
            if let Some(g) = slot {
                let c = g.chars().next().unwrap();
                let (r, l, m) = glyph_to_chord(c).expect("missing reverse lookup");
                assert_eq!((r, l, m), (0, crate::layout::effort::RANKING[i], false));
                let key = crate::layout::chord_key::ChordKey::from_packed(r, l, m);
                assert_eq!(chord_to_glyph(key), Some(*g));
            }
        }
    }
}
