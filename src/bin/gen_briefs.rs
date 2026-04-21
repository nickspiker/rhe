//! Generates `src/briefs_data.rs` — optimized brief (chord→word) assignments.
//!
//! Run with: `cargo run --bin gen_briefs`

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

// ── Consonant mappings (right hand, 5 bits: bit4=mod, bits3-0=PRMI) ──────────

fn cmu_consonant_to_right(ph: &str) -> Option<u8> {
    match ph {
        // Without mod (15 consonants)
        "T"  => Some(0b00001),
        "S"  => Some(0b00010),
        "K"  => Some(0b00100),
        "P"  => Some(0b01000),
        "N"  => Some(0b00011),
        "R"  => Some(0b00101),
        "L"  => Some(0b00110),
        "HH" => Some(0b00111),
        "F"  => Some(0b01001),
        "W"  => Some(0b01010),
        "TH" => Some(0b01100),
        "SH" => Some(0b01011),
        "CH" => Some(0b01101),
        "NG" => Some(0b01110),
        "Y"  => Some(0b01111),
        // With mod (9 voiced consonants)
        "D"  => Some(0b10001),
        "Z"  => Some(0b10010),
        "G"  => Some(0b10100),
        "B"  => Some(0b11000),
        "M"  => Some(0b10011),
        "DH" => Some(0b10101),
        "V"  => Some(0b11001),
        "ZH" => Some(0b11011),
        "JH" => Some(0b11101),
        _ => None,
    }
}

// ── Vowel mappings (left hand, 4 bits: PRMI) ────────────────────────────────

fn cmu_vowel_to_left(ph: &str) -> Option<u8> {
    match ph {
        "AH" => Some(0b0001), // ʌ  Ah
        "IH" => Some(0b0010), // ɪ  Ih
        "EH" => Some(0b0100), // ɛ  Eh
        "AE" => Some(0b1000), // æ  Ae
        "IY" => Some(0b0011), // iː Iy
        "AA" => Some(0b0101), // ɑ  Aa
        "EY" => Some(0b0110), // eɪ Ey
        "ER" => Some(0b0111), // ɝ  Er
        "AY" => Some(0b1001), // aɪ Ay
        "OW" => Some(0b1010), // oʊ Ow
        "AO" => Some(0b1100), // ɔ  Ao
        "UW" => Some(0b1011), // uː Uw
        "AW" => Some(0b1101), // aʊ Aw
        "UH" => Some(0b1110), // ʊ  Uh
        "OY" => Some(0b1111), // ɔɪ Oy
        _ => None,
    }
}

fn is_vowel_phoneme(ph: &str) -> bool {
    matches!(
        ph,
        "AH" | "IH" | "EH" | "AE" | "IY" | "AA" | "EY" | "ER" | "AY" | "OW" | "AO" | "UW"
            | "AW" | "UH" | "OY"
    )
}

fn is_consonant_phoneme(ph: &str) -> bool {
    cmu_consonant_to_right(ph).is_some()
}

/// Strip stress digits from CMU phoneme (e.g. "AE1" → "AE")
fn strip_stress(ph: &str) -> &str {
    let bytes = ph.as_bytes();
    if !bytes.is_empty() && bytes[bytes.len() - 1].is_ascii_digit() {
        &ph[..ph.len() - 1]
    } else {
        ph
    }
}

// ── Ergonomic scoring ────────────────────────────────────────────────────────

/// Count bits set.
fn popcount(v: u8) -> u32 {
    v.count_ones()
}

/// Finger effort for a 4-bit pattern (bits 0-3 = index, middle, ring, pinky).
/// Lower = easier.
fn finger_effort(bits: u8) -> u32 {
    let n = popcount(bits);
    if n == 0 {
        return 0;
    }
    // Base cost: number of fingers
    let finger_cost = match n {
        1 => 1,
        2 => 3,
        3 => 6,
        4 => 10,
        _ => 15,
    };
    // Adjacency bonus: non-adjacent pairs cost more
    let gap_penalty = if n >= 2 {
        let mut gaps = 0u32;
        let mut prev = None;
        for b in 0..4u8 {
            if bits & (1 << b) != 0 {
                if let Some(p) = prev {
                    let dist: u8 = b - p;
                    if dist > 1 {
                        gaps += (dist - 1) as u32;
                    }
                }
                prev = Some(b);
            }
        }
        gaps
    } else {
        0
    };
    // Finger weight: pinky (bit3) = +2, ring (bit2) = +1
    let weight = if bits & 0b1000 != 0 { 2 } else { 0 }
        + if bits & 0b0100 != 0 { 1 } else { 0 };

    finger_cost + gap_penalty + weight
}

