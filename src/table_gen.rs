use std::collections::HashMap;

/// Generates the syllable-to-chord mapping table from CMU dict + frequency data.
///
/// The consonant and vowel mappings are fixed by design:
///   - Consonants: frequency × ease, with ⌘ = voiced/related partner
///   - Vowels: mode ease order, with ⌘ = "stretch the vowel"
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
// Returns (combo_bits, needs_cmd) for a single consonant.
// Combo bits: I=0, M=1, R=2, P=3 (bit positions).
//
// Rank  Combo      no ⌘         ⌘
//  1    I (0b0001) t            d
//  2    M (0b0010) n            ŋ (ng)
//  3    I+M(0b0011)s            z
//  4    R (0b0100) r            l
//  5    I+R(0b0101)m            [spare]
//  6    M+R(0b0110)k            g
//  7    I+M+R(0b0111)ð(the)     θ(think)
//  8    P (0b1000) w            j(you)
//  9    I+P(0b1001)h            [spare]
// 10    M+P(0b1010)b            p
// 11    I+M+P(0b1011)f          v
// 12    R+P(0b1100)ʃ(sh)        ʒ(measure)
// 13    I+R+P(0b1101)dʒ(judge)  tʃ(church)
// 14    M+R+P(0b1110)[spare]    [spare]
// 15    all(0b1111) [spare]     [spare]

fn consonant_to_combo(phoneme: &str) -> Option<(u8, bool)> {
    match phoneme {
        // no ⌘
        "T"  => Some((0b0001, false)),
        "N"  => Some((0b0010, false)),
        "S"  => Some((0b0011, false)),
        "R"  => Some((0b0100, false)),
        "M"  => Some((0b0101, false)),
        "K"  => Some((0b0110, false)),
        "DH" => Some((0b0111, false)),
        "W"  => Some((0b1000, false)),
        "HH" => Some((0b1001, false)),
        "B"  => Some((0b1010, false)),
        "F"  => Some((0b1011, false)),
        "SH" => Some((0b1100, false)),
        "JH" => Some((0b1101, false)),
        // ⌘ partners
        "D"  => Some((0b0001, true)),
        "NG" => Some((0b0010, true)),
        "Z"  => Some((0b0011, true)),
        "L"  => Some((0b0100, true)),
        "G"  => Some((0b0110, true)),
        "TH" => Some((0b0111, true)),
        "Y"  => Some((0b1000, true)),
        "P"  => Some((0b1010, true)),
        "V"  => Some((0b1011, true)),
        "ZH" => Some((0b1100, true)),
        "CH" => Some((0b1101, true)),
        _ => None,
    }
}

