//! Generates `src/layout/briefs.rs` — optimized brief (chord→word) assignments.
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
        "T" => Some(0b00001),
        "S" => Some(0b00010),
        "K" => Some(0b00100),
        "P" => Some(0b01000),
        "N" => Some(0b00011),
        "R" => Some(0b00101),
        "L" => Some(0b00110),
        "HH" => Some(0b00111),
        "F" => Some(0b01001),
        "W" => Some(0b01010),
        "TH" => Some(0b01100),
        "SH" => Some(0b01011),
        "CH" => Some(0b01101),
        "NG" => Some(0b01110),
        "Y" => Some(0b01111),
        // With mod (9 voiced consonants)
        "D" => Some(0b10001),
        "Z" => Some(0b10010),
        "G" => Some(0b10100),
        "B" => Some(0b11000),
        "M" => Some(0b10011),
        "DH" => Some(0b10101),
        "V" => Some(0b11001),
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
        "AH" | "IH"
            | "EH"
            | "AE"
            | "IY"
            | "AA"
            | "EY"
            | "ER"
            | "AY"
            | "OW"
            | "AO"
            | "UW"
            | "AW"
            | "UH"
            | "OY"
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

/// Finger effort for a 4-bit pattern (bits 0-3 = index, middle, ring, pinky). Lower = easier.
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
    let weight = if bits & 0b1000 != 0 { 2 } else { 0 } + if bits & 0b0100 != 0 { 1 } else { 0 };

    finger_cost + gap_penalty + weight
}

/// Measured finger combo effort (from bench data, averaged across hands). Lower = faster. Returns milliseconds as effort proxy.
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

/// Total effort for a chord (right 5-bit, left 4-bit). Thumb (bit4) adds ~200ms penalty (measured average overhead).
fn chord_effort(right: u8, left: u8) -> u32 {
    let mod_cost = if right & 0b10000 != 0 { 200 } else { 0 };
    let right_fingers = right & 0xF;
    finger_combo_effort(right_fingers) + finger_combo_effort(left) + mod_cost
}

/// Return all valid chord slots (right, left) sorted by ergonomic ease. Excludes (0, 0) since that's no chord. Excludes left-only slots (right=0, left!=0) since those are reserved for suffixes.
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

// ── Ordered-brief mirror ─────────────────────────────────────────────────────

