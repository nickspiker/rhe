use std::collections::HashMap;

/// Generates the syllable-to-chord mapping table from CMU dict + frequency data.
///
/// Layout:
///   - 15 consonants on non-⌘ combos (top 15 by frequency, covers 93%)
///   - ⌘ = vowel modifier only (4 modes × 2 = 8 vowel slots)
///   - Remaining 9 consonants assigned to overflow slots in the opaque table
///   - Same map for both hands (onset = right, coda = left)

const VOWELS: &[&str] = &[
    "AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY",
    "IH", "IY", "OW", "OY", "UH", "UW",
];

fn arpabet_to_ipa(phoneme: &str) -> &'static str {
    match phoneme {
        "T" => "t", "N" => "n", "S" => "s", "R" => "ɹ", "D" => "d",
        "L" => "l", "M" => "m", "K" => "k", "DH" => "ð", "W" => "w",
        "Z" => "z", "Y" => "j", "HH" => "h", "B" => "b", "P" => "p",
        "F" => "f", "V" => "v", "G" => "ɡ", "NG" => "ŋ", "SH" => "ʃ",
        "TH" => "θ", "JH" => "d͡ʒ", "CH" => "t͡ʃ", "ZH" => "ʒ",
        "AH" => "ʌ", "IH" => "ɪ", "IY" => "iː", "EH" => "ɛ",
        "UW" => "uː", "AY" => "aɪ", "AE" => "æ", "AA" => "ɑ",
        "ER" => "ɝ", "OW" => "oʊ", "EY" => "eɪ", "AO" => "ɔ",
        "AW" => "aʊ", "UH" => "ʊ", "OY" => "ɔɪ",
        _ => "?",
    }
}

fn is_vowel(phoneme: &str) -> bool {
    VOWELS.contains(&phoneme)
}

fn strip_stress(phoneme: &str) -> &str {
    phoneme.trim_end_matches(|c: char| c.is_ascii_digit())
}

// ─── Fixed consonant map ──────────────────────────────────────
//
// Top 15 consonants by frequency, assigned to combos by ease.
// No ⌘ involvement — ⌘ is purely for vowels.
//
// Rank  Combo         Consonant   Freq
//  1    I    (0b0001)  t          165M
//  2    M    (0b0010)  n          141M
//  3    I+M  (0b0011)  s          110M
//  4    R    (0b0100)  r           91M
//  5    I+R  (0b0101)  d           87M
//  6    M+R  (0b0110)  l           83M
//  7    I+M+R(0b0111)  m           73M
//  8    P    (0b1000)  k           71M
//  9    I+P  (0b1001)  ð (the)     58M
// 10    M+P  (0b1010)  w           57M
// 11    I+M+P(0b1011)  z           46M
// 12    R+P  (0b1100)  j/y (you)   45M
// 13    I+R+P(0b1101)  h           42M
// 14    M+R+P(0b1110)  b           40M
// 15    all  (0b1111)  p           36M

fn consonant_to_combo(phoneme: &str) -> Option<u8> {
    match phoneme {
        "T"  => Some(0b0001),
        "N"  => Some(0b0010),
        "S"  => Some(0b0011),
        "R"  => Some(0b0100),
        "D"  => Some(0b0101),
        "L"  => Some(0b0110),
        "M"  => Some(0b0111),
        "K"  => Some(0b1000),
        "DH" => Some(0b1001),
        "W"  => Some(0b1010),
        "Z"  => Some(0b1011),
        "Y"  => Some(0b1100),
        "HH" => Some(0b1101),
        "B"  => Some(0b1110),
        "P"  => Some(0b1111),
        // Remaining 9 (f,v,g,ŋ,ʃ,θ,dʒ,tʃ,ʒ) — no fixed combo,
        // assigned to overflow slots in the opaque table
        _ => None,
    }
}

// ─── Fixed vowel map ──────────────────────────────────────────
//
// ⌘ = stretch the vowel. Same mouth position, longer/more open.
//
// zil  (Mode 0, no ⌘) = AH (uh, "but")      — easiest
// lun  (Mode 3, no ⌘) = IH (ih, "sit")       — 2nd
// ter  (Mode 1, no ⌘) = EH (eh, "bed")       — 3rd
// stel (Mode 2, no ⌘) = AE (ah, "cat")       — 4th
// zila (Mode 0, ⌘)    = AA (ah, "father")
// luna (Mode 3, ⌘)    = IY (ee, "see")
// tera (Mode 1, ⌘)    = EY (ay, "say")
// stela(Mode 2, ⌘)    = AY (eye, "my")

