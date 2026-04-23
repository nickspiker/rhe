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
    let candidates_path = project.join("data/brief_candidates.txt");
    let homophones_path = project.join("data/homophones.txt");
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

    // 3c. Candidate curation.
    //
    // If `data/brief_candidates.txt` exists, treat it as the user-curated
    // source of truth: only words listed in it get assigned briefs. That
    // file is what the user edits — deleting a line blacklists the word.
    //
    // If it doesn't exist, write the current default list out so the user
    // can pick it up and prune. To regenerate defaults, delete the file.
    top_words = load_or_write_candidates(&candidates_path, &top_words, &cmu);
    eprintln!("Using {} candidate words from {}", top_words.len(), candidates_path.display());

    // 3d. Write a homophone-collision report. Helps the user decide
    // which pairs/sets deserve ordered-brief entries. Scope: any CMU
    // word sharing a phoneme sequence with a candidate word, restricted
    // to words that also appear in the frequency list (drops obscure
    // CMU entries that would otherwise pollute the report).
    write_homophone_report(&homophones_path, &top_words, &cmu, &freq_words);

    // 4. Compute natural brief for each word
    struct WordInfo {
        word: String,
        first_consonant: String,
        first_vowel: String,
        phoneme_count: usize,
        /// Savings-weighted value: `frequency × (phonemes - 1)`.
        /// Actual keystroke savings regardless of slot type.
        value: f64,
    }

    let mut words: Vec<WordInfo> = Vec::new();
    for (word, count, phs) in top_words.iter() {
        let stripped: Vec<&str> = phs.iter().map(|p| strip_stress(p)).collect();

        let first_cons = stripped.iter().find(|p| is_consonant_phoneme(p)).copied();
        let first_vow = stripped.iter().find(|p| is_vowel_phoneme(p)).copied();

        // Count phonemes (consonants + vowels that map to our system)
        let phoneme_count = stripped
            .iter()
            .filter(|p| is_consonant_phoneme(p) || is_vowel_phoneme(p))
            .count();

        let value = *count as f64 * (phoneme_count.saturating_sub(1) as f64);

        words.push(WordInfo {
            word: word.clone(),
            first_consonant: first_cons.unwrap_or("-").to_string(),
            first_vowel: first_vow.unwrap_or("-").to_string(),
            phoneme_count,
            value,
        });
    }

    // 5. Assignment
    //
    // Pinned / ordered-claimed slots first, then one greedy pass:
    // words sorted by savings-weighted value descending, slots sorted
    // by ergonomic effort ascending, zip them. Highest-value word
    // gets the easiest free slot.

    let pinned: &[(u8, u8, &str)] = &[];

    // Mirror of `ORDERED_BRIEFS` in src/ordered_briefs_data.rs. Used here
    // for two things:
    //   1. Mark the (right, left) slots as occupied so unordered briefs
    //      don't collide with them.
    //   2. Exclude the listed words from the unordered candidate pool so
    //      they don't get a second brief somewhere else (ordered brief
    //      is already their home).
    // KEEP IN SYNC when ordered_briefs_data.rs changes.
    // Format: (right_5bits, left_4bits, word).
    const ORDERED_CLAIMED: &[(u8, u8, &str)] = &[
        // 2-way symmetric splits
        (0b00010, 0b0010, "no"),
        (0b00010, 0b0010, "know"),
        (0b01000, 0b1000, "here"),
        (0b01000, 0b1000, "hear"),
        (0b00100, 0b0100, "right"),
        (0b00100, 0b0100, "write"),
        // Single-hand + thumb (thumb-first = rare)
        (0b10011, 0b0000, "to"),
        (0b10011, 0b0000, "too"),
        (0b10011, 0b0000, "two"),
        (0b10010, 0b0000, "in"),
        (0b10010, 0b0000, "inn"),
        (0b10101, 0b0000, "do"),
        (0b10101, 0b0000, "due"),
        (0b10000, 0b0100, "not"),
        (0b10000, 0b0100, "knot"),
        (0b10000, 0b1110, "be"),
        (0b10000, 0b1110, "bee"),
        (0b10000, 0b0010, "but"),
        (0b10000, 0b0010, "butt"),
        (0b10110, 0b0000, "there"),
        (0b10110, 0b0000, "their"),
        (0b11011, 0b1010, "read"),
        (0b11011, 0b1010, "red"),
        (0b11100, 0b0100, "son"),
        (0b11100, 0b0100, "sun"),
        (0b11110, 0b0111, "meet"),
        (0b11110, 0b0111, "meat"),
        (0b10010, 0b0010, "wait"),
        (0b10010, 0b0010, "weight"),
        (0b10001, 0b0101, "through"),
        (0b10001, 0b0101, "threw"),
        (0b10100, 0b0011, "which"),
        (0b10100, 0b0011, "witch"),
        // Pinky-first for rare (no thumb)
        (0b01000, 0b1100, "our"),
        (0b01000, 0b1100, "hour"),
        (0b00111, 0b0001, "where"),
        (0b00111, 0b0001, "wear"),
        (0b01000, 0b1101, "new"),
        (0b01000, 0b1101, "knew"),
        (0b01011, 0b1010, "week"),
        (0b01011, 0b1010, "weak"),
        (0b00110, 0b1000, "would"),
        (0b00110, 0b1000, "wood"),
        (0b01101, 0b0100, "whole"),
        (0b01101, 0b0100, "hole"),
        (0b00101, 0b0001, "see"),
        (0b00101, 0b0001, "sea"),
        (0b00101, 0b1111, "night"),
        (0b00101, 0b1111, "knight"),
        // 3-way / special
        (0b01111, 0b0000, "for"),
        (0b01111, 0b0000, "four"),
        (0b01111, 0b0000, "fore"),
        (0b01100, 0b0110, "by"),
        (0b01100, 0b0110, "buy"),
        (0b01100, 0b0110, "bye"),
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

    // Ordered-brief slots are occupied; those words live in
    // `ORDERED_BRIEFS` directly and shouldn't also land in BRIEFS.
    for &(right, left, word) in ORDERED_CLAIMED {
        occupied.insert((right, left));
        assigned_words.insert(word.to_string());
    }

    // Greedy assignment: highest-value word gets the easiest free slot.
    let mut unassigned: Vec<&WordInfo> = words
        .iter()
        .filter(|w| w.value > 0.0 && !assigned_words.contains(&w.word))
        .collect();
    unassigned.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());

    let free_slots: Vec<(u8, u8)> = all_slots
        .iter()
        .filter(|s| !occupied.contains(s))
        .copied()
        .collect();

    let assigned_at_start = assignments.len();
    for (slot, w) in free_slots.iter().zip(unassigned.iter()) {
        occupied.insert(*slot);
        assigned_words.insert(w.word.clone());
        let slot_kind = if slot.1 == 0 { "R-only" } else { "2-hand" };
        let comment = format!(
            "{} val={:.0} {}ph ({}+{})",
            slot_kind, w.value, w.phoneme_count, w.first_consonant, w.first_vowel
        );
        assignments.push((slot.0, slot.1, w.word.clone(), comment));
    }

    eprintln!(
        "  Assigned {} briefs by value × slot-effort ranking",
        assignments.len() - assigned_at_start
    );

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

