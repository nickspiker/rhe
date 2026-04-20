use std::collections::HashMap;
use crate::chord_map::Phoneme;

/// Phoneme dictionary: maps a sequence of phonemes → English word.
/// Built from CMU dict. Homophones resolve to most frequent word.
pub struct PhonemeDictionary {
    entries: HashMap<Vec<Phoneme>, String>,
}

impl PhonemeDictionary {
    /// Build from CMU dict text and frequency data.
    pub fn build(cmudict_text: &str, freq_text: &str) -> Self {
        // Parse word frequencies
        let mut freq: HashMap<String, u64> = HashMap::new();
        for line in freq_text.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(word), Some(count)) = (parts.next(), parts.next()) {
                if let Ok(count) = count.parse::<u64>() {
                    freq.insert(word.to_lowercase(), count);
                }
            }
        }

        let mut entries: HashMap<Vec<Phoneme>, (String, u64)> = HashMap::new();

        for line in cmudict_text.lines() {
            if line.starts_with(";;;") { continue; }
            let mut parts = line.split_whitespace();
            let Some(raw_word) = parts.next() else { continue };

            // Strip variant markers like WORD(2)
            let word = raw_word.split('(').next().unwrap().to_lowercase();

            // Parse phonemes, stripping stress digits
            let phonemes: Vec<Phoneme> = parts
                .filter_map(|p| {
                    let clean = p.trim_end_matches(|c: char| c.is_ascii_digit());
                    Phoneme::from_arpabet(clean)
                })
                .collect();

            if phonemes.is_empty() { continue; }

            // Keep highest-frequency word for each phoneme sequence
            let word_freq = freq.get(&word).copied().unwrap_or(0);
            entries.entry(phonemes)
                .and_modify(|(existing_word, existing_freq)| {
                    if word_freq > *existing_freq {
                        *existing_word = word.clone();
                        *existing_freq = word_freq;
                    }
                })
                .or_insert((word, word_freq));
        }

        let dict: HashMap<Vec<Phoneme>, String> = entries
            .into_iter()
            .map(|(k, (word, _))| (k, word))
            .collect();

        eprintln!("rhe: phoneme dictionary: {} entries", dict.len());

        Self { entries: dict }
    }

    /// Look up a phoneme sequence → English word.
    pub fn lookup(&self, phonemes: &[Phoneme]) -> Option<&str> {
        self.entries.get(phonemes).map(|s| s.as_str())
    }

}

/// Parse CMU dict text and return word → phoneme vec mapping.
/// Useful for looking up specific words.
pub fn parse_cmudict(cmudict_text: &str) -> HashMap<String, Vec<Phoneme>> {
    let mut dict: HashMap<String, Vec<Phoneme>> = HashMap::new();
    for line in cmudict_text.lines() {
        if line.starts_with(";;;") { continue; }
        let mut parts = line.split_whitespace();
        let Some(raw_word) = parts.next() else { continue };
        let word = raw_word.split('(').next().unwrap().to_lowercase();

        let phonemes: Vec<Phoneme> = parts
            .filter_map(|p| {
                let clean = p.trim_end_matches(|c: char| c.is_ascii_digit());
                Phoneme::from_arpabet(clean)
            })
            .collect();

        if !phonemes.is_empty() {
            dict.entry(word).or_insert(phonemes);
        }
    }
    dict
}

#[cfg(test)]
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
        let dict = PhonemeDictionary::build(
            "CAT  K AE1 T\nTHE  DH AH0\n",
            "the 1000\ncat 500\n",
        );
        assert_eq!(dict.lookup(&[Phoneme::K, Phoneme::Ae, Phoneme::T]), Some("cat"));
        assert_eq!(dict.lookup(&[Phoneme::Dh, Phoneme::Ah]), Some("the"));
    }

    #[test]
    fn homophone_frequency() {
        // "to" and "too" have same pronunciation — higher freq wins
        let dict = PhonemeDictionary::build(
            "TO  T UW1\nTOO  T UW1\n",
            "to 5000\ntoo 100\n",
        );
        assert_eq!(dict.lookup(&[Phoneme::T, Phoneme::Uw]), Some("to"));
    }
}
