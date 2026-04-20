use std::collections::HashMap;
use crate::chord_map::{BriefTable, ChordKey};

/// Generate word briefs — assign common words to both-hands chord combos.
///
/// Both-hands combos (right != 0 && left != 0) are used for briefs.
/// With mod: 15 × 15 × 2 = 450 slots.
/// Without mod: 15 × 15 = 225 slots (prefer these — no thumb needed).
///
/// Phoneme slots (single-hand) are reserved and never used for briefs.
pub fn generate_briefs(
    cmudict_text: &str,
    freq_text: &str,
) -> BriefTable {
    // Parse word frequencies, most common first
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

    // Parse CMU dict to verify words exist
    let mut cmudict: HashMap<String, bool> = HashMap::new();
    for line in cmudict_text.lines() {
        if line.starts_with(";;;") { continue; }
        if let Some(word) = line.split_whitespace().next() {
            let w = word.split('(').next().unwrap().to_lowercase();
            cmudict.insert(w, true);
        }
    }

    // Build both-hands slots sorted by ergonomic ease
    let single_effort = [1.0f64, 1.1, 1.5, 2.0]; // index, middle, ring, pinky
    let combo_effort = |bits: u8| -> f64 {
        let fingers: Vec<u8> = (0..4).filter(|&i| bits & (1 << i) != 0).collect();
        if fingers.len() == 1 {
            single_effort[fingers[0] as usize]
        } else {
            fingers.iter().map(|&f| single_effort[f as usize]).sum::<f64>() * 0.6
        }
    };

    let mut slots: Vec<(u16, f64)> = Vec::new();

    // Both-hands, no mod (easiest)
    for right in 1..16u8 {
        for left in 1..16u8 {
            let key = right as u16 | (left as u16) << 4;
            let effort = combo_effort(right) + combo_effort(left);
            slots.push((key, effort));
        }
    }

    // Both-hands, with mod
    for right in 1..16u8 {
        for left in 1..16u8 {
            let key = right as u16 | (left as u16) << 4 | (1u16 << 8);
            let effort = combo_effort(right) + combo_effort(left) + 0.5; // mod penalty
            slots.push((key, effort));
        }
    }

    // Sort by effort (easiest first)
    slots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let mut briefs = BriefTable::new();
    let mut assigned = 0usize;

    for (word, _) in &word_freq {
        if assigned >= slots.len() { break; }
        if !cmudict.contains_key(word) { continue; }
        if word.contains('\'') { continue; }

        let (key, _) = slots[assigned];
        briefs.insert(ChordKey(key), format!("{} ", word));
        assigned += 1;
    }

    eprintln!("rhe: briefs: {} words assigned to both-hands combos", assigned);

    briefs
}

/// Print coverage stats.
pub fn print_coverage(briefs: &BriefTable, freq_text: &str) {
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

    let mut covered_freq: u64 = 0;
    let mut covered_count = 0;

    for (word, freq) in word_freq.iter().take(500) {
        for key in 0..ChordKey::MAX {
            if let Some(brief) = briefs.lookup(ChordKey(key)) {
                if brief.trim() == word {
                    covered_freq += freq;
                    covered_count += 1;
                    break;
                }
            }
        }
    }

    eprintln!(
        "rhe: brief coverage: {}/{} of top 500 words ({:.1}% of all usage)",
        covered_count, 500,
        covered_freq as f64 / total_freq as f64 * 100.0
    );
}
