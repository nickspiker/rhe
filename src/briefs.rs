use std::collections::HashMap;

use crate::table_gen::syllabify;

/// Generate word briefs — pure frequency assignment, no phonetics.
///
/// Single-hand chords (right-only or left-only) get the most common
/// words. These are instant, no space needed, no ordering to think about.
///
/// Right-hand only = always Mode 1 (first-down=right, first-up=right)
/// Left-hand only = always Mode 4 (first-down=left, first-up=left)
///
/// 15 finger combos × 2 ctrl states × 2 hands = 60 single-hand slots
/// for the top 60 words.
///
/// Remaining briefs use both-hands chords (any mode).
pub fn generate_briefs(
    cmudict_text: &str,
    freq_text: &str,
    syllable_table: &HashMap<u16, String>,
) -> HashMap<u16, String> {
    // Parse word frequencies, sorted most common first
    let mut word_freq: Vec<(String, u64)> = Vec::new();
    for line in freq_text.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(word), Some(count)) = (parts.next(), parts.next()) {
            if let Ok(count) = count.parse::<u64>() {
                word_freq.push((word.to_lowercase(), count));
            }
        }
    }
    word_freq.sort_by(|a, b| b.1.cmp(&a.1));

    // Parse CMU dict for syllable matching later
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

    let mut briefs: HashMap<u16, String> = HashMap::new();

    // Effort-sorted finger combos (1-15, skip 0 = no fingers)
    let single_effort = [1.0f64, 1.1, 1.5, 2.0]; // index, middle, ring, pinky
    let mut combos_by_effort: Vec<(u8, f64)> = (1..16u8)
        .map(|bits| {
            let fingers: Vec<u8> = (0..4).filter(|&i| bits & (1 << i) != 0).collect();
            let effort = if fingers.len() == 1 {
                single_effort[fingers[0] as usize]
            } else {
                let base: f64 = fingers.iter().map(|&f| single_effort[f as usize]).sum::<f64>() * 0.6;
                let stretch: f64 = fingers.windows(2)
                    .map(|w| { let gap = w[1] - w[0]; if gap > 1 { 0.3 * (gap - 1) as f64 } else { 0.0 } })
                    .sum();
                base + stretch
            };
            (bits, effort)
        })
        .collect();
    combos_by_effort.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Build single-hand brief slots:
    // Right-hand: chord key = right_bits | (0 << 4) | (mode_0 << 8) | (ctrl << 10)
    //   Mode 1 = 0, so mode bits = 0
    // Left-hand: chord key = 0 | (left_bits << 4) | (mode_4 << 8) | (ctrl << 10)
    //   Mode 4 = 3, so mode bits = 3
    let mut slots: Vec<u16> = Vec::new();

    // Right-hand, no ctrl (easiest)
    for &(bits, _) in &combos_by_effort {
        slots.push(bits as u16); // R:bits L:0 M:0 C:0
    }
    // Left-hand, no ctrl
    for &(bits, _) in &combos_by_effort {
        slots.push((bits as u16) << 4 | (3 << 8)); // R:0 L:bits M:3 C:0
    }
    // Right-hand, ctrl
    for &(bits, _) in &combos_by_effort {
        slots.push(bits as u16 | (1 << 10)); // R:bits L:0 M:0 C:1
    }
    // Left-hand, ctrl
    for &(bits, _) in &combos_by_effort {
        slots.push((bits as u16) << 4 | (3 << 8) | (1 << 10)); // R:0 L:bits M:3 C:1
    }

    // Assign top words to slots
    let mut slot_idx = 0;
    let mut single_count = 0;

    for (word, _) in &word_freq {
        if slot_idx >= slots.len() {
            break;
        }
        // Skip words not in CMU dict
        if !cmudict.contains_key(word) {
            continue;
        }
        // Skip contractions and possessives for now
        if word.contains('\'') {
            continue;
        }

        let key = slots[slot_idx];
        briefs.insert(key, format!("{} ", word));
        slot_idx += 1;
        single_count += 1;
    }

    // Then: add remaining single-syllable words as both-hands briefs
    // using their phonetic syllable mapping
    let mut ipa_to_key: HashMap<String, u16> = HashMap::new();
    for (&key_bits, ipa) in syllable_table {
        if key_bits & 0xFF != 0 && (key_bits & 0xF) != 0 && ((key_bits >> 4) & 0xF) != 0 {
            // Both hands have fingers — valid for syllable briefs
            ipa_to_key.entry(ipa.clone()).or_insert(key_bits);
        }
    }

    let mut phonetic_count = 0;
    for (word, _) in &word_freq {
        if briefs.len() >= 1500 {
            break;
        }
        let Some(phonemes) = cmudict.get(word) else { continue };
        if word.contains('\'') {
            continue;
        }
        let syllables = syllabify(phonemes);
        if syllables.len() != 1 {
            continue;
        }
        let ipa = syllables[0].to_ipa();
        if let Some(&key_bits) = ipa_to_key.get(&ipa) {
            if !briefs.contains_key(&key_bits) {
                // Check this word isn't already assigned to a single-hand slot
                if !briefs.values().any(|v| v.trim() == *word) {
                    briefs.insert(key_bits, format!("{} ", word));
                    phonetic_count += 1;
                }
            }
        }
    }

    eprintln!(
        "rhe: briefs: {} single-hand + {} phonetic = {} total",
        single_count, phonetic_count, briefs.len()
    );

    briefs
}

/// Print coverage stats.
pub fn print_coverage(briefs: &HashMap<u16, String>, freq_text: &str) {
    let mut word_freq: Vec<(String, u64)> = Vec::new();
    let mut total_freq: u64 = 0;
    for line in freq_text.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(word), Some(count)) = (parts.next(), parts.next()) {
            if let Ok(count) = count.parse::<u64>() {
                word_freq.push((word.to_lowercase(), count));
                total_freq += count;
            }
        }
    }
    word_freq.sort_by(|a, b| b.1.cmp(&a.1));

    let brief_words: std::collections::HashSet<String> =
        briefs.values().map(|w| w.trim().to_string()).collect();

    let mut covered_freq: u64 = 0;
    let mut covered_count = 0;

    for (word, freq) in word_freq.iter().take(500) {
        if brief_words.contains(word) {
            covered_freq += freq;
            covered_count += 1;
        }
    }

    eprintln!(
        "rhe: brief coverage: {}/{} of top 500 words ({:.1}% of all usage)",
        covered_count,
        500,
        covered_freq as f64 / total_freq as f64 * 100.0
    );
}