/// Mirror of `ORDERED_BRIEFS` in `src/layout/ordered_briefs.rs`. Used in three places: (1) mark the `(right, left)` slots as occupied so unordered briefs don't collide; (2) exclude these words from the unordered candidate pool so they don't get a second brief somewhere else (their ordered slot is already their home); (3) feed the suffix-collision report so ordered briefs are also scanned for `base + <suffix>` redundancy.
///
/// KEEP IN SYNC with `ordered_briefs.rs`. Format: `(right_5bits, left_4bits, word)`.
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
    // Spelled numbers (position-aligned with number-mode)
    (0b10100, 0b0000, "one"),
    (0b10100, 0b0000, "won"),
    // Homophone pairs reusing existing chords
    (0b00001, 0b1111, "ok"),
    (0b00001, 0b1111, "okay"),
    (0b01100, 0b1110, "seen"),
    (0b01100, 0b1110, "scene"),
    (0b01101, 0b1101, "knows"),
    (0b01101, 0b1101, "nose"),
    (0b01110, 0b1011, "damn"),
    (0b01110, 0b1011, "dam"),
    // Colour-word homophone pairs
    (0b00111, 0b1001, "colour"),
    (0b00111, 0b1001, "color"),
    (0b01111, 0b1111, "blue"),
    (0b01111, 0b1111, "blew"),
    (0b10011, 0b1000, "green"),
    (0b10011, 0b1000, "greene"),
    (0b10010, 0b1100, "grey"),
    (0b10010, 0b1100, "gray"),
    // Unreachable-homophone pairs (new chord allocations)
    (0b11011, 0b0010, "peace"),
    (0b11011, 0b0010, "piece"),
    (0b10010, 0b1011, "weather"),
    (0b10010, 0b1011, "whether"),
    (0b11011, 0b1000, "cell"),
    (0b11011, 0b1000, "sell"),
    (0b11000, 0b1011, "led"),
    (0b11000, 0b1011, "lead"),
    (0b11011, 0b0100, "role"),
    (0b11011, 0b0100, "roll"),
    (0b10100, 0b1011, "site"),
    (0b10100, 0b1011, "sight"),
    // Morphological pairs (base + suffix on one chord)
    (0b11001, 0b0000, "go"),
    (0b11001, 0b0000, "going"),
    (0b10011, 0b0010, "have"),
    (0b10011, 0b0010, "having"),
    (0b10001, 0b1100, "work"),
    (0b10001, 0b1100, "working"),
    // (thing in pinned thing-family; things reachable via thing+s suffix.)
    (0b10100, 0b0010, "year"),
    (0b10100, 0b0010, "years"),
    (0b10001, 0b1110, "take"),
    (0b10001, 0b1110, "taking"),
    (0b11100, 0b0100, "tell"),
    (0b11100, 0b0100, "telling"),
    (0b11010, 0b0000, "real"),
    (0b11010, 0b0000, "really"),
    (0b11101, 0b0001, "live"),
    (0b11101, 0b0001, "living"),
    (0b11010, 0b0011, "serious"),
    (0b11010, 0b0011, "seriously"),
    (0b11001, 0b1101, "your"),
    (0b11001, 0b1101, "yours"),
    (0b10101, 0b1101, "speak"),
    (0b10101, 0b1101, "speaking"),
    (0b11011, 0b1101, "long"),
    (0b11011, 0b1101, "longer"),
    (0b10110, 0b1110, "question"),
    (0b10110, 0b1110, "questions"),
    // (minute/minutes removed — chord now used by green/greene)
    (0b10000, 0b0110, "think"),
    (0b10000, 0b0110, "thinking"),
    (0b11011, 0b0000, "start"),
    (0b11011, 0b0000, "started"),
    (0b11111, 0b0100, "friend"),
    (0b11111, 0b0100, "friends"),
    (0b10010, 0b0101, "hand"),
    (0b10010, 0b0101, "hands"),
    (0b11110, 0b0001, "kill"),
    (0b11110, 0b0001, "killed"),
    (0b11110, 0b0011, "day"),
    (0b11110, 0b0011, "days"),
    (0b11101, 0b0100, "care"),
    (0b11101, 0b0100, "careful"),
    // (feel/feeling removed — chord 0b10111 is the thing-family base.)
    (0b11100, 0b1100, "play"),
    (0b11100, 0b1100, "playing"),
    (0b10010, 0b0001, "thank"),
    (0b10010, 0b0001, "thanks"),
    (0b10011, 0b1001, "kid"),
    (0b10011, 0b1001, "kids"),

    // Free morphological ride-alongs
    (0b10110, 0b0011, "call"),
    (0b10110, 0b0011, "called"),
    (0b11001, 0b0001, "guys"),
    (0b11001, 0b0001, "guy"),
    (0b10010, 0b1101, "turn"),
    (0b10010, 0b1101, "turned"),
    (0b10010, 0b1101, "turns"),
    (0b10010, 0b1101, "turning"),
    (0b10010, 0b1101, "turner"),
    (0b10101, 0b0011, "move"),
    (0b10101, 0b0011, "moving"),
    (0b10101, 0b0011, "moved"),
    (0b10101, 0b0011, "moves"),
    (0b00110, 0b0011, "love"),
    (0b00110, 0b0011, "loved"),
    (0b00110, 0b0011, "lovely"),
    (0b10111, 0b1100, "miss"),
    (0b10111, 0b1100, "missing"),
    (0b10111, 0b1100, "missed"),
    (0b10001, 0b1010, "change"),
    (0b10001, 0b1010, "changed"),
    (0b10001, 0b1010, "changes"),
    (0b10001, 0b1010, "changing"),
    (0b11111, 0b0000, "like"),
    (0b11111, 0b0000, "liked"),
    (0b11111, 0b0000, "likes"),
    (0b11111, 0b0000, "likely"),
    (0b11111, 0b0000, "liking"),
    (0b00101, 0b0100, "give"),
    (0b00101, 0b0100, "giving"),
    (0b00101, 0b0100, "gives"),
    (0b11101, 0b0010, "sounds"),
    (0b11101, 0b0010, "sound"),
    (0b10110, 0b1010, "close"),
    (0b10110, 0b1010, "closed"),
    (0b10110, 0b1010, "closer"),
    (0b10110, 0b1010, "closes"),
    (0b11110, 0b1010, "seems"),
    (0b11110, 0b1010, "seem"),
    (0b00111, 0b1111, "stop"),
    (0b00111, 0b1111, "stopped"),
    (0b00111, 0b1111, "stops"),
    (0b00111, 0b1111, "stopping"),
    (0b01100, 0b0010, "other"),
    (0b01100, 0b0010, "others"),
    (0b11101, 0b0111, "watch"),
    (0b11101, 0b0111, "watching"),
    (0b11101, 0b0111, "watched"),
    (0b00100, 0b0110, "look"),
    (0b00100, 0b0110, "looked"),
    (0b11010, 0b1100, "worry"),
    (0b11010, 0b1100, "worried"),
    (0b11010, 0b1100, "worrying"),
    (0b10010, 0b0111, "name"),
    (0b10010, 0b0111, "named"),
    (0b10010, 0b0111, "names"),
    (0b10110, 0b0101, "open"),
    (0b10110, 0b0101, "opened"),
    (0b10110, 0b0101, "opens"),
    (0b10110, 0b0101, "opening"),
    (0b01111, 0b0101, "talk"),
    (0b01111, 0b0101, "talked"),
    (0b01111, 0b0101, "talks"),
    (0b00111, 0b1000, "need"),
    (0b00111, 0b1000, "needed"),
    (0b00111, 0b1000, "needing"),
    (0b00011, 0b0100, "help"),
    (0b00011, 0b0100, "helping"),
    (0b00011, 0b0100, "helped"),
    (0b11010, 0b1000, "use"),
    (0b11010, 0b1000, "using"),
    (0b11010, 0b1000, "useful"),
    (0b01101, 0b1110, "office"),
    (0b01101, 0b1110, "officer"),
    (0b01101, 0b1110, "offices"),
    (0b00010, 0b1101, "head"),
    (0b00010, 0b1101, "heads"),
    (0b00010, 0b1101, "heading"),
    (0b00010, 0b1101, "headed"),
    (0b10110, 0b1011, "clear"),
    (0b10110, 0b1011, "clearly"),
    (0b10110, 0b1011, "clears"),
    (0b10110, 0b1011, "cleared"),
    (0b10110, 0b1011, "clearing"),
    (0b00111, 0b1010, "married"),
    (0b00111, 0b1010, "marry"),
    (0b11110, 0b1000, "stand"),
    (0b11110, 0b1000, "standing"),
    (0b11110, 0b1000, "stands"),
    (0b11001, 0b1000, "supposed"),
    (0b11001, 0b1000, "suppose"),
    (0b11101, 0b1001, "sit"),
    (0b11101, 0b1001, "sitting"),
    (0b11101, 0b1001, "sits"),
    (0b11010, 0b0100, "months"),
    (0b11010, 0b0100, "month"),
    (0b11011, 0b0110, "order"),
    (0b11011, 0b0110, "orders"),
    (0b11011, 0b0110, "ordered"),
    (0b01100, 0b1001, "drink"),
    (0b01100, 0b1001, "drinking"),
    (0b01100, 0b1001, "drinks"),
    (0b10011, 0b0001, "put"),
    (0b10011, 0b0001, "putting"),
    (0b10011, 0b0001, "puts"),
    (0b11110, 0b0010, "promise"),
    (0b11110, 0b0010, "promised"),
    (0b11110, 0b0010, "promises"),
    (0b11110, 0b0010, "promising"),
    (0b11010, 0b0101, "certainly"),
    (0b11010, 0b0101, "certain"),
    (0b10100, 0b1101, "hope"),
    (0b10100, 0b1101, "hoping"),
    (0b10100, 0b1101, "hopes"),
    (0b10100, 0b1101, "hoped"),
    (0b11001, 0b0110, "hard"),
    (0b11001, 0b0110, "hardly"),
    (0b11001, 0b0110, "harder"),
    (0b11001, 0b0010, "bring"),
    (0b11001, 0b0010, "bringing"),
    (0b11001, 0b0010, "brings"),
    (0b01010, 0b1110, "end"),
    (0b01010, 0b1110, "ended"),
    (0b01010, 0b1110, "ends"),
    (0b01110, 0b0111, "hold"),
    (0b01110, 0b0111, "holding"),
    (0b01110, 0b0111, "holds"),
    (0b01100, 0b0111, "feel"),
    (0b01100, 0b0111, "feels"),
    (0b00101, 0b1000, "listen"),
    (0b00101, 0b1000, "listening"),
    (0b00101, 0b1000, "listened"),
    (0b00011, 0b1011, "laughing"),
    (0b00011, 0b1011, "laugh"),
    (0b01101, 0b1111, "sleep"),
    (0b01101, 0b1111, "sleeping"),
    (0b01101, 0b1111, "sleeps"),
    (0b01011, 0b0010, "expect"),
    (0b01011, 0b0010, "expecting"),
    (0b01011, 0b0010, "expected"),
    (0b00111, 0b0110, "find"),
    (0b00111, 0b0110, "finding"),
    (0b00111, 0b0110, "finds"),
    (0b01100, 0b0100, "place"),
    (0b01100, 0b0100, "places"),
    (0b01100, 0b0100, "placed"),
    (0b11010, 0b1101, "building"),
    (0b11010, 0b1101, "build"),
    (0b11010, 0b1101, "buildings"),
    (0b11010, 0b1010, "report"),
    (0b11010, 0b1010, "reports"),
    (0b11010, 0b1010, "reporter"),
    (0b11010, 0b1010, "reported"),
    (0b01100, 0b1111, "second"),
    (0b01100, 0b1111, "seconds"),
    (0b01000, 0b1010, "stay"),
    (0b01000, 0b1010, "staying"),
    (0b10101, 0b0001, "lot"),
    (0b10101, 0b0001, "lots"),
    (0b10101, 0b1010, "break"),
    (0b10101, 0b1010, "breaking"),
    (0b10101, 0b1010, "breaks"),
    (0b01011, 0b1011, "return"),
    (0b01011, 0b1011, "returned"),
    (0b01011, 0b1011, "returning"),
    (0b01011, 0b1011, "returns"),
    (0b01011, 0b1111, "answer"),
    (0b01011, 0b1111, "answers"),
    (0b01011, 0b1111, "answering"),
    (0b01011, 0b1111, "answered"),
    (0b11010, 0b0110, "finally"),
    (0b11010, 0b0110, "final"),
    (0b00001, 0b0110, "let"),
    (0b00001, 0b0110, "letting"),
    (0b00001, 0b0110, "lets"),
    (0b11110, 0b1100, "decided"),
    (0b11110, 0b1100, "decide"),
    (0b11010, 0b1111, "feeling"),
    (0b11010, 0b1111, "feelings"),
    (0b01110, 0b1111, "big"),
    (0b01110, 0b1111, "bigger"),
    (0b00010, 0b1110, "way"),
    (0b00010, 0b1110, "ways"),
    (0b01010, 0b0101, "become"),
    (0b01010, 0b0101, "becomes"),
    (0b01010, 0b0101, "becoming"),
    (0b10111, 0b0101, "completely"),
    (0b10111, 0b0101, "complete"),
    (0b00101, 0b1101, "face"),
    (0b00101, 0b1101, "faces"),
    (0b00101, 0b1101, "facing"),
    (0b00101, 0b1101, "faced"),
    (0b01101, 0b1010, "dangerous"),
    (0b01101, 0b1010, "danger"),
    (0b10111, 0b1110, "interesting"),
    (0b10111, 0b1110, "interest"),
    (0b11100, 0b0011, "point"),
    (0b11100, 0b0011, "points"),
    (0b11100, 0b0011, "pointed"),
    (0b11100, 0b0011, "pointing"),
    (0b11110, 0b0000, "will"),
    (0b11110, 0b0000, "willing"),
    (0b11100, 0b1011, "personal"),
    (0b11100, 0b1011, "personally"),
    (0b11100, 0b1011, "personality"),
    (0b00110, 0b1110, "fuck"),
    (0b00110, 0b1110, "fucking"),
    (0b00110, 0b1110, "fucked"),
    (0b00110, 0b1110, "fucker"),
    (0b00110, 0b1110, "fucks"),
    (0b11111, 0b1100, "reason"),
    (0b11111, 0b1100, "reasons"),
    (0b11111, 0b1100, "reasonable"),
    (0b00100, 0b1010, "happy"),
    (0b00100, 0b1010, "happiness"),
    (0b00100, 0b1010, "happier"),
    (0b01010, 0b1101, "send"),
    (0b01010, 0b1101, "sending"),
    (0b01010, 0b1101, "sends"),
    (0b11110, 0b0100, "car"),
    (0b11110, 0b0100, "cars"),
    (0b00011, 0b1001, "leave"),
    (0b00011, 0b1001, "leaves"),
    (0b11000, 0b1010, "human"),
    (0b11000, 0b1010, "humans"),
    (0b11000, 0b1010, "humanity"),
    (0b00111, 0b1110, "idea"),
    (0b00111, 0b1110, "ideas"),
    (0b00001, 0b1010, "old"),
    (0b00001, 0b1010, "older"),
    (0b01111, 0b1001, "kind"),
    (0b01111, 0b1001, "kinds"),
    (0b01111, 0b1001, "kindness"),
    (0b01111, 0b1001, "kindly"),
    (0b01100, 0b1100, "number"),
    (0b01100, 0b1100, "numbers"),
    (0b11100, 0b0111, "perfect"),
    (0b11100, 0b0111, "perfectly"),
    (0b01001, 0b0100, "mean"),
    (0b01001, 0b0100, "meaning"),
    (0b00110, 0b1001, "sure"),
    (0b00110, 0b1001, "surely"),
    (0b10101, 0b1000, "matter"),
    (0b10101, 0b1000, "matters"),
    (0b01101, 0b0101, "secret"),
    (0b01101, 0b0101, "secrets"),
    (0b01101, 0b0101, "secretly"),
    (0b01000, 0b1001, "believe"),
    (0b01000, 0b1001, "believed"),
    (0b11011, 0b1100, "totally"),
    (0b11011, 0b1100, "total"),
    (0b01011, 0b1001, "late"),
    (0b01011, 0b1001, "lately"),
    (0b11101, 0b0101, "street"),
    (0b11101, 0b0101, "streets"),
    (0b01001, 0b1010, "door"),
    (0b01001, 0b1010, "doors"),
    (0b11111, 0b1000, "thought"),
    (0b11111, 0b1000, "thoughts"),
    (0b01000, 0b1011, "soon"),
    (0b01000, 0b1011, "sooner"),
    (0b10100, 0b1001, "doctor"),
    (0b10100, 0b1001, "doctors"),
    (0b00111, 0b1011, "protect"),
    (0b00111, 0b1011, "protecting"),
    (0b00111, 0b1011, "protected"),
    (0b01101, 0b0111, "run"),
    (0b01101, 0b0111, "runs"),
    (0b01011, 0b1110, "obviously"),
    (0b01011, 0b1110, "obvious"),
    (0b01000, 0b0001, "good"),
    (0b01000, 0b0001, "goodness"),
    (0b11001, 0b0011, "part"),
    (0b11001, 0b0011, "parts"),
    (0b01100, 0b1101, "case"),
    (0b01100, 0b1101, "cases"),
    (0b10101, 0b0111, "job"),
    (0b10101, 0b0111, "jobs"),
    (0b11010, 0b1001, "scared"),
    (0b11010, 0b1001, "scare"),
    (0b01111, 0b0100, "please"),
    (0b01111, 0b0100, "pleased"),
    (0b10001, 0b1011, "young"),
    (0b10001, 0b1011, "younger"),
    (0b11010, 0b0010, "explain"),
    (0b11010, 0b0010, "explained"),
    (0b11010, 0b0010, "explains"),
    (0b11010, 0b0010, "explaining"),
    (0b00010, 0b1111, "remember"),
    (0b00010, 0b1111, "remembered"),
    (0b00010, 0b1111, "remembers"),
    (0b01110, 0b1010, "million"),
    (0b01110, 0b1010, "millions"),
    (0b00010, 0b1010, "bad"),
    (0b00010, 0b1010, "badly"),
    (0b11001, 0b0111, "heart"),
    (0b11001, 0b0111, "hearts"),
    (0b11111, 0b1111, "most"),
    (0b11111, 0b1111, "mostly"),
    (0b11101, 0b1100, "strange"),
    (0b11101, 0b1100, "stranger"),
    (0b10010, 0b1110, "room"),
    (0b10010, 0b1110, "rooms"),
    (0b11110, 0b1101, "present"),
    (0b11110, 0b1101, "presents"),
    (0b11110, 0b1101, "presented"),
    (0b11111, 0b1011, "mistake"),
    (0b11111, 0b1011, "mistakes"),
    (0b10100, 0b0101, "house"),
    (0b10100, 0b0101, "houses"),
    (0b10100, 0b0110, "exactly"),
    (0b10100, 0b0110, "exact"),
    (0b00011, 0b1101, "phone"),
    (0b00011, 0b1101, "phones"),
    (0b00011, 0b1101, "phoned"),
    (0b11001, 0b1100, "chance"),
    (0b11001, 0b1100, "chances"),
    (0b11111, 0b0111, "moment"),
    (0b11111, 0b0111, "moments"),
    (0b10101, 0b0100, "sometimes"),
    (0b10101, 0b0100, "sometime"),
    (0b00010, 0b0100, "understand"),
    (0b00010, 0b0100, "understanding"),
    (0b01110, 0b0000, "want"),
    (0b01110, 0b0000, "wanting"),
    (0b11011, 0b1111, "fact"),
    (0b11011, 0b1111, "facts"),
    (0b10111, 0b0011, "trouble"),
    (0b10111, 0b0011, "troubles"),
    (0b10111, 0b0011, "troubled"),
    (0b00011, 0b0110, "actually"),
    (0b00011, 0b0110, "actual"),
    (0b01110, 0b1100, "security"),
    (0b01110, 0b1100, "secure"),
    (0b01101, 0b1100, "experience"),
    (0b01101, 0b1100, "experienced"),
    (0b01101, 0b1100, "experiences"),
    (0b11111, 0b0110, "mind"),
    (0b11111, 0b0110, "minds"),
    (0b01100, 0b1010, "guess"),
    (0b01100, 0b1010, "guessing"),
    (0b01100, 0b1010, "guessed"),
    (0b01001, 0b1001, "nice"),
    (0b01001, 0b1001, "nicely"),
    (0b01001, 0b1001, "nicer"),
    (0b11011, 0b1110, "quiet"),
    (0b11011, 0b1110, "quietly"),
    (0b00100, 0b1101, "trust"),
    (0b00100, 0b1101, "trusted"),
    (0b01010, 0b0110, "ask"),
    (0b01010, 0b0110, "asks"),
    (0b00100, 0b0101, "great"),
    (0b00100, 0b0101, "greater"),
    (0b00001, 0b1101, "control"),
    (0b00001, 0b1101, "controlled"),
    (0b00001, 0b1101, "controls"),
    (0b11001, 0b1011, "system"),
    (0b11001, 0b1011, "systems"),
    (0b00100, 0b0010, "time"),
    (0b00100, 0b0010, "timing"),
    (0b10110, 0b0110, "home"),
    (0b10110, 0b0110, "homes"),
    (0b00010, 0b1100, "being"),
    (0b00010, 0b1100, "beings"),
    (0b11001, 0b1111, "absolutely"),
    (0b11001, 0b1111, "absolute"),
    (0b10011, 0b0111, "school"),
    (0b10011, 0b0111, "schools"),
    (0b01011, 0b1101, "imagine"),
    (0b01011, 0b1101, "imagined"),
    (0b11100, 0b0010, "parents"),
    (0b11100, 0b0010, "parent"),
    (0b11100, 0b1111, "water"),
    (0b11100, 0b1111, "waters"),
    (0b10101, 0b1100, "relationship"),
    (0b10101, 0b1100, "relationships"),
    (0b10000, 0b1011, "different"),
    (0b10000, 0b1011, "differently"),
    (0b01110, 0b0101, "dead"),
    (0b01110, 0b0101, "deadly"),
    (0b00111, 0b0101, "music"),
    (0b00111, 0b0101, "musical"),
    (0b00110, 0b1101, "bit"),
    (0b00110, 0b1101, "bits"),
    (0b11011, 0b0001, "meeting"),
    (0b11011, 0b0001, "meetings"),
    (0b10011, 0b1111, "forget"),
    (0b10011, 0b1111, "forgetting"),
    (0b00101, 0b1011, "immediately"),
    (0b00101, 0b1011, "immediate"),
    (0b11101, 0b1111, "detective"),
    (0b11101, 0b1111, "detectives"),
    (0b01101, 0b1011, "continue"),
    (0b01101, 0b1011, "continued"),
    (0b10001, 0b0011, "world"),
    (0b10001, 0b0011, "worlds"),
    (0b01100, 0b1011, "death"),
    (0b01100, 0b1011, "deaths"),
    (0b11000, 0b0010, "excuse"),
    (0b11000, 0b0010, "excuses"),
    (0b11110, 0b0110, "stuff"),
    (0b11110, 0b0110, "stuffed"),
    (0b11000, 0b1001, "person"),
    (0b01001, 0b1110, "anyway"),
    (0b01001, 0b1110, "anyways"),
    (0b11010, 0b0001, "couple"),
    (0b11010, 0b0001, "couples"),
    (0b10000, 0b1101, "back"),
    (0b10000, 0b1101, "backs"),
    (0b01001, 0b1011, "general"),
    (0b01001, 0b1011, "generally"),
    (0b10001, 0b1101, "situation"),
    (0b10001, 0b1101, "situations"),
    (0b11110, 0b1001, "rest"),
    (0b11110, 0b1001, "resting"),
    (0b11000, 0b1100, "hospital"),
    (0b11000, 0b1100, "hospitals"),
    (0b01110, 0b0001, "important"),
    (0b01110, 0b0001, "importantly"),
    (0b00101, 0b0010, "beautiful"),
    (0b00101, 0b0010, "beautifully"),
    // Consolidated families
    (0b10110, 0b1100, "try"),
    (0b10110, 0b1100, "trying"),
    (0b10110, 0b1100, "tried"),
    (0b10110, 0b1100, "tries"),
    (0b10110, 0b0111, "minute"),
    (0b10110, 0b0111, "minutes"),
    (0b00011, 0b0111, "problem"),
    (0b00011, 0b0111, "problems"),
];

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cmu_path = project.join("data/cmudict.dict");
    let freq_path = project.join("data/en_freq.txt");
    let candidates_path = project.join("data/brief_candidates.txt");
    let homophones_path = project.join("data/homophones.txt");
    let out_path = project.join("src/layout/en/briefs.rs");

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

    // 3. Take top 2048 words that exist in CMU.
    //    The 470 assigned briefs are saturated well before rank 1000,
    //    but each blacklist edit cascades — removing one candidate
    //    pulls the next-ranked word into that slot, which may pull
    //    the next into its slot, and so on. Keeping ~4x headroom
    //    ensures those shifts always have a replacement ready without
    //    forcing another pool expansion.
    //    Skip apostrophe words and contraction fragments.
    let mut top_words: Vec<(String, u64, Vec<String>)> = Vec::new();
    for (word, count) in &freq_words {
        if top_words.len() >= 2048 {
            break;
        }
        if word.contains('\'') {
            continue;
        }
        // Skip contraction fragments (high-freq only because of "don't", "won't", etc.)
        // "won" is intentionally NOT here — it's a real word (past tense of
        // "win", homophone of "one"), not just a fragment of "won't".
        const FRAGMENTS: &[&str] = &[
            "don", "doesn", "didn", "wasn", "weren", "isn", "wouldn", "couldn", "shouldn",
            "hasn", "hadn", "ain", "aren", "mustn",
        ];
        if FRAGMENTS.contains(&word.as_str()) {
            continue;
        }
        // Skip proper nouns (names from subtitle corpus)
        const NAMES: &[&str] = &[
            "jesus",
            "michael",
            "david",
            "frank",
            "charlie",
            "jack",
            "john",
            "george",
            "sam",
            "harry",
            "joe",
            "tom",
            "bob",
            "henry",
            "alex",
            "nick",
            "max",
            "ben",
            "dan",
            "tony",
            "tommy",
            "jimmy",
            "johnny",
            "bobby",
            "danny",
            "brian",
            "mary",
            "sarah",
            "anna",
            "elizabeth",
            "peter",
            "james",
            "paul",
            "richard",
            "robert",
            "bill",
            "mike",
            "ray",
            "eddie",
            "leo",
            "steve",
            "chris",
            "matt",
            "mark",
            "scott",
            "eric",
            "grace",
            "emma",
            "kate",
            "rachel",
            "sophie",
            "lily",
        ];
        if NAMES.contains(&word.as_str()) {
            continue;
        }
        if let Some(phs) = cmu.get(word) {
            top_words.push((word.clone(), *count, phs.clone()));
        }
    }
    eprintln!("Selected {} words for brief assignment", top_words.len());

    // 3b. (No inflection filter.) Past iterations stripped inflected
    // forms whose base word was already in the list (e.g. "looking"
    // dropped because "look" was present). In practice too many
    // high-frequency irregulars share almost no phonemes with their
    // base — "was" vs "be", "thought" vs "think", "better" vs "good"
    // — so filtering cost real keystrokes for no gain. Cheaper to
    // rely on `data/brief_candidates.txt` pruning for the rare cases
    // where the base + suffix path really does dominate.

    // 3c. Candidate curation.
    //
    // If `data/brief_candidates.txt` exists, treat it as the user-curated
    // source of truth: only words listed in it get assigned briefs. That
    // file is what the user edits — `#`-out a line to blacklist that word.
    // The candidate file is rewritten each run with refreshed ranks/values
    // and inline `+suffix→base(freq) tag` annotations next to any word
    // that decomposes as `base + <SUFFIX>`. The user's exclusions and any
    // custom-added words survive the rewrite; inline user notes do not.
    let freq_map: HashMap<String, u64> = freq_words.iter().cloned().collect();
    let excludes_path = project.join("data/brief_excludes.txt");
    let excludes = read_excludes_file(&excludes_path);
    if !excludes.is_empty() {
        eprintln!(
            "Loaded {} sidecar excludes from {}",
            excludes.len(),
            excludes_path.display()
        );
    }
    top_words = refresh_candidate_file(
        &candidates_path,
        &top_words,
        &cmu,
        &freq_map,
        ORDERED_CLAIMED,
        &excludes,
    );
    eprintln!(
        "Using {} candidate words from {}",
        top_words.len(),
        candidates_path.display()
    );

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
        /// Savings-weighted value: `frequency × (phonemes - 1)`. Actual keystroke savings regardless of slot type.
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

    // Hand-pinned brief slots. These claim a specific chord before the greedy
    // pass runs, so the chord is locked to this word regardless of value rank.
    // Format: (right_5bits, left_4bits, word).
    // Hand-pinned brief slots. These claim a specific chord before the greedy
    // pass runs, so the chord is locked to this word regardless of value rank.
    // Format: (right_5bits, left_4bits, word).
    //
    // Compound family — `thing` and its prefix-modified compounds.
    // Base chord = R-IDX + R-MID + R-RING + R-thumb (0b10111). Compounds
    // add a left-finger modifier per prefix, building muscle memory across
    // every `prefix-X` family (one, body, where, way, ...) when those are
    // pinned with the same prefix→finger convention:
    //   some-  → L-MID    any-   → L-RING
    //   no-    → L-PINKY  every- → L-IDX
    let pinned: &[(u8, u8, &str)] = &[
        // thing-family — base chord (R-IDX+R-MID+R-RING+R-thumb)
        (0b10111, 0b0000, "thing"),
        (0b10111, 0b0010, "something"),
        (0b10111, 0b0100, "anything"),
        (0b10111, 0b1000, "nothing"),
        (0b10111, 0b0001, "everything"),
        // one-family — base chord = R-RING+R-thumb (also in ordered_briefs
        // for one/won homophone bundle). Compounds extend with the same
        // prefix-modifier convention. No `noone` (it's two words "no one").
        (0b10100, 0b0010, "someone"),
        (0b10100, 0b0100, "anyone"),
        (0b10100, 0b0001, "everyone"),
        // three — center 6 fingers (idx+mid+ring on both hands, no thumbs,
        // no pinkies). Symmetric, near digit-3's number-mode finger
        // (R-IDX), and parallel to the digit-word brief slots for
        // one (R-RING+R-thumb), two (R-IDX+R-MID+R-thumb), four (all 4
        // right fingers). Auto-assignment had given "three" a 5-finger
        // chord that didn't fit the family.
        (0b00111, 0b0111, "three"),
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
        let group = if is_pinned {
            0u32
        } else if is_right_only {
            1
        } else {
            2
        };
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

    fs::write(&out_path, &out).expect("cannot write briefs.rs");
    eprintln!(
        "Wrote {} briefs to {}",
        assignments.len(),
        out_path.display()
    );
}

