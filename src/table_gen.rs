use std::collections::HashMap;

/// Generates the syllable-to-chord mapping table from CMU dict + frequency data.
///
/// Strategy:
///   1. Parse CMU dict: word → phoneme sequence
///   2. Syllabify each word: split into (onset, vowel, coda) triples
///   3. Weight syllables by word frequency
///   4. Assign onset patterns to right-hand finger combos (by frequency)
///   5. Assign coda patterns to left-hand finger combos (by frequency)
///   6. For each (onset, coda) pair, distribute vowel variants across modes × ctrl
///   7. Output: ChordKey → syllable string

const VOWELS: &[&str] = &[
    "AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY",
    "IH", "IY", "OW", "OY", "UH", "UW",
];

/// Convert an ARPAbet phoneme to its IPA unicode representation.
fn arpabet_to_ipa(phoneme: &str) -> &'static str {
    match phoneme {
        // Consonants
        "T" => "t",
        "N" => "n",
        "S" => "s",
        "R" => "ɹ",
        "D" => "d",
        "L" => "l",
        "M" => "m",
        "K" => "k",
        "DH" => "ð",
        "W" => "w",
        "Z" => "z",
        "Y" => "j",
        "HH" => "h",
        "B" => "b",
        "P" => "p",
        "F" => "f",
        "V" => "v",
        "G" => "ɡ",
        "NG" => "ŋ",
        "SH" => "ʃ",
        "TH" => "θ",
        "JH" => "d͡ʒ",
        "CH" => "t͡ʃ",
        "ZH" => "ʒ",
        // Vowels
        "AH" => "ʌ",
        "IH" => "ɪ",
        "IY" => "iː",
        "EH" => "ɛ",
        "UW" => "uː",
        "AY" => "aɪ",
        "AE" => "æ",
        "AA" => "ɑ",
        "ER" => "ɝ",
        "OW" => "oʊ",
        "EY" => "eɪ",
        "AO" => "ɔ",
        "AW" => "aʊ",
        "UH" => "ʊ",
        "OY" => "ɔɪ",
        _ => "?",
    }
}

fn is_vowel(phoneme: &str) -> bool {
    VOWELS.contains(&phoneme)
}

/// Strip stress markers from ARPAbet phonemes (e.g., "AH0" → "AH").
fn strip_stress(phoneme: &str) -> &str {
    phoneme.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// A syllable: onset consonants, vowel, coda consonants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhoneSyllable {
    pub onset: Vec<String>,
    pub vowel: String,
    pub coda: Vec<String>,
}

impl PhoneSyllable {
    /// Render as IPA unicode string, e.g., "kæt" for "cat".
    pub fn to_ipa(&self) -> String {
        let mut s = String::new();
        for p in &self.onset {
            s.push_str(arpabet_to_ipa(p));
        }
        s.push_str(arpabet_to_ipa(&self.vowel));
        for p in &self.coda {
            s.push_str(arpabet_to_ipa(p));
        }
        s
    }

    /// Render as ARPAbet label like "K-AE-T" for "cat".
    pub fn to_label(&self) -> String {
        let mut parts = vec![];
        if !self.onset.is_empty() {
            parts.push(self.onset.join("+"));
        }
        parts.push(self.vowel.clone());
        if !self.coda.is_empty() {
            parts.push(self.coda.join("+"));
        }
        parts.join("-")
    }

    pub fn onset_key(&self) -> String {
        if self.onset.is_empty() {
            "(none)".to_string()
        } else {
            self.onset.join("+")
        }
    }

    pub fn coda_key(&self) -> String {
        if self.coda.is_empty() {
            "(none)".to_string()
        } else {
            self.coda.join("+")
        }
    }
}

/// Syllabify a phoneme sequence using maximum onset principle.
pub fn syllabify(phonemes: &[String]) -> Vec<PhoneSyllable> {
    let vowel_positions: Vec<usize> = phonemes
        .iter()
        .enumerate()
        .filter(|(_, p)| is_vowel(p))
        .map(|(i, _)| i)
        .collect();

    if vowel_positions.is_empty() {
        return vec![];
    }

    let mut syllables = vec![];

    for (si, &vi) in vowel_positions.iter().enumerate() {
        let onset_start = if si == 0 {
            0
        } else {
            let prev_vi = vowel_positions[si - 1];
            let between = &phonemes[prev_vi + 1..vi];
            if between.len() <= 1 {
                vi - between.len()
            } else {
                // Maximum onset: give all but first to this syllable's onset
                prev_vi + 2
            }
        };

        let onset: Vec<String> = phonemes[onset_start..vi].to_vec();
        let vowel = phonemes[vi].clone();

        let coda = if si == vowel_positions.len() - 1 {
            phonemes[vi + 1..].to_vec()
        } else {
            let next_vi = vowel_positions[si + 1];
            let between = &phonemes[vi + 1..next_vi];
            if between.len() <= 1 {
                vec![]
            } else {
                vec![phonemes[vi + 1].clone()]
            }
        };

        syllables.push(PhoneSyllable { onset, vowel, coda });
    }

    syllables
}