fn vowel_to_slot(phoneme: &str) -> Option<(u8, bool)> {
    match phoneme {
        "AH" => Some((0, false)),  // zil
        "IH" => Some((3, false)),  // lun
        "EH" => Some((1, false)),  // ter
        "AE" => Some((2, false)),  // stel
        "AA" => Some((0, true)),   // zila
        "IY" => Some((3, true)),   // luna
        "EY" => Some((1, true)),   // tera
        "AY" => Some((2, true)),   // stela
        _ => None, // overflow vowels assigned dynamically
    }
}

// ─── Syllable types ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhoneSyllable {
    pub onset: Vec<String>,
    pub vowel: String,
    pub coda: Vec<String>,
}

impl PhoneSyllable {
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
}

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
            if between.len() <= 1 { vi - between.len() } else { prev_vi + 2 }
        };

        let onset: Vec<String> = phonemes[onset_start..vi].to_vec();
        let vowel = phonemes[vi].clone();

        let coda = if si == vowel_positions.len() - 1 {
            phonemes[vi + 1..].to_vec()
        } else {
            let next_vi = vowel_positions[si + 1];
            let between = &phonemes[vi + 1..next_vi];
            if between.len() <= 1 { vec![] } else { vec![phonemes[vi + 1].clone()] }
        };

        syllables.push(PhoneSyllable { onset, vowel, coda });
    }

    syllables
}

// ─── Chord assignment ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChordAssignment {
    pub right: u8,
    pub left: u8,
    pub mode: u8,
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

// ─── Table generation ─────────────────────────────────────────