// ─── Fixed vowel map ──────────────────────────────────────────
//
// zil  (Mode 0, no ⌘) = AH (uh, "but")      — easiest
// lun  (Mode 3, no ⌘) = IH (ih, "sit")       — 2nd
// ter  (Mode 1, no ⌘) = EH (eh, "bed")       — 3rd
// stel (Mode 2, no ⌘) = AE (ah, "cat")       — 4th
// zila (Mode 0, ⌘)    = AA (ah, "father")     — ⌘ = stretch
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
        // Overflow vowels — no fixed slot, assigned dynamically
        _ => None,
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
        if self.onset.is_empty() { "(none)".into() } else { self.onset.join("+") }
    }

    pub fn coda_key(&self) -> String {
        if self.coda.is_empty() { "(none)".into() } else { self.coda.join("+") }
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

/// Generate the syllable table from raw dictionary and frequency data.
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

    // Group by (onset_combo, coda_combo, cmd_from_consonants)
    // For each syllable, determine the right/left combos and whether ⌘ is needed
    let mut table: HashMap<u16, String> = HashMap::new();

    // Group syllables by their finger pattern (right, left, consonant_cmd)
    struct SlotGroup {
        right: u8,
        left: u8,
        consonant_cmd: bool,
        syllables: Vec<(PhoneSyllable, u64)>,
    }

    let mut groups: HashMap<(u8, u8, bool), Vec<(PhoneSyllable, u64)>> = HashMap::new();

    for (syl, count) in &syllable_freq {
        // Look up onset consonant
        let (right, onset_cmd) = if syl.onset.len() == 1 {
            match consonant_to_combo(&syl.onset[0]) {
                Some(v) => v,
                None => continue, // unknown consonant
            }
        } else if syl.onset.is_empty() {
            (0u8, false) // no onset
        } else {
            continue; // consonant cluster — skip for now
        };

        // Look up coda consonant
        let (left, coda_cmd) = if syl.coda.len() == 1 {
            match consonant_to_combo(&syl.coda[0]) {
                Some(v) => v,
                None => continue,
            }
        } else if syl.coda.is_empty() {
            (0u8, false)
        } else {
            continue; // consonant cluster
        };

        // If either consonant needs ⌘, the whole chord uses ⌘
        let consonant_cmd = onset_cmd || coda_cmd;

        // Skip unreachable: both hands empty
        if right == 0 && left == 0 { continue; }

        groups.entry((right, left, consonant_cmd))
            .or_default()
            .push((syl.clone(), *count));
    }

    // For each group, assign vowels to mode slots
    let all_modes: [u8; 4] = [0, 1, 2, 3];

    for ((right, left, consonant_cmd), variants) in &groups {
        // Determine reachable modes
        let reachable_modes: &[u8] = if *left == 0 {
            &[0] // right-only = zil only
        } else if *right == 0 {
            &[3] // left-only = lun only
        } else {
            &all_modes
        };

        // The ⌘ state is already determined by consonants.
        // Within that ⌘ state, modes encode vowels.
        let ctrl = *consonant_cmd;

        // First pass: assign primary vowels to their fixed mode slots
        let mut used_modes: Vec<u8> = Vec::new();

        for (syl, _) in variants.iter() {
            if let Some((mode, vowel_cmd)) = vowel_to_slot(&syl.vowel) {
                // The vowel's ⌘ requirement must match the consonant's
                if vowel_cmd != ctrl { continue; }
                if !reachable_modes.contains(&mode) { continue; }
                if used_modes.contains(&mode) { continue; }

                let a = ChordAssignment { right: *right, left: *left, mode, ctrl };
                let key = a.to_chord_key_bits();
                if !table.contains_key(&key) {
                    table.insert(key, syl.to_ipa());
                    used_modes.push(mode);
                }
            }
        }

        // Second pass: assign remaining vowels to free mode slots by frequency
        let mut free_modes: Vec<u8> = reachable_modes.iter()
            .filter(|m| !used_modes.contains(m))
            .copied()
            .collect();

        let mut remaining: Vec<&(PhoneSyllable, u64)> = variants.iter()
            .filter(|(syl, _)| {
                let a = ChordAssignment { right: *right, left: *left, mode: 0, ctrl };
                // Check this syllable isn't already assigned
                !used_modes.iter().any(|&m| {
                    let a2 = ChordAssignment { right: *right, left: *left, mode: m, ctrl };
                    table.get(&a2.to_chord_key_bits()).map(|v| v == &syl.to_ipa()).unwrap_or(false)
                })
            })
            .collect();
        remaining.sort_by(|a, b| b.1.cmp(&a.1));

        for (syl, _) in remaining {
            if let Some(mode) = free_modes.pop() {
                let a = ChordAssignment { right: *right, left: *left, mode, ctrl };
                table.insert(a.to_chord_key_bits(), syl.to_ipa());
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
    fn consonant_map_basics() {
        // t is index, no cmd
        assert_eq!(consonant_to_combo("T"), Some((0b0001, false)));
        // d is index, with cmd (voiced partner of t)
        assert_eq!(consonant_to_combo("D"), Some((0b0001, true)));
        // s/z pair
        assert_eq!(consonant_to_combo("S"), Some((0b0011, false)));
        assert_eq!(consonant_to_combo("Z"), Some((0b0011, true)));
        // ð/θ pair
        assert_eq!(consonant_to_combo("DH"), Some((0b0111, false)));
        assert_eq!(consonant_to_combo("TH"), Some((0b0111, true)));
    }

    #[test]
    fn vowel_map_basics() {
        // zil = AH, no cmd
        assert_eq!(vowel_to_slot("AH"), Some((0, false)));
        // zila = AA, cmd
        assert_eq!(vowel_to_slot("AA"), Some((0, true)));
        // lun = IH, no cmd
        assert_eq!(vowel_to_slot("IH"), Some((3, false)));
        // luna = IY, cmd
        assert_eq!(vowel_to_slot("IY"), Some((3, true)));
    }

    #[test]
    fn generate_small_table() {
        let cmudict = "cat K AE1 T\ncut K AH1 T\nthe DH AH0\n";
        let freq = "the 1000\ncat 500\ncut 200\n";

        let table = generate(cmudict, freq);

        // K = 0b0110 no cmd, T = 0b0001 no cmd
        // "cut" = K+AH+T, AH = zil (mode 0, no cmd) → both consonants no cmd, vowel no cmd ✓
        assert!(table.values().any(|v| v == "kʌt"), "missing kʌt: {:?}", table.values().collect::<Vec<_>>());

        // "cat" = K+AE+T, AE = stel (mode 2, no cmd) → consonants no cmd, vowel no cmd ✓
        assert!(table.values().any(|v| v == "kæt"), "missing kæt: {:?}", table.values().collect::<Vec<_>>());

        // "the" = DH+AH, DH = 0b0111 no cmd, AH = zil (mode 0, no cmd)
        assert!(table.values().any(|v| v == "ðʌ"), "missing ðʌ: {:?}", table.values().collect::<Vec<_>>());

        // "cut" and "cat" should share right+left (same K and T combos)
        let cat_key = table.iter().find(|(_, v)| *v == "kæt").unwrap().0;
        let cut_key = table.iter().find(|(_, v)| *v == "kʌt").unwrap().0;
        assert_eq!(cat_key & 0xFF, cut_key & 0xFF);
        assert_ne!(cat_key, cut_key);

        // AH = mode 0, no ctrl
        let cut_mode = (cut_key >> 8) & 0x3;
        let cut_ctrl = (cut_key >> 10) & 1;
        assert_eq!(cut_mode, 0, "AH should be mode 0 (zil)");
        assert_eq!(cut_ctrl, 0, "AH should be no cmd");

        // AE = mode 2, no ctrl
        let cat_mode = (cat_key >> 8) & 0x3;
        let cat_ctrl = (cat_key >> 10) & 1;
        assert_eq!(cat_mode, 2, "AE should be mode 2 (stel)");
        assert_eq!(cat_ctrl, 0, "AE should be no cmd");
    }
}