/// Parse a CMU dict line: "word P1 P2 P3" ��� (word, [P1, P2, P3])
fn parse_cmudict_line(line: &str) -> Option<(String, Vec<String>)> {
    if line.starts_with(";;;") {
        return None;
    }
    let mut parts = line.split_whitespace();
    let word = parts.next()?.to_lowercase();
    // Skip variant markers like (2)
    let word = if let Some(idx) = word.find('(') {
        word[..idx].to_string()
    } else {
        word
    };
    let phonemes: Vec<String> = parts.map(|p| strip_stress(p).to_string()).collect();
    if phonemes.is_empty() {
        return None;
    }
    Some((word, phonemes))
}

/// Parse a frequency file line: "word count" → (word, count)
fn parse_freq_line(line: &str) -> Option<(String, u64)> {
    let mut parts = line.split_whitespace();
    let word = parts.next()?.to_lowercase();
    let count: u64 = parts.next()?.parse().ok()?;
    Some((word, count))
}

/// A chord assignment: right-hand combo (onset), left-hand combo (coda), mode, ctrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChordAssignment {
    pub right: u8,   // 0-15 finger combo for onset
    pub left: u8,    // 0-15 finger combo for coda
    pub mode: u8,    // 0-3
    pub ctrl: bool,
}

impl ChordAssignment {
    pub fn to_chord_key_bits(&self) -> u16 {
        let right = self.right as u16 & 0xF;
        let left = (self.left as u16 & 0xF) << 4;
        let mode = (self.mode as u16) << 8;
        let ctrl = if self.ctrl { 1u16 << 10 } else { 0 };
        right | left | mode | ctrl
    }
}

/// Effort score for a finger combo (lower = easier).
fn combo_effort(bits: u8) -> f64 {
    let fingers: Vec<u8> = (0..4).filter(|&i| bits & (1 << i) != 0).collect();
    if fingers.is_empty() {
        return 0.0; // no fingers = no effort (empty onset/coda)
    }

    let single_effort = [1.0, 1.1, 1.5, 2.0]; // index, middle, ring, pinky
    if fingers.len() == 1 {
        return single_effort[fingers[0] as usize];
    }

    let mut effort: f64 = fingers.iter().map(|&f| single_effort[f as usize]).sum::<f64>() * 0.6;
    for i in 0..fingers.len() {
        for j in i + 1..fingers.len() {
            let gap = fingers[j] - fingers[i];
            if gap > 1 {
                effort += 0.3 * (gap - 1) as f64;
            }
        }
    }
    effort
}