/// Measured finger combo effort (from bench data, averaged across hands).
/// Lower = faster. Returns milliseconds as effort proxy.
fn finger_combo_effort(bits: u8) -> u32 {
    match bits {
        0b0000 => 0,
        0b0001 => 668,  // index
        0b0100 => 703,  // ring
        0b1000 => 721,  // pinky
        0b0010 => 739,  // middle
        0b1111 => 784,  // all four
        0b0110 => 754,  // middle+ring
        0b0011 => 843,  // index+middle
        0b0111 => 809,  // index+middle+ring
        0b1001 => 895,  // index+pinky
        0b0101 => 913,  // index+ring
        0b1100 => 950,  // ring+pinky
        0b1110 => 992,  // middle+ring+pinky
        0b1010 => 1099, // middle+pinky
        0b1101 => 1254, // index+ring+pinky
        0b1011 => 1516, // index+middle+pinky
        _ => 2000,      // shouldn't happen
    }
}

/// Total effort for a chord (right 5-bit, left 4-bit).
/// Thumb (bit4) adds ~200ms penalty (measured average overhead).
fn chord_effort(right: u8, left: u8) -> u32 {
    let mod_cost = if right & 0b10000 != 0 { 200 } else { 0 };
    let right_fingers = right & 0xF;
    finger_combo_effort(right_fingers) + finger_combo_effort(left) + mod_cost
}

/// Return all valid chord slots (right, left) sorted by ergonomic ease.
/// Excludes (0, 0) since that's no chord.
/// Excludes left-only slots (right=0, left!=0) since those are reserved for suffixes.
fn all_slots_by_effort() -> Vec<(u8, u8)> {
    let mut slots: Vec<(u8, u8, u32)> = Vec::new();
    for right in 0u8..32 {
        for left in 0u8..16 {
            if right == 0 && left == 0 {
                continue;
            }
            // Left-only slots are reserved for suffixes
            if right == 0 && left != 0 {
                continue;
            }
            let e = chord_effort(right, left);
            slots.push((right, left, e));
        }
    }
    slots.sort_by_key(|&(r, l, e)| (e, popcount(r) + popcount(l), r, l));
    slots.into_iter().map(|(r, l, _)| (r, l)).collect()
}

// ── Phoneme label helpers for comments ───────────────────────────────────────

fn right_label(right: u8) -> String {
    let fingers = right & 0xF;
    let has_mod = right & 0b10000 != 0;
    let cons = match fingers {
        0b0000 => "-",
        0b0001 => "T",
        0b0010 => "S",
        0b0100 => "K",
        0b1000 => "P",
        0b0011 => "N",
        0b0101 => "R",
        0b0110 => "L",
        0b0111 => "H",
        0b1001 => "F",
        0b1010 => "W",
        0b1100 => "Th",
        0b1011 => "Sh",
        0b1101 => "Ch",
        0b1110 => "Ng",
        0b1111 => "Y",
        _ => "?",
    };
    let voiced = if has_mod {
        match fingers {
            0b0001 => "D",
            0b0010 => "Z",
            0b0100 => "G",
            0b1000 => "B",
            0b0011 => "M",
            0b0101 => "Dh",
            0b1001 => "V",
            0b1011 => "Zh",
            0b1101 => "Jh",
            _ => return format!("{}+mod", cons),
        }
    } else {
        cons
    };
    if has_mod && fingers != 0 {
        voiced.to_string()
    } else {
        cons.to_string()
    }
}