/// Suffix list mirroring `src/layout/suffixes.rs`. KEEP IN SYNC.
///
/// "'s" omitted — it's a possessive marker, not a productive derivational suffix, and `interest's` isn't competing with `interest` for a brief slot. Also "s" included even though plurals dominate, because that's exactly the point: plurals shouldn't eat brief slots when the user can just hit the s-suffix chord after the base.
const SUFFIX_LIST: &[&str] = &[
    "ing", "ed", "ly", "er", "tion", "al", "ment", "ness", "able", "ive", "ful", "ous", "ity", "s",
];

/// True if `base`'s phonemes are a strict prefix of `word`'s phonemes. Filters spelling-coincidence false positives like `is` → `i` (phonemes don't align: /IH Z/ vs /AY/) from real morphological matches like `going` → `go` (/G OW IH NG/ starts with /G OW/).
fn base_is_phoneme_prefix_of(
    base: &str,
    word: &str,
    cmu: &HashMap<String, Vec<String>>,
) -> bool {
    let (Some(base_phs), Some(word_phs)) = (cmu.get(base), cmu.get(word)) else {
        return false;
    };
    let base_clean: Vec<&str> = base_phs.iter().map(|p| strip_stress(p)).collect();
    let word_clean: Vec<&str> = word_phs.iter().map(|p| strip_stress(p)).collect();
    word_clean.len() > base_clean.len() && word_clean[..base_clean.len()] == base_clean[..]
}

