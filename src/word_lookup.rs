//! Word-to-phoneme lookup. Used by the tutor to translate target words into the chord sequence the user needs to type.
//!
//! English path (`lang-en`): preloaded from CMU dict text — `WordLookup::new(cmudict_text)`.
//!
//! Māori path (`lang-mri`): `WordLookup::new()` takes no input. te reo orthography is 1:1 with the phoneme inventory, so the lookup is implemented by parsing the word's graphemes on demand against the Māori `PhonemeTable`. No preloaded HashMap.

use crate::layout::chords::Phoneme;

#[cfg(feature = "lang-en")]
use crate::phoneme_dict;

/// Maps a word to its phoneme sequence.
pub struct WordLookup {
    /// English: preloaded from CMU dict. Māori: empty — `lookup()` parses graphemes on demand.
    #[allow(dead_code)]
    word_to_phonemes: std::collections::HashMap<String, Vec<Phoneme>>,
    /// Cache of grapheme-parse results for Māori, populated on demand. Saves re-parsing the same word repeatedly.
    #[cfg(feature = "lang-mri")]
    cache: std::cell::RefCell<std::collections::HashMap<String, Vec<Phoneme>>>,
}

impl WordLookup {
    /// English: build from CMU dict text.
    #[cfg(feature = "lang-en")]
    pub fn new(cmudict_text: &str) -> Self {
        Self {
            word_to_phonemes: phoneme_dict::parse_cmudict(cmudict_text),
        }
    }

    /// Māori: signature matches the English constructor for call-site uniformity, but the input is ignored — Māori has no CMU equivalent and `lookup()` parses graphemes on demand instead.
    #[cfg(feature = "lang-mri")]
    pub fn new(_unused_cmudict: &str) -> Self {
        Self {
            word_to_phonemes: std::collections::HashMap::new(),
            cache: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    /// Look up a word's phoneme sequence. English: HashMap lookup. Māori: grapheme-parse on demand (cached).
    #[cfg(feature = "lang-en")]
    pub fn lookup(&self, word: &str) -> Option<&[Phoneme]> {
        self.word_to_phonemes
            .get(&word.to_lowercase())
            .map(|v| v.as_slice())
    }

    /// Māori grapheme-parse path. Returns `None` if the word contains characters that don't map to any te reo phoneme. Result is owned (not borrowed) because parsing happens on demand.
    #[cfg(feature = "lang-mri")]
    pub fn lookup(&self, word: &str) -> Option<Vec<Phoneme>> {
        let lower = word.to_lowercase();
        if let Some(cached) = self.cache.borrow().get(&lower) {
            return Some(cached.clone());
        }
        let parsed = parse_mri_word(&lower)?;
        self.cache.borrow_mut().insert(lower, parsed.clone());
        Some(parsed)
    }
}

/// Parse a te reo Māori word into a phoneme sequence by longest-match grapheme scan. Two-character graphemes (`ng`, `wh`) and macron-marked vowels (`ā`, `ē`, `ī`, `ō`, `ū`) are recognised; unknown characters cause the parse to return `None`.
#[cfg(feature = "lang-mri")]
fn parse_mri_word(word: &str) -> Option<Vec<Phoneme>> {
    use Phoneme::*;
    let chars: Vec<char> = word.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        // Try 2-char graphemes first (digraphs).
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            let p = match two.as_str() {
                "ng" => Some(Ng),
                "wh" => Some(Wh),
                _ => None,
            };
            if let Some(p) = p {
                result.push(p);
                i += 2;
                continue;
            }
        }
        let one = chars[i];
        let p = match one {
            'h' => H,
            'k' => K,
            'm' => M,
            'n' => N,
            'p' => P,
            'r' => R,
            't' => T,
            'w' => W,
            'a' => A,
            'e' => E,
            'i' => I,
            'o' => O,
            'u' => U,
            'ā' => ALong,
            'ē' => ELong,
            'ī' => ILong,
            'ō' => OLong,
            'ū' => ULong,
            _ => return None,
        };
        result.push(p);
        i += 1;
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(all(test, feature = "lang-mri"))]
mod tests {
    use super::*;
    use crate::layout::chords::Phoneme::*;

    #[test]
    fn parses_short_vowels_and_basic_consonants() {
        assert_eq!(parse_mri_word("kia").unwrap(), vec![K, I, A]);
        assert_eq!(parse_mri_word("ora").unwrap(), vec![O, R, A]);
        assert_eq!(parse_mri_word("te").unwrap(), vec![T, E]);
    }

    #[test]
    fn parses_long_vowels() {
        assert_eq!(parse_mri_word("kāinga").unwrap(), vec![K, ALong, I, Ng, A]);
        assert_eq!(parse_mri_word("tūī").unwrap(), vec![T, ULong, ILong]);
    }

    #[test]
    fn parses_digraphs_ng_and_wh() {
        assert_eq!(parse_mri_word("whakapā").unwrap(), vec![Wh, A, K, A, P, ALong]);
        assert_eq!(parse_mri_word("ngā").unwrap(), vec![Ng, ALong]);
    }

    #[test]
    fn rejects_unknown_characters() {
        // Letters not in the Māori inventory (b, c, d, …) cause parse to fail.
        assert!(parse_mri_word("bad").is_none());
        assert!(parse_mri_word("hello").is_none()); // l isn't te reo
    }

    #[test]
    fn lookup_caches_repeated_words() {
        let lookup = WordLookup::new("");
        let first = lookup.lookup("kia").unwrap();
        let second = lookup.lookup("kia").unwrap();
        assert_eq!(first, second);
    }
}