pub fn generate(cmudict_text: &str, freq_text: &str) -> HashMap<u16, String> {
    // Parse frequency data
    let mut word_freq: HashMap<String, u64> = HashMap::new();
    for line in freq_text.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(w), Some(c)) = (parts.next(), parts.next()) {
            if let Ok(c) = c.parse::<u64>() {
                word_freq.entry(w.to_lowercase()).or_insert(c);
            }
        }
    }

    // Parse CMU dict
    let mut cmudict: HashMap<String, Vec<String>> = HashMap::new();
    for line in cmudict_text.lines() {
        if line.starts_with(";;;") { continue; }
        let mut parts = line.split_whitespace();
        let Some(word) = parts.next() else { continue };
        let word = word.to_lowercase();
        let word = if let Some(idx) = word.find('(') { word[..idx].to_string() } else { word };
        let phonemes: Vec<String> = parts.map(|p| strip_stress(p).to_string()).collect();
        if !phonemes.is_empty() {
            cmudict.entry(word).or_insert(phonemes);
        }
    }

    // Count syllable frequency
    let mut syllable_freq: HashMap<PhoneSyllable, u64> = HashMap::new();
    for (word, count) in &word_freq {
        let Some(phonemes) = cmudict.get(word) else { continue };
        for syl in &syllabify(phonemes) {
            *syllable_freq.entry(syl.clone()).or_default() += count;
        }
    }

    let mut table: HashMap<u16, String> = HashMap::new();

    // Group syllables by (right_combo, left_combo)
    // Only syllables where both onset and coda are single mapped consonants (or empty)
    let mut groups: HashMap<(u8, u8), Vec<(PhoneSyllable, u64)>> = HashMap::new();

    for (syl, count) in &syllable_freq {
        let right = if syl.onset.len() == 1 {
            match consonant_to_combo(&syl.onset[0]) {
                Some(c) => c,
                None => continue, // unmapped consonant — skip for now
            }
        } else if syl.onset.is_empty() {
            0
        } else {
            continue // cluster
        };

        let left = if syl.coda.len() == 1 {
            match consonant_to_combo(&syl.coda[0]) {
                Some(c) => c,
                None => continue,
            }
        } else if syl.coda.is_empty() {
            0
        } else {
            continue
        };

        if right == 0 && left == 0 { continue; }

        groups.entry((right, left)).or_default().push((syl.clone(), *count));
    }

    // All 8 vowel slots
    let all_slots: [(u8, bool); 8] = [
        (0, false), (1, false), (2, false), (3, false),
        (0, true), (1, true), (2, true), (3, true),
    ];

    for ((right, left), variants) in &groups {
        // Reachable modes: single-hand = 1 mode, both-hands = 4 modes
        let reachable: &[(u8, bool)] = if *left == 0 {
            &[(0, false), (0, true)] // right-only: zil ± ⌘
        } else if *right == 0 {
            &[(3, false), (3, true)] // left-only: lun ± ⌘
        } else {
            &all_slots
        };

        let is_single_hand = *right == 0 || *left == 0;
        let mut used: Vec<(u8, bool)> = Vec::new();

        // First pass: primary vowels to fixed slots (both-hands only)
        if !is_single_hand {
            for (syl, _) in variants.iter() {
                if let Some((mode, ctrl)) = vowel_to_slot(&syl.vowel) {
                    if !reachable.contains(&(mode, ctrl)) { continue; }
                    if used.contains(&(mode, ctrl)) { continue; }
                    let key = ChordAssignment { right: *right, left: *left, mode, ctrl }
                        .to_chord_key_bits();
                    if !table.contains_key(&key) {
                        table.insert(key, syl.to_ipa());
                        used.push((mode, ctrl));
                    }
                }
            }
        }

        // Second pass: remaining vowels by frequency to free slots
        let mut free: Vec<(u8, bool)> = reachable.iter()
            .filter(|s| !used.contains(s))
            .copied()
            .collect();

        let assigned: std::collections::HashSet<String> = table.iter()
            .filter(|(k, _)| {
                let k = **k;
                (k & 0xF) as u8 == *right && ((k >> 4) & 0xF) as u8 == *left
            })
            .map(|(_, v)| v.clone())
            .collect();

        let mut remaining: Vec<&(PhoneSyllable, u64)> = variants.iter()
            .filter(|(syl, _)| !assigned.contains(&syl.to_ipa()))
            .collect();
        remaining.sort_by(|a, b| b.1.cmp(&a.1));

        for (syl, _) in remaining {
            if let Some((mode, ctrl)) = free.pop() {
                let key = ChordAssignment { right: *right, left: *left, mode, ctrl }
                    .to_chord_key_bits();
                table.insert(key, syl.to_ipa());
            }
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
    fn consonant_map_top15() {
        assert_eq!(consonant_to_combo("T"), Some(0b0001));
        assert_eq!(consonant_to_combo("N"), Some(0b0010));
        assert_eq!(consonant_to_combo("D"), Some(0b0101)); // has its own combo now
        assert_eq!(consonant_to_combo("L"), Some(0b0110)); // has its own combo now
        assert_eq!(consonant_to_combo("P"), Some(0b1111));
        // Unmapped — returns None
        assert_eq!(consonant_to_combo("F"), None);
        assert_eq!(consonant_to_combo("G"), None);
        assert_eq!(consonant_to_combo("V"), None);
    }

    #[test]
    fn vowel_map_basics() {
        assert_eq!(vowel_to_slot("AH"), Some((0, false)));  // zil
        assert_eq!(vowel_to_slot("AA"), Some((0, true)));    // zila
        assert_eq!(vowel_to_slot("IH"), Some((3, false)));   // lun
        assert_eq!(vowel_to_slot("IY"), Some((3, true)));    // luna
        assert_eq!(vowel_to_slot("ER"), None);               // overflow
    }

    #[test]
    fn generate_small_table() {
        let cmudict = "cat K AE1 T\ncut K AH1 T\nthe DH AH0\ntell T EH1 L\n";
        let freq = "the 1000\ncat 500\ncut 200\ntell 400\n";

        let table = generate(cmudict, freq);

        assert!(table.values().any(|v| v == "kæt"), "missing kæt: {:?}", table.values().collect::<Vec<_>>());
        assert!(table.values().any(|v| v == "kʌt"), "missing kʌt");
        assert!(table.values().any(|v| v == "ðʌ"), "missing ðʌ");
        // "tell" should work now — T(no⌘) + L(no⌘), no conflict!
        assert!(table.values().any(|v| v == "tɛl"), "missing tɛl — tell should be mappable now");

        // AH = mode 0, no ctrl (zil)
        let cut_key = table.iter().find(|(_, v)| *v == "kʌt").unwrap().0;
        assert_eq!((cut_key >> 8) & 0x3, 0, "AH should be mode 0");
        assert_eq!((cut_key >> 10) & 1, 0, "AH should be no ⌘");

        // AE = mode 2, no ctrl (stel)
        let cat_key = table.iter().find(|(_, v)| *v == "kæt").unwrap().0;
        assert_eq!((cat_key >> 8) & 0x3, 2, "AE should be mode 2");
        assert_eq!((cat_key >> 10) & 1, 0, "AE should be no ⌘");
    }
}