/// If `word` ends in one of `SUFFIX_LIST` AND the resulting base is in CMU dict AND the base's phonemes are a prefix of the word's phonemes, return the base form. Tries the direct strip first, then a few common spelling-rule restorations:
///   - drop trailing 'e' before ing/ed (`making` → `make`, `loved` → `love`)
///   - undouble final consonant before ing/ed (`running` → `run`, `stopped` → `stop`)
///   - y → i before s/ed/ly (`studies` → `study`, `studied` → `study`, `happily` → `happy`)
/// Returns the longest-suffix match (so `interesting` resolves via `ing` not via `s`).
fn try_strip_suffix(word: &str, cmu: &HashMap<String, Vec<String>>) -> Option<(String, &'static str)> {
    let mut sorted = SUFFIX_LIST.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.len()));
    let check = |candidate: &str, suffix: &'static str| -> Option<(String, &'static str)> {
        if cmu.contains_key(candidate) && base_is_phoneme_prefix_of(candidate, word, cmu) {
            Some((candidate.to_string(), suffix))
        } else {
            None
        }
    };
    for suffix in sorted {
        let Some(stem) = word.strip_suffix(suffix) else {
            continue;
        };
        if stem.is_empty() {
            continue;
        }
        if let Some(hit) = check(stem, suffix) {
            return Some(hit);
        }
        if suffix == "ing" || suffix == "ed" {
            // make + ing → making; love + ed → loved
            let with_e = format!("{}e", stem);
            if let Some(hit) = check(&with_e, suffix) {
                return Some(hit);
            }
            // run + ing → running (last char doubled)
            let bytes = stem.as_bytes();
            if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                let undoubled = &stem[..stem.len() - 1];
                if let Some(hit) = check(undoubled, suffix) {
                    return Some(hit);
                }
            }
        }
        if (suffix == "ed" || suffix == "ly" || suffix == "s") && stem.ends_with('i') {
            // studi(ed) → study; happi(ly) → happy; studi(es) → study
            let with_y = format!("{}y", &stem[..stem.len() - 1]);
            if let Some(hit) = check(&with_y, suffix) {
                return Some(hit);
            }
        }
    }
    None
}

