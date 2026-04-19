use std::collections::HashMap;

use crate::table_gen::{self, PhoneSyllable};

/// Maps any English word to the chord(s) needed to type it.
pub struct WordLookup {
    /// Single-chord briefs: word → chord key
    briefs: HashMap<String, u16>,
    /// CMU dict: word → phonemes
    cmudict: HashMap<String, Vec<String>>,
    /// IPA syllable → chord key (reverse of syllable table)
    ipa_to_chord: HashMap<String, u16>,
}

/// How to type a word.
#[derive(Debug, Clone)]
pub enum WordChords {
    /// Single chord, no space (brief)
    Brief {
        word: String,
        chord_key: u16,
        ipa: String,
    },
    /// Multiple chords with space held (syllable-by-syllable)
    MultiSyllable {
        word: String,
        syllables: Vec<SyllableChord>,
    },
    /// Word not in dictionary
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct SyllableChord {
    pub ipa: String,
    pub chord_key: u16,
    pub right_fingers: u8,
    pub left_fingers: u8,
    pub mode: u8,
    pub ctrl: bool,
}

impl WordLookup {
    pub fn new(
        briefs: &HashMap<u16, String>,
        syllable_table: &HashMap<u16, String>,
        cmudict_text: &str,
    ) -> Self {
        // Reverse brief map: word → chord key
        let mut brief_lookup: HashMap<String, u16> = HashMap::new();
        for (&key, word) in briefs {
            brief_lookup.entry(word.trim().to_lowercase()).or_insert(key);
        }

        // Parse CMU dict
        let mut cmudict: HashMap<String, Vec<String>> = HashMap::new();
        for line in cmudict_text.lines() {
            if line.starts_with(";;;") {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(word) = parts.next() else { continue };
            let word = word.to_lowercase();
            let word = if let Some(idx) = word.find('(') {
                word[..idx].to_string()
            } else {
                word
            };
            let phonemes: Vec<String> = parts
                .map(|p| p.trim_end_matches(|c: char| c.is_ascii_digit()).to_string())
                .collect();
            if !phonemes.is_empty() {
                cmudict.entry(word).or_insert(phonemes);
            }
        }

        // Reverse syllable table: IPA → chord key
        let mut ipa_to_chord: HashMap<String, u16> = HashMap::new();
        for (&key, ipa) in syllable_table {
            ipa_to_chord.entry(ipa.clone()).or_insert(key);
        }

        Self {
            briefs: brief_lookup,
            cmudict,
            ipa_to_chord,
        }
    }

    /// Look up how to type a word.
    pub fn lookup(&self, word: &str) -> WordChords {
        let w = word.to_lowercase();

        // Check briefs first
        if let Some(&chord_key) = self.briefs.get(&w) {
            let ipa = self.ipa_to_chord.iter()
                .find(|(_, v)| **v == chord_key)
                .map(|(k, _)| k.clone())
                .unwrap_or_default();
            return WordChords::Brief {
                word: w,
                chord_key,
                ipa,
            };
        }

        // Look up in CMU dict and syllabify
        let Some(phonemes) = self.cmudict.get(&w) else {
            return WordChords::Unknown(w);
        };

        let syllables = table_gen::syllabify(phonemes);
        if syllables.is_empty() {
            return WordChords::Unknown(w);
        }

        let mut chords: Vec<SyllableChord> = Vec::new();
        for syl in &syllables {
            let ipa = syl.to_ipa();
            if let Some(&chord_key) = self.ipa_to_chord.get(&ipa) {
                chords.push(SyllableChord {
                    ipa,
                    chord_key,
                    right_fingers: (chord_key & 0xF) as u8,
                    left_fingers: ((chord_key >> 4) & 0xF) as u8,
                    mode: ((chord_key >> 8) & 0x3) as u8,
                    ctrl: (chord_key >> 10) & 1 == 1,
                });
            } else {
                // Syllable not in table — word partially unmapped
                return WordChords::Unknown(w);
            }
        }

        if chords.len() == 1 {
            // Single syllable but no brief — still use word mode
            let c = &chords[0];
            WordChords::Brief {
                word: w,
                chord_key: c.chord_key,
                ipa: c.ipa.clone(),
            }
        } else {
            WordChords::MultiSyllable {
                word: w,
                syllables: chords,
            }
        }
    }

    /// Parse text into words, returning chord info for each.
    pub fn parse_text(&self, text: &str) -> Vec<WordChords> {
        text.split_whitespace()
            .map(|w| {
                // Strip punctuation for lookup but keep the word
                let clean: String = w.chars()
                    .filter(|c| c.is_alphabetic() || *c == '\'')
                    .collect();
                self.lookup(&clean)
            })
            .collect()
    }
}