/// Group CMU words by phoneme sequence and report collisions where at
/// least one member is in the candidate pool. Output goes to
/// `data/homophones.txt` for the user to browse and decide which pairs
/// warrant ordered-brief entries in `src/ordered_briefs_data.rs`.
///
/// Only words that appear in `en_freq.txt` are included (filters out
/// obscure CMU entries that would otherwise dominate the report).
/// Groups are sorted by max member frequency descending.
fn write_homophone_report(
    path: &Path,
    candidates: &[(String, u64, Vec<String>)],
    cmu: &HashMap<String, Vec<String>>,
    freq_words: &[(String, u64)],
) {
    let seq_of = |phs: &[String]| -> String {
        phs.iter().map(|p| strip_stress(p)).collect::<Vec<_>>().join(" ")
    };

    let freq_lookup: HashMap<&str, u64> =
        freq_words.iter().map(|(w, c)| (w.as_str(), *c)).collect();

    // Candidate phoneme sequences — we only report groups whose
    // phoneme sequence is reachable via a candidate word (so the report
    // is useful to curation, not noisy).
    let candidate_seqs: HashSet<String> =
        candidates.iter().map(|(_, _, phs)| seq_of(phs)).collect();

    // Group all frequency-listed CMU words by phoneme sequence.
    let mut groups: HashMap<String, Vec<(String, u64)>> = HashMap::new();
    for (word, phs) in cmu {
        let Some(&freq) = freq_lookup.get(word.as_str()) else {
            continue;
        };
        let seq = seq_of(phs);
        if !candidate_seqs.contains(&seq) {
            continue;
        }
        groups.entry(seq).or_default().push((word.clone(), freq));
    }

    // Only report groups with >1 member.
    let mut reportable: Vec<(String, Vec<(String, u64)>)> = groups
        .into_iter()
        .filter(|(_, ws)| ws.len() >= 2)
        .collect();
    for (_, ws) in &mut reportable {
        ws.sort_by_key(|(_, f)| std::cmp::Reverse(*f));
    }
    reportable.sort_by_key(|(_, ws)| std::cmp::Reverse(ws[0].1));

    let mut text = String::new();
    text.push_str(
        "# Homophone collision report.\n\
         #\n\
         # Each line is a phoneme sequence followed by every CMU word\n\
         # that pronounces to it (with frequency). Use this list to\n\
         # pick candidates for ordered briefs (src/ordered_briefs_data.rs).\n\
         #\n\
         # Phoneme path can only reach the most-frequent word of each\n\
         # set — the others require a brief (ordered or unordered).\n\
         # Auto-regenerated each gen_briefs run from cmudict × en_freq.\n\
         #\n",
    );
    for (seq, words) in &reportable {
        let list = words
            .iter()
            .map(|(w, f)| format!("{} ({})", w, f))
            .collect::<Vec<_>>()
            .join(", ");
        text.push_str(&format!("{:<16}  {}\n", seq, list));
    }

    fs::write(path, text).expect("cannot write homophones.txt");
    eprintln!(
        "Wrote {} homophone sets to {}",
        reportable.len(),
        path.display()
    );
}