/// Read `data/brief_excludes.txt` (if present), returning a `word → category` map. File format: section headers like `[gender]` or `[religion]`, followed by one word per line. Categories whose header is commented (starts with `#[`) are ignored. Words in active categories will be auto-`#`'d in `brief_candidates.txt` with reason `excluded: <category>`.
fn read_excludes_file(path: &Path) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    if !path.exists() {
        return out;
    }
    let content = fs::read_to_string(path).expect("cannot read brief_excludes.txt");
    let mut current_category: Option<String> = None;
    let mut category_active = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Active section header: `[name]`
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_category = Some(trimmed[1..trimmed.len() - 1].to_string());
            category_active = true;
            continue;
        }
        // Disabled section header: `#[name]`
        if trimmed.starts_with("#[") && trimmed.ends_with(']') {
            current_category = Some(trimmed[2..trimmed.len() - 1].to_string());
            category_active = false;
            continue;
        }
        if trimmed.starts_with('#') {
            continue; // pure comment
        }
        if category_active {
            if let Some(cat) = &current_category {
                let word = trimmed.to_lowercase();
                out.insert(word, cat.clone());
            }
        }
    }
    out
}

/// Extract the user-typed portion of a comment column. Auto-reasons (recognisable by their leading keyword) get stripped; whatever's left is the user note. Convention for explicit auto/user separation is ` ; ` — anything after that is unambiguously user. Without `;`, we check whether the comment starts with a known auto-reason keyword: if so, no user note. Otherwise the whole comment IS the user note (catches legacy bare-paren notes like `# (number-mode gesture)`).
fn extract_user_note(comment: &str) -> String {
    let trimmed = comment.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(idx) = trimmed.find(" ; ") {
        return trimmed[idx + 3..].trim().to_string();
    }
    let auto_starts = ["excluded:", "ordered brief", "homophone of"];
    if auto_starts.iter().any(|p| trimmed.starts_with(p)) || trimmed.starts_with('+') {
        return String::new();
    }
    trimmed.to_string()
}

