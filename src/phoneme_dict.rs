//! Builds phoneme-sequence-to-word dictionary.
//!
//! For English (`lang-en`), parses the CMU pronouncing dictionary's ARPABET notation. For Māori (`lang-mri`), the dictionary is empty — te reo's 1:1 grapheme-to-phoneme orthography means the engine emits via the autospell (phoneme→grapheme) path and never needs a phoneme→word lookup.

use crate::layout::chords::Phoneme;
use std::collections::HashMap;

/// Phoneme dictionary: maps a sequence of phonemes → word. Under `lang-en` it's built from CMU dict + frequency data; under `lang-mri` it's permanently empty (every lookup misses, engine falls through to autospell).
pub struct PhonemeDictionary {
    entries: HashMap<Vec<Phoneme>, String>,
    /// All words sharing a phoneme sequence, sorted by descending frequency.
    /// Used by the homophone toggle to cycle through alternate spellings.
    homophones: HashMap<Vec<Phoneme>, Vec<String>>,
}

/// Walk CMU dict text, yielding `(lowercase_word, phonemes)` for every well-formed entry. Skips comment lines (`;;;`), strips variant markers (`WORD(2)` → `word`), drops stress digits from each phoneme, and filters entries that contain no recognised phonemes. Both consumers below build over this single iterator so the parsing rules live in exactly one place. ARPABET-specific; only compiled under `lang-en`.
#[cfg(feature = "lang-en")]
fn iter_entries(cmudict_text: &str) -> impl Iterator<Item = (String, Vec<Phoneme>)> + '_ {
    cmudict_text.lines().filter_map(|line| {
        if line.starts_with(";;;") {
            return None;
        }
        let mut parts = line.split_whitespace();
        let raw_word = parts.next()?;
        let word = raw_word.split('(').next().unwrap().to_lowercase();
        let phonemes: Vec<Phoneme> = parts
            .filter_map(|p| {
                let clean = p.trim_end_matches(|c: char| c.is_ascii_digit());
                Phoneme::from_arpabet(clean)
            })
            .collect();
        if phonemes.is_empty() {
            return None;
        }
        Some((word, phonemes))
    })
}

impl PhonemeDictionary {
    /// Empty dictionary — always-miss lookup. Used by `lang-mri` (no CMU equivalent for te reo) and as the v0.1.2 baseline before any per-language phoneme-word table is wired in. `#[allow(dead_code)]` because the lang-en build has no call site.
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
            homophones: HashMap::new(),
        }
    }

    /// Build from CMU dict text and frequency data. English-only — ARPABET-specific.
    #[cfg(feature = "lang-en")]
    pub fn build(cmudict_text: &str, freq_text: &str) -> Self {
        let mut freq: HashMap<String, u64> = HashMap::new();
        for line in freq_text.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(word), Some(count)) = (parts.next(), parts.next()) {
                if let Ok(count) = count.parse::<u64>() {
                    freq.insert(word.to_lowercase(), count);
                }
            }
        }

        // For each phoneme sequence, keep the highest-frequency word.
        let mut entries: HashMap<Vec<Phoneme>, (String, u64)> = HashMap::new();
        for (word, phonemes) in iter_entries(cmudict_text) {
            let word_freq = freq.get(&word).copied().unwrap_or(0);
            entries
                .entry(phonemes)
                .and_modify(|(existing_word, existing_freq)| {
                    if word_freq > *existing_freq {
                        *existing_word = word.clone();
                        *existing_freq = word_freq;
                    }
                })
                .or_insert_with(|| (word, word_freq));
        }

        // Build the homophones map: for each phoneme sequence, collect
        // ALL words sorted by descending frequency.
        let mut all_words: HashMap<Vec<Phoneme>, Vec<(String, u64)>> = HashMap::new();
        for (word, phonemes) in iter_entries(cmudict_text) {
            let word_freq = freq.get(&word).copied().unwrap_or(0);
            all_words
                .entry(phonemes)
                .or_default()
                .push((word, word_freq));
        }
        // Deduplicate (CMU dict can have variant pronunciations for the
        // same word) and sort by frequency descending.
        let mut homophones: HashMap<Vec<Phoneme>, Vec<String>> = HashMap::new();
        for (phonemes, mut words) in all_words {
            // Deduplicate by word, keeping highest freq per word
            let mut best: HashMap<String, u64> = HashMap::new();
            for (w, f) in &words {
                best.entry(w.clone())
                    .and_modify(|existing| {
                        if *f > *existing {
                            *existing = *f;
                        }
                    })
                    .or_insert(*f);
            }
            words = best.into_iter().map(|(w, f)| (w, f)).collect();
            words.sort_by(|a, b| b.1.cmp(&a.1));
            let sorted: Vec<String> = words.into_iter().map(|(w, _)| w).collect();
            if sorted.len() > 1 {
                homophones.insert(phonemes, sorted);
            }
        }

        let dict: HashMap<Vec<Phoneme>, String> =
            entries.into_iter().map(|(k, (word, _))| (k, word)).collect();
        Self {
            entries: dict,
            homophones,
        }
    }

    /// Look up a phoneme sequence → word. Always misses under `lang-mri`.
    pub fn lookup(&self, phonemes: &[Phoneme]) -> Option<&str> {
        self.entries.get(phonemes).map(|s| s.as_str())
    }

    /// Get all homophones for a phoneme sequence, sorted by descending frequency. Returns None if the sequence has no homophones (single spelling only). Used by the homophone toggle.
    pub fn homophones(&self, phonemes: &[Phoneme]) -> Option<&[String]> {
        self.homophones.get(phonemes).map(|v| v.as_slice())
    }
}