/// Read `data/brief_candidates.txt` if it exists, else write the current
/// defaults to it. Returns the candidate list to use for assignment
/// (freq-ordered, with CMU phonemes attached).
///
/// File format: one word per line, blank lines and `#`-comment lines
/// ignored. The word is the last whitespace-separated token on the line,
/// so the auto-generated annotations (`rank  frequency  phonemes  word`)
/// parse cleanly without needing the user to strip columns.
fn load_or_write_candidates(
    path: &Path,
    defaults: &[(String, u64, Vec<String>)],
    cmu: &HashMap<String, Vec<String>>,
) -> Vec<(String, u64, Vec<String>)> {
    if path.exists() {
        let freq_by_word: HashMap<&str, u64> = defaults
            .iter()
            .map(|(w, c, _)| (w.as_str(), *c))
            .collect();
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let content = fs::read_to_string(path).expect("cannot read brief_candidates.txt");
        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some(word) = trimmed.split_whitespace().last() else {
                continue;
            };
            let word = word.to_lowercase();
            if !seen.insert(word.clone()) {
                continue;
            }
            let Some(phs) = cmu.get(&word) else {
                eprintln!(
                    "  warning: {}:{}: {} not in CMU dict — skipping",
                    path.display(),
                    lineno + 1,
                    word
                );
                continue;
            };
            // Use freq from the default list if we can, else 0 (user-added
            // words can still be assigned, just with lower value).
            let count = freq_by_word.get(word.as_str()).copied().unwrap_or(0);
            out.push((word, count, phs.clone()));
        }
        out
    } else {
        // Rank by keystroke-savings value, not raw frequency. A brief
        // replaces phc phoneme chords with 1 chord, so a word's value is
        // approximately `frequency × (phonemes - 1)`. Single-phoneme
        // words have value 0 — a brief saves nothing and they're dropped
        // from the list entirely. Users can add them back by hand if
        // they really want.
        let mut scored: Vec<(&(String, u64, Vec<String>), u64, usize)> = defaults
            .iter()
            .filter_map(|entry| {
                let (_, count, phs) = entry;
                let phc = phs
                    .iter()
                    .map(|p| strip_stress(p))
                    .filter(|p| is_consonant_phoneme(p) || is_vowel_phoneme(p))
                    .count();
                if phc < 2 {
                    return None;
                }
                let savings = (phc - 1) as u64;
                let value = *count * savings;
                Some((entry, value, phc))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        let mut text = String::new();
        text.push_str(
            "# Brief candidates for rhe, ranked by savings-weighted value.\n\
             #\n\
             # value = frequency × (phonemes - 1). Single-phoneme words are\n\
             # omitted because a brief chord saves nothing over typing the\n\
             # one phoneme directly.\n\
             #\n\
             # One word per line. Delete any line to exclude that word from\n\
             # brief assignment. Lines starting with '#' are ignored. Delete\n\
             # the whole file to regenerate from defaults.\n\
             #\n\
             # Adding a word not in this list is fine — give it its own line\n\
             # with any annotations you like, as long as the word is the\n\
             # last whitespace-separated token.\n\
             #\n\
             # Fields: rank  value  phonemes  word\n\
             #\n",
        );
        for (i, (entry, value, phc)) in scored.iter().enumerate() {
            text.push_str(&format!(
                "{:>5}  {:>14}  {:>3}  {}\n",
                i + 1,
                value,
                phc,
                entry.0
            ));
        }
        fs::write(path, text).expect("cannot write brief_candidates.txt");
        eprintln!(
            "Wrote default candidate list to {} ({} words, 1-phoneme filtered). Edit it and rerun.",
            path.display(),
            scored.len()
        );
        scored.into_iter().map(|(e, _, _)| e.clone()).collect()
    }
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
