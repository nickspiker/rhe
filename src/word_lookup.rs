//! Word-to-phoneme lookup from CMU dict.

use crate::layout::chords::Phoneme;
use crate::phoneme_dict;

/// Maps an English word to its phoneme sequence (for the tutor).
pub struct WordLookup {
    word_to_phonemes: std::collections::HashMap<String, Vec<Phoneme>>,
}

impl WordLookup {
    pub fn new(cmudict_text: &str) -> Self {
        Self {
            word_to_phonemes: phoneme_dict::parse_cmudict(cmudict_text),
        }
    }

    /// Look up a word's phoneme sequence.
    pub fn lookup(&self, word: &str) -> Option<&[Phoneme]> {
        self.word_to_phonemes
            .get(&word.to_lowercase())
            .map(|v| v.as_slice())
    }
}