/// Parse CMU dict text and return word → phoneme vec mapping. Useful for looking up specific words. English-only.
#[cfg(feature = "lang-en")]
pub fn parse_cmudict(cmudict_text: &str) -> HashMap<String, Vec<Phoneme>> {
    let mut dict: HashMap<String, Vec<Phoneme>> = HashMap::new();
    for (word, phonemes) in iter_entries(cmudict_text) {
        dict.entry(word).or_insert(phonemes);
    }
    dict
}

#[cfg(all(test, feature = "lang-en"))]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_word() {
        let dict = parse_cmudict("CAT  K AE1 T\n");
        let phonemes = dict.get("cat").unwrap();
        assert_eq!(phonemes, &[Phoneme::K, Phoneme::Ae, Phoneme::T]);
    }

    #[test]
    fn dictionary_lookup() {
        let dict = PhonemeDictionary::build("CAT  K AE1 T\nTHE  DH AH0\n", "the 1000\ncat 500\n");
        assert_eq!(
            dict.lookup(&[Phoneme::K, Phoneme::Ae, Phoneme::T]),
            Some("cat")
        );
        assert_eq!(dict.lookup(&[Phoneme::Dh, Phoneme::Ah]), Some("the"));
    }

    #[test]
    fn homophone_frequency() {
        // "to" and "too" have same pronunciation — higher freq wins
        let dict = PhonemeDictionary::build("TO  T UW1\nTOO  T UW1\n", "to 5000\ntoo 100\n");
        assert_eq!(dict.lookup(&[Phoneme::T, Phoneme::Uw]), Some("to"));
    }

    #[test]
    fn homophones_list() {
        let dict = PhonemeDictionary::build(
            "TO  T UW1\nTOO  T UW1\nTWO  T UW1\nCAT  K AE1 T\n",
            "to 5000\ntoo 100\ntwo 3000\ncat 500\n",
        );
        let alts = dict.homophones(&[Phoneme::T, Phoneme::Uw]).unwrap();
        assert_eq!(alts, &["to", "two", "too"]); // sorted by freq desc
        // Single-spelling words have no homophones entry
        assert!(dict.homophones(&[Phoneme::K, Phoneme::Ae, Phoneme::T]).is_none());
    }
}