/// Group CMU words by phoneme sequence and report collisions where at least one member is in the candidate pool. Output goes to `data/homophones.txt` for the user to browse and decide which pairs warrant ordered-brief entries in `src/layout/ordered_briefs.rs`.
///
/// Only words that appear in `en_freq.txt` are included (filters out obscure CMU entries that would otherwise dominate the report). Groups are sorted by max member frequency descending.
fn write_homophone_report(
    path: &Path,
    candidates: &[(String, u64, Vec<String>)],
    cmu: &HashMap<String, Vec<String>>,
    freq_words: &[(String, u64)],
) {
    let seq_of = |phs: &[String]| -> String {
        phs.iter()
            .map(|p| strip_stress(p))
            .collect::<Vec<_>>()
            .join(" ")
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
    let mut reportable: Vec<(String, Vec<(String, u64)>)> =
        groups.into_iter().filter(|(_, ws)| ws.len() >= 2).collect();
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
         # pick candidates for ordered briefs (src/layout/ordered_briefs.rs).\n\
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

/// Read `data/brief_candidates.txt` (if any), refresh it with up-to-date ranks/values and inline suffix-collision annotations, then return the candidate list to use for assignment.
///
/// User state preserved across rewrites:
///   - `#`-prefixed lines stay excluded (the only durable user signal — `#` is "I don't want this word as a brief").
///   - Custom user-added words (anything not in `defaults`) survive as a "Custom user-added words" section at the end of the file.
///
/// User state NOT preserved: inline user comments after a word. The header explains; users wanting durable notes should keep them outside this file.
///
/// File format per line: `[#] rank  value  phonemes  word  [# +<suffix>→<base>(<base_freq>) <tag>]` where `<tag>` is `defer`, `keep`, or empty (within-2× judgment call). Lines starting with `#` are excluded; the word is the last whitespace token before any inline `#`.
fn refresh_candidate_file(
    path: &Path,
    defaults: &[(String, u64, Vec<String>)],
    cmu: &HashMap<String, Vec<String>>,
    freq_map: &HashMap<String, u64>,
    ordered_claimed: &[(u8, u8, &str)],
    excludes: &HashMap<String, String>,
) -> Vec<(String, u64, Vec<String>)> {
    let defaults_set: HashSet<String> = defaults.iter().map(|(w, _, _)| w.clone()).collect();
    let ordered_set: HashSet<String> =
        ordered_claimed.iter().map(|(_, _, w)| (*w).to_string()).collect();

    // Phase 1: parse existing file. State preserved across regens:
    //   - `#`-excluded lines (durable user signal)
    //   - custom user-added words (anything not in defaults)
    //   - user notes — anything in the comment column AFTER the auto-reasons,
    //     separated by ` ; `. Pure-user notes (no auto reason) survive too:
    //     `# my note` with no `excluded:`, `+suffix→`, `homophone of`, or
    //     `ordered brief` prefix is treated as a pure user note.
    //
    // The new column order puts the word FIRST, but old files have rank
    // first and word last. Handle both: if the first token looks like a
    // word (alphabetic), use it; otherwise fall back to the last token
    // before any inline `#`.
    let mut excluded: HashSet<String> = HashSet::new();
    let mut custom: Vec<String> = Vec::new();
    let mut custom_set: HashSet<String> = HashSet::new();
    let mut user_notes: HashMap<String, String> = HashMap::new();
    let mut in_custom_section = false;
    if path.exists() {
        let content = fs::read_to_string(path).expect("cannot read brief_candidates.txt");
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // The "Custom user-added words" marker switches us into
            // single-token-acceptable mode. Before this marker, single-word
            // lines are header text — reject them so leaked header words
            // (e.g., `defer`, `omitted`, `whitespace`) don't end up in the
            // custom set.
            if trimmed.contains("Custom user-added words") {
                in_custom_section = true;
                continue;
            }
            let is_excluded = trimmed.starts_with('#');
            let body = if is_excluded {
                trimmed.trim_start_matches('#').trim()
            } else {
                trimmed
            };
            // Split body at the first `#` to separate the columns from any
            // trailing comment text.
            let (before_comment, comment) = match body.find('#') {
                Some(i) => (body[..i].trim(), Some(body[i + 1..].trim())),
                None => (body, None),
            };
            let is_alpha = |s: &str| s.chars().all(|c| c.is_ascii_alphabetic() || c == '\'');
            let tokens: Vec<&str> = before_comment.split_whitespace().collect();
            let word_str = match tokens.first() {
                Some(first) if is_alpha(first) => Some(first.to_string()),
                _ => {
                    // Old format: word is the last whitespace token before
                    // any inline `#`.
                    tokens.last().map(|s| s.to_string())
                }
            };
            let Some(word) = word_str else {
                continue;
            };
            let word = word.to_lowercase();
            if !is_alpha(&word) {
                continue;
            }
            // Validate line shape: a real candidate line is either column-
            // formatted (word + numeric value/phc/rank columns) or a single
            // token in the custom section. Anything else is header text.
            let is_column_formatted = tokens.len() >= 4
                && tokens[1..]
                    .iter()
                    .take(3)
                    .all(|t| t.chars().all(|c| c.is_ascii_digit()));
            let is_old_format_line = tokens.len() >= 4
                && tokens[..tokens.len() - 1]
                    .iter()
                    .all(|t| t.chars().all(|c| c.is_ascii_digit()));
            let is_lone_word = tokens.len() == 1;
            if !is_column_formatted && !is_old_format_line && !(is_lone_word && in_custom_section) {
                continue;
            }
            if is_excluded {
                excluded.insert(word.clone());
            }
            if !defaults_set.contains(&word) && custom_set.insert(word.clone()) {
                custom.push(word.clone());
            }
            // Extract any user note from the comment.
            if let Some(comment) = comment {
                let note = extract_user_note(comment);
                if !note.is_empty() {
                    user_notes.insert(word, note);
                }
            }
        }
    }

    // Phase 2: score defaults by savings-weighted value.
    let mut scored: Vec<(&(String, u64, Vec<String>), u64, usize)> = defaults
        .iter()
        .filter_map(|entry| {
            let phc = entry
                .2
                .iter()
                .map(|p| strip_stress(p))
                .filter(|p| is_consonant_phoneme(p) || is_vowel_phoneme(p))
                .count();
            if phc < 2 {
                return None;
            }
            Some((entry, entry.1 * (phc - 1) as u64, phc))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    // Phase 3: pre-compute homophone winners. For each phoneme sequence,
    // find the highest-frequency word in CMU; if a candidate word ISN'T
    // that winner, the phoneme path will emit the winner instead and this
    // word would only be reachable via its own brief.
    let seq_of = |phs: &[String]| -> String {
        phs.iter()
            .map(|p| strip_stress(p))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut seq_winner: HashMap<String, (String, u64)> = HashMap::new();
    for (word, phs) in cmu {
        let Some(&freq) = freq_map.get(word) else {
            continue;
        };
        if freq == 0 {
            continue;
        }
        let seq = seq_of(phs);
        seq_winner
            .entry(seq)
            .and_modify(|e| {
                if freq > e.1 {
                    *e = (word.clone(), freq);
                }
            })
            .or_insert((word.clone(), freq));
    }

    // Phase 4: format the comment column = auto reasons + preserved user
    // notes. Auto reasons regenerate each run (suffix collision, homophone,
    // ordered-brief slot). User notes (if present) survive verbatim,
    // separated by ` ; ` from any auto reasons.
    let format_reason = |word: &str, inflected_freq: u64, phs: &[String]| -> String {
        let mut reasons: Vec<String> = Vec::new();

        // Sidecar excludes file category match: auto-`#` and tag with the
        // category so the user can see WHY it's excluded (and adjust the
        // sidecar file if they want to opt back in).
        if let Some(category) = excludes.get(word) {
            reasons.push(format!("excluded: {}", category));
        }

        // Already covered by an ordered brief: the chord slot is locked
        // and the word is excluded from regular brief assignment. Listing
        // it here is purely informational so the user can see WHY it
        // doesn't get a normal slot.
        if ordered_set.contains(word) {
            reasons.push("ordered brief (slot already claimed)".to_string());
        }

        // Suffix collision: word decomposes as base + known suffix.
        if let Some((base, suffix)) = try_strip_suffix(word, cmu) {
            if let Some(&base_freq) = freq_map.get(&base) {
                if base_freq > 0 {
                    let tag = if inflected_freq >= 2 * base_freq {
                        " keep"
                    } else if base_freq >= 2 * inflected_freq {
                        " defer"
                    } else {
                        ""
                    };
                    reasons.push(format!("+{}→{}({}){}", suffix, base, base_freq, tag));
                }
            }
        }

        // Homophone: another higher-frequency word shares this phoneme
        // sequence, so phoneme-path emits THAT word, not this one.
        let seq = seq_of(phs);
        if let Some((winner, winner_freq)) = seq_winner.get(&seq) {
            if winner != word {
                reasons.push(format!("homophone of \"{}\"({})", winner, winner_freq));
            }
        }

        let auto = reasons.join(", ");
        let user = user_notes.get(word).map(String::as_str).unwrap_or("");
        match (auto.is_empty(), user.is_empty()) {
            (true, true) => String::new(),
            (false, true) => format!("  # {}", auto),
            (true, false) => format!("  # {}", user),
            (false, false) => format!("  # {} ; {}", auto, user),
        }
    };

    // Phase 5: emit. Column order: word, value, phonemes, rank, reason.
    let mut text = String::new();
    text.push_str(
        "# Brief candidates for rhe.\n\
         #\n\
         # Comment a line out (`#` at line start) to exclude that word from\n\
         # brief assignment — the freed slot goes to the next-ranked word on\n\
         # the next gen_briefs run. The reason column tells you what gen_briefs\n\
         # noticed about each word; an empty reason means no auto signal (a\n\
         # `#`'d line with empty reason is something you decided manually).\n\
         #\n\
         # Reasons gen_briefs emits:\n\
         #\n\
         #   ordered brief (slot already claimed)\n\
         #       Word is in `ordered_briefs.rs` already. The chord slot is\n\
         #       locked and the word is excluded from regular brief\n\
         #       assignment. Listed here only so its rank/value is visible.\n\
         #\n\
         #   +<suffix>→<base>(<base_freq>) <tag>\n\
         #       Word decomposes as `base + <SUFFIX>` (the suffix is in the\n\
         #       SUFFIXES table) and `base` is in the frequency table, so\n\
         #       you could reach this word via the suffix chord on `base`.\n\
         #       Tags:\n\
         #         defer — base freq ≥ 2× this word's freq (likely\n\
         #                 redundant; consider commenting out)\n\
         #         keep  — this word's freq ≥ 2× base (auxiliary or\n\
         #                 dominant; brief slot earns its keep)\n\
         #         (none) — within 2×, judgment call\n\
         #\n\
         #   homophone of \"<word>\"(<freq>)\n\
         #       Same phoneme sequence as <word>. Phoneme-path emits the\n\
         #       higher-freq <word>, so this word only fires via its own\n\
         #       brief — even with a slot, you'll often type the homophone\n\
         #       instead. Promote to ordered_briefs.rs if you want both\n\
         #       reachable from the same chord with finger-order disambig.\n\
         #\n\
         # Multiple auto reasons are comma-separated. Auto reasons regenerate\n\
         # each run.\n\
         #\n\
         # User notes: anything you type after ` ; ` (separator: space-semi-space)\n\
         # in the comment column is preserved verbatim across regens. On a line\n\
         # with no auto reason you can also write `# my note` directly.\n\
         # Examples:\n\
         #   #one     ...    # excluded: number_mode_alias ; legacy reason\n\
         #   #up      ...    # personal preference, too short to bother\n\
         #\n\
         # value = frequency × (phonemes - 1). Single-phoneme words are\n\
         # omitted (a brief saves nothing over typing the one chord).\n\
         #\n\
         # Adding a custom word: append a line with the word as the FIRST\n\
         # whitespace token. Custom words, `#` exclusions, and user notes\n\
         # survive regens; everything else (rank, value, phonemes, auto\n\
         # reasons) refreshes.\n\
         #\n\
         # Fields: word  value  phonemes  rank  [# auto-reason(s) [ ; user-note]]\n\
         #\n",
    );

    let mut emitted_excluded: HashSet<String> = HashSet::new();
    for (i, (entry, value, phc)) in scored.iter().enumerate() {
        let word = &entry.0;
        let is_excluded = excluded.contains(word) || excludes.contains_key(word);
        if is_excluded {
            emitted_excluded.insert(word.clone());
        }
        let prefix = if is_excluded { "#" } else { " " };
        let reason = format_reason(word, entry.1, &entry.2);
        text.push_str(&format!(
            "{}{:<15}  {:>14}  {:>3}  {:>5}{}\n",
            prefix,
            word,
            value,
            phc,
            i + 1,
            reason
        ));
    }

    if !custom.is_empty() {
        text.push_str("\n# ---- Custom user-added words ----\n");
        for word in &custom {
            let is_excluded = excluded.contains(word) || excludes.contains_key(word);
            if is_excluded {
                emitted_excluded.insert(word.clone());
            }
            let prefix = if is_excluded { "#" } else { " " };
            let phs = cmu.get(word).cloned().unwrap_or_default();
            let inflected_freq = freq_map.get(word).copied().unwrap_or(0);
            let reason = format_reason(word, inflected_freq, &phs);
            text.push_str(&format!("{}{}{}\n", prefix, word, reason));
        }
    }

    // Surface `#`-excluded words that fell out of the candidate pool. Keep
    // them around so re-entry re-applies the exclusion.
    let stale_excluded: Vec<&String> = excluded
        .iter()
        .filter(|w| !emitted_excluded.contains(*w))
        .collect();
    if !stale_excluded.is_empty() {
        text.push_str(
            "\n# ---- Previously-excluded words no longer in the candidate pool ----\n\
             # Kept here so re-entering the pool re-applies the exclusion.\n",
        );
        for word in stale_excluded {
            text.push_str(&format!("#{}\n", word));
        }
    }

    fs::write(path, text).expect("cannot write brief_candidates.txt");

    // Phase 6: build the return list (active candidates + custom, minus excluded
    // and minus ordered-claimed words which already have a permanent home).
    let mut out: Vec<(String, u64, Vec<String>)> = Vec::new();
    for (entry, _, _) in scored {
        if excluded.contains(&entry.0) || ordered_set.contains(&entry.0) || excludes.contains_key(&entry.0) {
            continue;
        }
        out.push(entry.clone());
    }
    for word in &custom {
        if excluded.contains(word) || ordered_set.contains(word) || excludes.contains_key(word) {
            continue;
        }
        let Some(phs) = cmu.get(word) else {
            eprintln!("warning: custom word '{}' not in CMU dict — skipping", word);
            continue;
        };
        let count = freq_map.get(word).copied().unwrap_or(0);
        out.push((word.clone(), count, phs.clone()));
    }
    out
}

/// Find the nearest unoccupied slot to `target`. Priority: same right (consonant) different left, then same left different right, then fallback to any by effort.
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
    all_slots.iter().find(|s| !occupied.contains(s)).copied()
}