/// Generate the syllable table from raw dictionary and frequency data.
///
/// Returns a map of ChordKey bits (u16) → syllable label string.
pub fn generate(cmudict_text: &str, freq_text: &str) -> HashMap<u16, String> {
    // Step 1: Parse frequency data
    let mut word_freq: HashMap<String, u64> = HashMap::new();
    for line in freq_text.lines() {
        if let Some((word, count)) = parse_freq_line(line) {
            word_freq.entry(word).or_insert(count);
        }
    }

    // Step 2: Parse CMU dict (first pronunciation only)
    let mut cmudict: HashMap<String, Vec<String>> = HashMap::new();
    for line in cmudict_text.lines() {
        if let Some((word, phonemes)) = parse_cmudict_line(line) {
            cmudict.entry(word).or_insert(phonemes);
        }
    }

    // Step 3: Count syllable frequency
    let mut syllable_freq: HashMap<PhoneSyllable, u64> = HashMap::new();
    let mut onset_freq: HashMap<String, u64> = HashMap::new();
    let mut coda_freq: HashMap<String, u64> = HashMap::new();

    for (word, count) in &word_freq {
        let Some(phonemes) = cmudict.get(word) else { continue };
        let syllables = syllabify(phonemes);
        for syl in &syllables {
            *syllable_freq.entry(syl.clone()).or_default() += count;
            *onset_freq.entry(syl.onset_key()).or_default() += count;
            *coda_freq.entry(syl.coda_key()).or_default() += count;
        }
    }

    // Step 4: Rank onsets and codas by frequency, assign to finger combos by effort
    let mut onset_ranked: Vec<(String, u64)> = onset_freq.into_iter().collect();
    onset_ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let mut coda_ranked: Vec<(String, u64)> = coda_freq.into_iter().collect();
    coda_ranked.sort_by(|a, b| b.1.cmp(&a.1));

    // Sort finger combos 0-15 by effort
    let mut combos_by_effort: Vec<(u8, f64)> = (0..16)
        .map(|bits| (bits, combo_effort(bits)))
        .collect();
    combos_by_effort.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Assign: most frequent onset → easiest combo
    let mut onset_to_combo: HashMap<String, u8> = HashMap::new();
    for (i, (onset, _)) in onset_ranked.iter().enumerate() {
        if i < combos_by_effort.len() {
            onset_to_combo.insert(onset.clone(), combos_by_effort[i].0);
        }
    }

    let mut coda_to_combo: HashMap<String, u8> = HashMap::new();
    for (i, (coda, _)) in coda_ranked.iter().enumerate() {
        if i < combos_by_effort.len() {
            coda_to_combo.insert(coda.clone(), combos_by_effort[i].0);
        }
    }

    // Step 5: Fixed vowel-to-slot mapping.
    // Same mode+ctrl always means the same vowel sound, everywhere.
    // Top 8 vowels by frequency get fixed slots:
    let vowel_to_slot: &[(&str, u8, bool)] = &[
        ("AH", 0, false),  // ʌ  — Mode 1, no ctrl
        ("IH", 1, false),  // ɪ  — Mode 2, no ctrl
        ("IY", 2, false),  // iː — Mode 3, no ctrl
        ("EH", 3, false),  // ɛ  — Mode 4, no ctrl
        ("UW", 0, true),   // uː — Mode 1, ctrl
        ("AY", 1, true),   // aɪ — Mode 2, ctrl
        ("AE", 2, true),   // æ  — Mode 3, ctrl
        ("AA", 3, true),   // ɑ  — Mode 4, ctrl
    ];

    // Build lookup: vowel name → (mode, ctrl)
    let mut vowel_slot_map: HashMap<&str, (u8, bool)> = HashMap::new();
    for &(vowel, mode, ctrl) in vowel_to_slot {
        vowel_slot_map.insert(vowel, (mode, ctrl));
    }

    // Remaining 7 vowels get overflow slots — assigned per onset+coda pair
    // to whichever mode+ctrl combo isn't used by the primary 8.
    let overflow_vowels: &[&str] = &["ER", "OW", "EY", "AO", "AW", "UH", "OY"];

    // All 8 slot keys for overflow scanning
    let all_slots: Vec<(u8, bool)> = vec![
        (0, false), (1, false), (2, false), (3, false),
        (0, true), (1, true), (2, true), (3, true),
    ];

    // Group syllables by (onset, coda)
    let mut pair_vowels: HashMap<(String, String), Vec<(PhoneSyllable, u64)>> = HashMap::new();
    for (syl, count) in &syllable_freq {
        let key = (syl.onset_key(), syl.coda_key());
        pair_vowels.entry(key).or_default().push((syl.clone(), *count));
    }

    // Step 6: Build the final table
    let mut table: HashMap<u16, String> = HashMap::new();

    for ((onset_key, coda_key), variants) in &pair_vowels {
        let Some(&right) = onset_to_combo.get(onset_key) else { continue };
        let Some(&left) = coda_to_combo.get(coda_key) else { continue };

        // Track which slots are used for this pair
        let mut used_slots: Vec<(u8, bool)> = Vec::new();

        // First pass: assign primary vowels to their fixed slots
        for (syl, _) in variants.iter() {
            if let Some(&(mode, ctrl)) = vowel_slot_map.get(syl.vowel.as_str()) {
                let assignment = ChordAssignment { right, left, mode, ctrl };
                let key = assignment.to_chord_key_bits();
                if !table.contains_key(&key) {
                    table.insert(key, syl.to_ipa());
                    used_slots.push((mode, ctrl));
                }
            }
        }

        // Second pass: assign overflow vowels to unused slots
        let mut free_slots: Vec<(u8, bool)> = all_slots
            .iter()
            .filter(|s| !used_slots.contains(s))
            .copied()
            .collect();

        // Sort overflow variants by frequency (most common first)
        let mut overflow: Vec<&(PhoneSyllable, u64)> = variants
            .iter()
            .filter(|(syl, _)| overflow_vowels.contains(&syl.vowel.as_str()))
            .collect();
        overflow.sort_by(|a, b| b.1.cmp(&a.1));

        for (syl, _) in overflow {
            if let Some((mode, ctrl)) = free_slots.pop() {
                let assignment = ChordAssignment { right, left, mode, ctrl };
                table.insert(assignment.to_chord_key_bits(), syl.to_ipa());
            }
            // else: no free slots, this rare vowel variant is dropped
        }
    }

    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_stress_works() {
        assert_eq!(strip_stress("AH0"), "AH");
        assert_eq!(strip_stress("IY1"), "IY");
        assert_eq!(strip_stress("T"), "T");
    }

    #[test]
    fn syllabify_cat() {
        let phonemes: Vec<String> = vec!["K", "AE", "T"]
            .into_iter().map(String::from).collect();
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 1);
        assert_eq!(syls[0].onset, vec!["K"]);
        assert_eq!(syls[0].vowel, "AE");
        assert_eq!(syls[0].coda, vec!["T"]);
    }

    #[test]
    fn syllabify_super() {
        let phonemes: Vec<String> = vec!["S", "UW", "P", "ER"]
            .into_iter().map(String::from).collect();
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 2);
        assert_eq!(syls[0].onset, vec!["S"]);
        assert_eq!(syls[0].vowel, "UW");
        assert_eq!(syls[0].coda, Vec::<String>::new());
        assert_eq!(syls[1].onset, vec!["P"]);
        assert_eq!(syls[1].vowel, "ER");
    }

    #[test]
    fn syllabify_the() {
        let phonemes: Vec<String> = vec!["DH", "AH"]
            .into_iter().map(String::from).collect();
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 1);
        assert_eq!(syls[0].onset, vec!["DH"]);
        assert_eq!(syls[0].vowel, "AH");
        assert!(syls[0].coda.is_empty());
    }

    #[test]
    fn syllabify_strong() {
        let phonemes: Vec<String> = vec!["S", "T", "R", "AO", "NG"]
            .into_iter().map(String::from).collect();
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 1);
        assert_eq!(syls[0].onset, vec!["S", "T", "R"]);
        assert_eq!(syls[0].vowel, "AO");
        assert_eq!(syls[0].coda, vec!["NG"]);
    }

    #[test]
    fn combo_effort_ordering() {
        // Single index should be easiest
        assert!(combo_effort(0b0001) < combo_effort(0b1000));
        // Adjacent pair easier than spread pair
        assert!(combo_effort(0b0011) < combo_effort(0b1001));
        // Empty = 0
        assert_eq!(combo_effort(0b0000), 0.0);
    }

    #[test]
    fn generate_small_table() {
        let cmudict = "cat K AE1 T\ncut K AH1 T\nthe DH AH0\n";
        let freq = "the 1000\ncat 500\ncut 200\n";

        let table = generate(cmudict, freq);

        // Should have entries for all three syllables (now in IPA)
        assert!(table.values().any(|v| v == "kæt"), "missing kæt: {:?}", table.values().collect::<Vec<_>>());
        assert!(table.values().any(|v| v == "kʌt"), "missing kʌt");
        assert!(table.values().any(|v| v == "ðʌ"), "missing ðʌ");

        // "cat" and "cut" share the same onset+coda (K, T) so they
        // should differ only by mode/ctrl
        let cat_key = table.iter().find(|(_, v)| *v == "kæt").unwrap().0;
        let cut_key = table.iter().find(|(_, v)| *v == "kʌt").unwrap().0;

        // Same right and left hand (bits 0-7) but different mode/ctrl (bits 8-10)
        assert_eq!(cat_key & 0xFF, cut_key & 0xFF);
        assert_ne!(cat_key, cut_key);

        // AH (ʌ) should always be Mode 1 (0), no ctrl
        let cut_mode = (cut_key >> 8) & 0x3;
        let cut_ctrl = (cut_key >> 10) & 1;
        assert_eq!(cut_mode, 0, "AH should be mode 0");
        assert_eq!(cut_ctrl, 0, "AH should be no ctrl");

        // AE (æ) should always be Mode 3 (2), ctrl
        let cat_mode = (cat_key >> 8) & 0x3;
        let cat_ctrl = (cat_key >> 10) & 1;
        assert_eq!(cat_mode, 2, "AE should be mode 2");
        assert_eq!(cat_ctrl, 1, "AE should be ctrl");
    }
}