fn left_label(left: u8) -> String {
    match left {
        0b0000 => "-".into(),
        0b0001 => "Ah".into(),
        0b0010 => "Ih".into(),
        0b0100 => "Eh".into(),
        0b1000 => "Ae".into(),
        0b0011 => "Iy".into(),
        0b0101 => "Aa".into(),
        0b0110 => "Ey".into(),
        0b0111 => "Er".into(),
        0b1001 => "Ay".into(),
        0b1010 => "Ow".into(),
        0b1100 => "Ao".into(),
        0b1011 => "Uw".into(),
        0b1101 => "Aw".into(),
        0b1110 => "Uh".into(),
        0b1111 => "Oy".into(),
        _ => "?".into(),
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cmu_path = project.join("data/cmudict.dict");
    let freq_path = project.join("data/en_freq.txt");
    let out_path = project.join("src/briefs_data.rs");

    // 1. Load CMU dict (word → phoneme list)
    let mut cmu: HashMap<String, Vec<String>> = HashMap::new();
    {
        let f = fs::File::open(&cmu_path).expect("cannot open cmudict.dict");
        for line in BufReader::new(f).lines() {
            let line = line.unwrap();
            let line = line.trim();
            if line.is_empty() || line.starts_with(";;;") {
                continue;
            }
            let mut parts = line.splitn(2, ' ');
            let word_raw = match parts.next() {
                Some(w) => w.trim(),
                None => continue,
            };
            let phonemes = match parts.next() {
                Some(p) => p.trim(),
                None => continue,
            };
            // Skip alternate pronunciations like "word(2)"
            if word_raw.contains('(') {
                continue;
            }
            // Skip words with punctuation (e.g. "'s", "a.")
            let word = word_raw.to_lowercase();
            if !word.chars().all(|c| c.is_ascii_alphabetic() || c == '\'') {
                continue;
            }
            // Only keep words that are pure alpha (no apostrophes for simplicity in briefs)
            if !word.chars().all(|c| c.is_ascii_alphabetic()) {
                // Allow "don't" style later if needed, but skip for now
                // Actually let's allow apostrophe words
            }
            let phs: Vec<String> = phonemes.split_whitespace().map(String::from).collect();
            cmu.entry(word).or_insert(phs);
        }
    }
    eprintln!("Loaded {} CMU entries", cmu.len());

    // 2. Load frequency list
    let mut freq_words: Vec<(String, u64)> = Vec::new();
    {
        let f = fs::File::open(&freq_path).expect("cannot open en_freq.txt");
        for line in BufReader::new(f).lines() {
            let line = line.unwrap();
            let mut parts = line.split_whitespace();
            let word = match parts.next() {
                Some(w) => w.to_lowercase(),
                None => continue,
            };
            let count: u64 = match parts.next().and_then(|c| c.parse().ok()) {
                Some(c) => c,
                None => continue,
            };
            freq_words.push((word, count));
        }
    }
    // Sort descending by frequency
    freq_words.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("Loaded {} frequency entries", freq_words.len());

    // 3. Take top 1000 words that exist in CMU
    //    Skip apostrophe words and contraction fragments.
    let mut top_words: Vec<(String, u64, Vec<String>)> = Vec::new();
    for (word, count) in &freq_words {
        if top_words.len() >= 1000 {
            break;
        }
        if word.contains('\'') {
            continue;
        }
        // Skip contraction fragments (high-freq only because of "don't", "won't", etc.)
        const FRAGMENTS: &[&str] = &[
            "don", "doesn", "didn", "wasn", "weren", "isn",
            "won", "wouldn", "couldn", "shouldn", "hasn",
            "hadn", "ain", "aren", "mustn",
        ];
        if FRAGMENTS.contains(&word.as_str()) {
            continue;
        }
        // Skip proper nouns (names from subtitle corpus)
        const NAMES: &[&str] = &[
            "jesus", "michael", "david", "frank", "charlie",
            "jack", "john", "george", "sam", "harry", "joe",
            "tom", "bob", "henry", "alex", "nick", "max",
            "ben", "dan", "tony", "tommy", "jimmy", "johnny",
            "bobby", "danny", "brian", "mary", "sarah", "anna",
            "elizabeth", "peter", "james", "paul", "richard",
            "robert", "bill", "mike", "ray", "eddie", "leo",
            "steve", "chris", "matt", "mark", "scott", "eric",
            "grace", "emma", "kate", "rachel", "sophie", "lily",
        ];
        if NAMES.contains(&word.as_str()) {
            continue;
        }
        if let Some(phs) = cmu.get(word) {
            top_words.push((word.clone(), *count, phs.clone()));
        }
    }
    eprintln!("Selected {} words for brief assignment", top_words.len());

    // 3b. Filter out inflected forms whose base word is already in the list.
    // If "look" is in the list, "looking" is redundant (use "look" + suffix brief).
    let base_words: HashSet<String> = top_words.iter().map(|(w, _, _)| w.clone()).collect();
    let before_filter = top_words.len();
    top_words.retain(|(word, _, _)| !is_inflected_of_base(word, &base_words));
    eprintln!(
        "Filtered {} inflected forms (keeping {} base words)",
        before_filter - top_words.len(),
        top_words.len()
    );

    // 4. Compute natural brief for each word
    struct WordInfo {
        word: String,
        #[allow(dead_code)]
        rank: usize,
        natural_right: u8,
        natural_left: u8,
        first_consonant: String,
        first_vowel: String,
        phoneme_count: usize,
        value_right_only: f64, // frequency * (phonemes - 1)
        value_two_hand: f64,   // frequency * (phonemes - 2)
    }

    let mut words: Vec<WordInfo> = Vec::new();
    for (rank, (word, count, phs)) in top_words.iter().enumerate() {
        let stripped: Vec<&str> = phs.iter().map(|p| strip_stress(p)).collect();

        let first_cons = stripped.iter().find(|p| is_consonant_phoneme(p)).copied();
        let first_vow = stripped.iter().find(|p| is_vowel_phoneme(p)).copied();

        let right = first_cons
            .and_then(cmu_consonant_to_right)
            .unwrap_or(0);
        let left = first_vow.and_then(cmu_vowel_to_left).unwrap_or(0);

        // Count phonemes (consonants + vowels that map to our system)
        let phoneme_count = stripped
            .iter()
            .filter(|p| is_consonant_phoneme(p) || is_vowel_phoneme(p))
            .count();

        // Value depends on slot type:
        //   right-only slot: saved = phoneme_count - 1
        //   two-hand slot:   saved = phoneme_count - 2
        // We compute both; assignment phase picks the right one.
        let value_right_only = *count as f64 * (phoneme_count.saturating_sub(1) as f64);
        let value_two_hand = *count as f64 * (phoneme_count.saturating_sub(2) as f64);

        words.push(WordInfo {
            word: word.clone(),
            rank,
            natural_right: right,
            natural_left: left,
            first_consonant: first_cons.unwrap_or("-").to_string(),
            first_vowel: first_vow.unwrap_or("-").to_string(),
            phoneme_count,
            value_right_only,
            value_two_hand,
        });
    }

    // 5. Assignment
    //
    // Phase 0: Pinned overrides.
    // Phase A: Right-only slots (left=0) for words with 2+ phonemes, by value_right_only.
    // Phase B: Two-hand slots for words with 3+ phonemes, by value_two_hand.
    // Words with value=0 are skipped (no savings from a brief).

    // ─── PINNED OVERRIDES ───
    let pinned: &[(u8, u8, &str)] = &[
        (0b10000, 0b0000, "the"),   // fastest slot (mod only) for most common word
    ];

    let all_slots = all_slots_by_effort();
    let mut occupied: HashSet<(u8, u8)> = HashSet::new();
    let mut assigned_words: HashSet<String> = HashSet::new();
    let mut assignments: Vec<(u8, u8, String, String)> = Vec::new();

    // Phase 0: pinned
    for &(right, left, word) in pinned {
        occupied.insert((right, left));
        assigned_words.insert(word.to_string());
        assignments.push((right, left, word.to_string(), "pinned".into()));
    }

    // Phase A: right-only slots (left=0) — 2+ phoneme words, sorted by value_right_only
    let right_only_slots: Vec<(u8, u8)> = all_slots
        .iter()
        .filter(|&&(r, l)| r != 0 && l == 0 && !occupied.contains(&(r, l)))
        .copied()
        .collect();

    let mut words_for_right: Vec<&WordInfo> = words
        .iter()
        .filter(|w| w.value_right_only > 0.0 && !assigned_words.contains(&w.word))
        .collect();
    words_for_right.sort_by(|a, b| b.value_right_only.partial_cmp(&a.value_right_only).unwrap());

    for (slot, w) in right_only_slots.iter().zip(words_for_right.iter()) {
        occupied.insert(*slot);
        assigned_words.insert(w.word.clone());
        let comment = format!(
            "R-only val={:.0} {}ph ({}+{})",
            w.value_right_only, w.phoneme_count, w.first_consonant, w.first_vowel
        );
        assignments.push((slot.0, slot.1, w.word.clone(), comment));
    }

    eprintln!("  Phase A: {} right-only briefs", assignments.len() - 1); // minus pinned

    // Phase B: two-hand slots (left!=0, right!=0) — 3+ phoneme words, sorted by value_two_hand
    // Then fill remaining slots with 2-phoneme words that didn't get right-only slots
    let two_hand_slots: Vec<(u8, u8)> = all_slots
        .iter()
        .filter(|&&(r, l)| r != 0 && l != 0 && !occupied.contains(&(r, l)))
        .copied()
        .collect();

    let mut words_for_two: Vec<&WordInfo> = words
        .iter()
        .filter(|w| !assigned_words.contains(&w.word))
        .collect();
    // Sort: 3+ phoneme words by value_two_hand first, then remaining by value_right_only
    words_for_two.sort_by(|a, b| {
        let a_val = if a.phoneme_count >= 3 { a.value_two_hand } else { 0.0 };
        let b_val = if b.phoneme_count >= 3 { b.value_two_hand } else { 0.0 };
        b_val.partial_cmp(&a_val).unwrap()
    });

    let phase_b_start = assignments.len();
    for (slot, w) in two_hand_slots.iter().zip(words_for_two.iter()) {
        if w.phoneme_count < 3 && w.value_two_hand <= 0.0 {
            break; // no more words worth assigning to two-hand slots
        }
        occupied.insert(*slot);
        assigned_words.insert(w.word.clone());
        let saved = w.phoneme_count.saturating_sub(2);
        let comment = format!(
            "val={:.0} {}ph save={} ({}+{})",
            w.value_two_hand, w.phoneme_count, saved, w.first_consonant, w.first_vowel
        );
        assignments.push((slot.0, slot.1, w.word.clone(), comment));
    }

    eprintln!("  Phase B: {} two-hand briefs", assignments.len() - phase_b_start);

    // Phase C: fill remaining two-hand slots with leftover words (value > 0)
    let remaining_slots: Vec<(u8, u8)> = all_slots
        .iter()
        .filter(|s| !occupied.contains(s) && s.0 != 0 && s.1 != 0)
        .copied()
        .collect();

    let mut leftover_words: Vec<&WordInfo> = words
        .iter()
        .filter(|w| !assigned_words.contains(&w.word) && w.phoneme_count >= 2)
        .collect();
    leftover_words.sort_by(|a, b| b.value_right_only.partial_cmp(&a.value_right_only).unwrap());

    let phase_c_start = assignments.len();
    for (slot, w) in remaining_slots.iter().zip(leftover_words.iter()) {
        occupied.insert(*slot);
        assigned_words.insert(w.word.clone());
        let comment = format!(
            "fill val={:.0} {}ph ({}+{})",
            w.value_right_only, w.phoneme_count, w.first_consonant, w.first_vowel
        );
        assignments.push((slot.0, slot.1, w.word.clone(), comment));
    }

    eprintln!("  Phase C: {} fill briefs", assignments.len() - phase_c_start);

    // Sort: pinned first, then right-only by effort, then two-hand by effort
    assignments.sort_by_key(|(r, l, _, comment)| {
        let is_pinned = comment == "pinned";
        let is_right_only = *l == 0 && !is_pinned;
        let effort = chord_effort(*r, *l);
        // pinned=0, right-only=1, two-hand=2, then by effort within each group
        let group = if is_pinned { 0u32 } else if is_right_only { 1 } else { 2 };
        (group, effort)
    });

    // 6. Write output
    let mut out = String::new();
    out.push_str(
        "/// Auto-generated brief assignments. Edit and recompile to customize.\n\
         /// Format: (left_4bits, right_5bits, \"word\")\n\
         ///\n\
         /// Bit encoding (both hands): index=bit0 (LSB), outward from center.\n\
         /// Left:  I=0001 M=0010 R=0100 P=1000\n\
         /// Right: I=0001 M=0010 R=0100 P=1000 T(thumb/mod)=10000\n\
         ///\n\
         /// Note: binary literals read right-to-left (LSB first), so\n\
         /// 0b0110 = middle+ring, NOT ring+middle. Index is always the rightmost bit.\n\
         pub const BRIEFS: &[(u8, u8, &str)] = &[\n",
    );

    for (right, left, word, comment) in &assignments {
        out.push_str(&format!(
            "    (0b{:04b}, 0b{:05b}, {:w$}  // {}\n",
            left,
            right,
            format!("\"{}\"),", word),
            comment,
            w = 20,
        ));
    }

    out.push_str("];\n");

    fs::write(&out_path, &out).expect("cannot write briefs_data.rs");
    eprintln!(
        "Wrote {} briefs to {}",
        assignments.len(),
        out_path.display()
    );
}

/// Check if `word` is an inflected form of another word in `base_words`.
/// Returns true if we should SKIP this word (it's redundant with base + suffix).
fn is_inflected_of_base(word: &str, base_words: &HashSet<String>) -> bool {
    // Irregular forms — hardcoded map for common ones in top 1000
    static IRREGULARS: &[(&str, &str)] = &[
        ("went", "go"),
        ("gone", "go"),
        ("going", "go"),
        ("goes", "go"),
        ("was", "be"),
        ("were", "be"),
        ("been", "be"),
        ("being", "be"),
        ("had", "have"),
        ("has", "have"),
        ("having", "have"),
        ("did", "do"),
        ("does", "do"),
        ("doing", "do"),
        ("done", "do"),
        ("said", "say"),
        ("saying", "say"),
        ("says", "say"),
        ("told", "tell"),
        ("telling", "tell"),
        ("took", "take"),
        ("taken", "take"),
        ("taking", "take"),
        ("got", "get"),
        ("getting", "get"),
        ("gotten", "get"),
        ("came", "come"),
        ("coming", "come"),
        ("comes", "come"),
        ("made", "make"),
        ("making", "make"),
        ("makes", "make"),
        ("gave", "give"),
        ("given", "give"),
        ("giving", "give"),
        ("gives", "give"),
        ("knew", "know"),
        ("known", "know"),
        ("knowing", "know"),
        ("knows", "know"),
        ("thought", "think"),
        ("thinking", "think"),
        ("thinks", "think"),
        ("felt", "feel"),
        ("feeling", "feel"),
        ("feels", "feel"),
        ("left", "leave"),
        ("leaving", "leave"),
        ("leaves", "leave"),
        ("kept", "keep"),
        ("keeping", "keep"),
        ("keeps", "keep"),
        ("meant", "mean"),
        ("meaning", "mean"),
        ("means", "mean"),
        ("put", "put"),
        ("putting", "put"),
        ("ran", "run"),
        ("running", "run"),
        ("runs", "run"),
        ("sat", "sit"),
        ("sitting", "sit"),
        ("stood", "stand"),
        ("standing", "stand"),
        ("lost", "lose"),
        ("losing", "lose"),
        ("brought", "bring"),
        ("bringing", "bring"),
        ("began", "begin"),
        ("begun", "begin"),
        ("beginning", "begin"),
        ("wrote", "write"),
        ("written", "write"),
        ("writing", "write"),
        ("spoke", "speak"),
        ("spoken", "speak"),
        ("speaking", "speak"),
        ("better", "good"),
        ("best", "good"),
        ("worse", "bad"),
        ("worst", "bad"),
        ("children", "child"),
        ("men", "man"),
        ("women", "woman"),
    ];

    // Check irregulars first
    for &(inflected, base) in IRREGULARS {
        if word == inflected && base_words.contains(base) {
            return true;
        }
    }

    // Don't filter very short words (3 chars or less) — too risky
    if word.len() <= 3 {
        return false;
    }

    // Regular suffix stripping with spelling rules
    let candidates = stem_candidates(word);
    for stem in candidates {
        if stem != word && base_words.contains(&stem) {
            return true;
        }
    }

    false
}

/// Generate possible base forms by stripping common suffixes.
/// Handles English spelling rules (doubled consonants, dropped e, y→i).
fn stem_candidates(word: &str) -> Vec<String> {
    let mut stems = Vec::new();

    // -ing: running→run (doubled), making→make (dropped e), going→go
    if let Some(base) = word.strip_suffix("ing") {
        if !base.is_empty() {
            stems.push(base.to_string()); // go+ing = going
            stems.push(format!("{}e", base)); // mak+ing = making → make
            // Doubled consonant: runn → run
            let bytes = base.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                stems.push(base[..base.len() - 1].to_string());
            }
        }
    }

    // -ed: wanted→want, tried→try, stopped→stop
    if let Some(base) = word.strip_suffix("ed") {
        if !base.is_empty() {
            stems.push(base.to_string()); // want+ed
            stems.push(format!("{}e", base)); // lik+ed → like
            // Doubled: stopp → stop
            let bytes = base.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                stems.push(base[..base.len() - 1].to_string());
            }
        }
    }
    // -ied → y: tried → try
    if let Some(base) = word.strip_suffix("ied") {
        if !base.is_empty() {
            stems.push(format!("{}y", base));
        }
    }

    // -es: goes→go, watches→watch, tries→try
    if let Some(base) = word.strip_suffix("es") {
        if !base.is_empty() {
            stems.push(base.to_string());
            stems.push(format!("{}e", base));
        }
    }
    // -ies → y: tries → try
    if let Some(base) = word.strip_suffix("ies") {
        if !base.is_empty() {
            stems.push(format!("{}y", base));
        }
    }

    // -s (simple plural/3rd person): looks→look, tells→tell
    if let Some(base) = word.strip_suffix('s') {
        if base.len() >= 3 && !base.ends_with('s') {
            stems.push(base.to_string());
        }
    }

    // -ly: really→real, exactly→exact
    if let Some(base) = word.strip_suffix("ly") {
        if base.len() >= 3 {
            stems.push(base.to_string());
            // happily → happy (ily → y)
        }
    }
    if let Some(base) = word.strip_suffix("ily") {
        if !base.is_empty() {
            stems.push(format!("{}y", base));
        }
    }

    // -er: bigger→big, later→late, player→play
    if let Some(base) = word.strip_suffix("er") {
        if base.len() >= 2 {
            stems.push(base.to_string());
            stems.push(format!("{}e", base)); // lat+er → late
            let bytes = base.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                stems.push(base[..base.len() - 1].to_string()); // bigg → big
            }
        }
    }

    // -est: biggest→big, latest→late
    if let Some(base) = word.strip_suffix("est") {
        if base.len() >= 2 {
            stems.push(base.to_string());
            stems.push(format!("{}e", base));
            let bytes = base.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                stems.push(base[..base.len() - 1].to_string());
            }
        }
    }

    // -tion: action→act (less common in top 1000 but handle it)
    if let Some(base) = word.strip_suffix("tion") {
        if base.len() >= 2 {
            stems.push(base.to_string());
            stems.push(format!("{}t", base)); // ac+tion → act
            stems.push(format!("{}te", base)); // crea+tion → create
        }
    }

    // -ment: movement→move, agreement→agree
    if let Some(base) = word.strip_suffix("ment") {
        if base.len() >= 3 {
            stems.push(base.to_string());
        }
    }

    // -ness: happiness→happy, kindness→kind
    if let Some(base) = word.strip_suffix("ness") {
        if base.len() >= 3 {
            stems.push(base.to_string());
        }
    }
    if let Some(base) = word.strip_suffix("iness") {
        if !base.is_empty() {
            stems.push(format!("{}y", base)); // happ+iness → happy
        }
    }

    stems
}

/// Find the nearest unoccupied slot to `target`.
/// Priority: same right (consonant) different left, then same left different right,
/// then fallback to any by effort.
fn find_nearest_slot(
    target: (u8, u8),
    occupied: &HashSet<(u8, u8)>,
    all_slots: &[(u8, u8)],
) -> Option<(u8, u8)> {
    let (tr, tl) = target;

    // 1. Same consonant, different vowel (sorted by effort)
    let same_cons: Option<(u8, u8)> = all_slots
        .iter()
        .filter(|(r, l)| *r == tr && *l != tl && !occupied.contains(&(*r, *l)))
        .copied()
        .next();
    if same_cons.is_some() {
        return same_cons;
    }

    // 2. Same vowel, different consonant
    let same_vowel: Option<(u8, u8)> = all_slots
        .iter()
        .filter(|(r, l)| *l == tl && *r != tr && !occupied.contains(&(*r, *l)))
        .copied()
        .next();
    if same_vowel.is_some() {
        return same_vowel;
    }

    // 3. Any unoccupied slot by effort
    all_slots
        .iter()
        .find(|s| !occupied.contains(s))
        .copied()
}
