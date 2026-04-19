mod briefs;
mod chord_map;
mod chord_state;
mod hand;
mod input;
mod interpreter;
mod output;
mod state_machine;
mod table_gen;
mod tray;
mod tutor;
mod word_lookup;

use input::KeyInput;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("generate") => generate_table(),
        Some("map") => show_map(),
        Some("briefs") => show_briefs(),
        Some("listen") => listen(),
        Some("run") => run(),
        Some("tutor") => tutor::run_tutor(),
        _ => {
            println!("rhe v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("usage:");
            println!("  rhe generate  — syllable table from data/");
            println!("  rhe briefs    — show word brief assignments");
            println!("  rhe listen    — show raw key events + chords");
            println!("  rhe run       — menu bar app + full engine");
            println!("  rhe tutor     — interactive typing tutor");
        }
    }
}

fn show_map() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict").unwrap();
    let freq = std::fs::read_to_string("data/en_freq.txt").unwrap();
    let syllable_table = table_gen::generate(&cmudict, &freq);

    let finger = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 { return "-".to_string(); }
        (0..4).filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger[i]).collect::<Vec<_>>().join("+")
    };

    // Collect consonant map (onset side — right hand combos)
    println!("=== CONSONANT MAP (same both hands) ===\n");
    println!("{:<6} {:<10} {:<8}", "Combo", "Fingers", "Sound");
    println!("{}", "-".repeat(30));

    // Get the onset consonant for each combo by looking at Mode 1 no-ctrl
    // syllables with no coda (L:0000)
    for bits in 1..16u8 {
        let key = bits as u16; // R:bits L:0 M:0 C:0
        if let Some(ipa) = syllable_table.get(&key) {
            // The IPA is the full syllable — onset consonant + vowel.
            // The onset is everything before the vowel.
            // For single-combo onset, it's the first char(s).
            println!("{:04b}   {:<10} {}", bits, combo_label(bits), ipa);
        }
    }

    // Vowel map for both-hands chords
    println!("\n=== VOWEL MAP (both-hands chords) ===\n");
    println!("{:<8} {:<20} {:<8}", "Name", "Order", "Vowel");
    println!("{}", "-".repeat(45));

    let mode_orders = [
        (0, false, "zil",   "R↓ L↓ R↑ L↑"),
        (1, false, "ter",   "R↓ L↓ L↑ R↑"),
        (2, false, "stel",  "L↓ R↓ R↑ L↑"),
        (3, false, "lun",   "L↓ R↓ L↑ R↑"),
        (0, true,  "zila",  "R↓ L↓ R↑ L↑ +cmd"),
        (1, true,  "tera",  "R↓ L↓ L↑ R↑ +cmd"),
        (2, true,  "stela", "L↓ R↓ R↑ L↑ +cmd"),
        (3, true,  "luna",  "L↓ R↓ L↑ R↑ +cmd"),
    ];

    // Use a known both-hands pair to read vowels (onset=T right=1, coda=T left=?)
    // Find coda=T combo
    let t_coda = syllable_table.iter()
        .find(|(k, v)| {
            let r = (**k & 0xF) as u8;
            let l = ((**k >> 4) & 0xF) as u8;
            r == 1 && l != 0 && ((**k >> 8) & 0x3) == 0 && ((**k >> 10) & 1) == 0
        })
        .map(|(k, _)| ((*k >> 4) & 0xF) as u8)
        .unwrap_or(1);

    for &(mode, ctrl, name, order) in &mode_orders {
        let key = 1u16 | ((t_coda as u16) << 4) | ((mode as u16) << 8) | if ctrl { 1u16 << 10 } else { 0 };
        let vowel = syllable_table.get(&key).map(|s| s.as_str()).unwrap_or("(none)");
        println!("{:<8} {:<20} {}", name, order, vowel);
    }

    // Top syllables by frequency
    println!("\n=== TOP 50 SYLLABLES (by frequency in table) ===\n");
    println!("{:<6} {:<8} {:<8} {:<6} {:<5} {}", "Key", "Right", "Left", "Mode", "⌘", "IPA");
    println!("{}", "-".repeat(50));

    // We need frequency data to sort. Use the order they appear in generate
    // (which is already frequency-driven for onset/coda assignment).
    // For now just show first 50 both-hands entries.
    let mut both: Vec<(u16, &str)> = syllable_table.iter()
        .filter(|(k, _)| {
            let r = (**k & 0xF) as u8;
            let l = ((**k >> 4) & 0xF) as u8;
            r != 0 && l != 0
        })
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    both.sort_by_key(|(k, _)| *k);

    for (key, ipa) in both.iter().take(50) {
        let right = (key & 0xF) as u8;
        let left = ((key >> 4) & 0xF) as u8;
        let mode = ((key >> 8) & 0x3) + 1;
        let ctrl = if (key >> 10) & 1 == 1 { " ⌘" } else { "" };
        println!("{:<6} {:<8} {:<8} {:<6} {:<5} {}",
            key, combo_label(right), combo_label(left), mode, ctrl, ipa);
    }

    // Single-hand syllables
    println!("\n=== SINGLE-HAND SYLLABLES ===\n");
    println!("Right-only (onset + vowel, no coda):");
    let mut right_only: Vec<(u16, &str)> = syllable_table.iter()
        .filter(|(k, _)| {
            let r = (**k & 0xF) as u8;
            let l = ((**k >> 4) & 0xF) as u8;
            r != 0 && l == 0
        })
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    right_only.sort_by_key(|(k, _)| *k);
    for (key, ipa) in &right_only {
        let right = (key & 0xF) as u8;
        let ctrl = if (key >> 10) & 1 == 1 { " ⌘" } else { "" };
        println!("  {:<8} {:<5} {}", combo_label(right), ctrl, ipa);
    }

    println!("\nLeft-only (vowel + coda, no onset):");
    let mut left_only: Vec<(u16, &str)> = syllable_table.iter()
        .filter(|(k, _)| {
            let r = (**k & 0xF) as u8;
            let l = ((**k >> 4) & 0xF) as u8;
            r == 0 && l != 0
        })
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    left_only.sort_by_key(|(k, _)| *k);
    for (key, ipa) in &left_only {
        let left = ((key >> 4) & 0xF) as u8;
        let ctrl = if (key >> 10) & 1 == 1 { " ⌘" } else { "" };
        println!("  {:<8} {:<5} {}", combo_label(left), ctrl, ipa);
    }

    println!("\nTotal: {} syllables ({} both-hands, {} right-only, {} left-only)",
        syllable_table.len(), both.len(), right_only.len(), left_only.len());
}

fn show_briefs() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict").unwrap();
    let freq = std::fs::read_to_string("data/en_freq.txt").unwrap();
    let syllable_table = table_gen::generate(&cmudict, &freq);
    let brief_map = briefs::generate_briefs(&cmudict, &freq, &syllable_table);

    let finger_names = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 { return "-".to_string(); }
        (0..4).filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger_names[i])
            .collect::<Vec<_>>().join("+")
    };

    let mut entries: Vec<(u16, &str)> = brief_map.iter().map(|(k, v)| (*k, v.as_str())).collect();
    entries.sort_by_key(|(k, _)| *k);

    println!("=== SINGLE-HAND BRIEFS (top 60 words) ===\n");
    let mode_names = ["zil", "ter", "stel", "lun"];
    let mode_names_cmd = ["zila", "tera", "stela", "luna"];

    println!("{:<8} {:<8} {:<8} {}", "Right", "Left", "Voice", "Word");
    println!("{}", "-".repeat(40));

    for (key, word) in &entries {
        let right = (key & 0xF) as u8;
        let left = ((key >> 4) & 0xF) as u8;
        let mode = ((key >> 8) & 0x3) as usize;
        let ctrl = (key >> 10) & 1 == 1;

        let hand = if left == 0 { "R" } else if right == 0 { "L" } else { "B" };

        if hand != "B" {
            let voice = if left == 0 {
                if ctrl { "⌘right" } else { "right" }
            } else {
                if ctrl { "⌘left" } else { "left" }
            };
            println!("{:<8} {:<8} {:<8} {}",
                combo_label(right), combo_label(left), voice, word.trim());
        }
    }

    println!("\n=== BOTH-HANDS BRIEFS (phonetic, first 50) ===\n");
    let mut both_count = 0;
    for (key, word) in &entries {
        let right = (key & 0xF) as u8;
        let left = ((key >> 4) & 0xF) as u8;
        if right != 0 && left != 0 {
            let mode = ((key >> 8) & 0x3) as usize;
            let ctrl = (key >> 10) & 1 == 1;
            let voice = if ctrl { mode_names_cmd[mode] } else { mode_names[mode] };
            let ipa = syllable_table.get(key).map(|s| s.as_str()).unwrap_or("");
            println!("{:<8} {:<8} {:<8} {:<12} {}",
                combo_label(right), combo_label(left), voice, word.trim(), ipa);
            both_count += 1;
            if both_count >= 50 { break; }
        }
    }
}

fn generate_table() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict")
        .expect("data/cmudict.dict not found — run from project root");
    let freq = std::fs::read_to_string("data/en_freq.txt")
        .expect("data/en_freq.txt not found — run from project root");

    let table = table_gen::generate(&cmudict, &freq);

    let mut entries: Vec<(u16, &str)> = table.iter().map(|(k, v)| (*k, v.as_str())).collect();
    entries.sort_by_key(|(k, _)| *k);

    println!("Generated {} syllable mappings out of {} slots", entries.len(), chord_map::ChordKey::MAX);
    println!();

    let limit = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(50);
    for (key, syllable) in entries.iter().take(limit) {
        let right = key & 0xF;
        let left = (key >> 4) & 0xF;
        let mode = (key >> 8) & 0x3;
        let ctrl = (key >> 10) & 1;
        println!(
            "  {:04} R:{:04b} L:{:04b} M:{} C:{} → {}",
            key, right, left, mode, ctrl, syllable
        );
    }

    if entries.len() > 50 {
        println!("  ... and {} more", entries.len() - 50);
    }
}

fn listen() {
    println!("rhe listen — press home row keys, ctrl-C to quit");
    println!("watching: a o e u (left) | h t n s (right) | ctrl | space");
    println!();

    let mut input = input::rdev_backend::RdevInput::start_listen()
        .expect("failed to start key capture");
    let mut sm = state_machine::StateMachine::new();

    loop {
        let Some(event) = input.next_event() else { break };
        println!("  key: {:?}", event);

        for sm_event in sm.feed(event) {
            match &sm_event {
                state_machine::Event::Chord(chord) => {
                    let key = chord_map::ChordKey::from_chord(chord);
                    println!("  >>> CHORD key={} R:{:04b} L:{:04b} mode={:?} ctrl={}",
                        key.0,
                        chord.right.0,
                        chord.left.0,
                        chord.mode,
                        chord.thumbs.has_ctrl(),
                    );
                }
                state_machine::Event::SpaceUp => println!("  >>> SPACE UP"),
            }
        }
    }
}

/// Full engine with menu bar app.
/// Tray runs on main thread (required by macOS).
/// Engine runs on a background thread.
fn run() {
    eprintln!("rhe — loading...");

    let enabled = Arc::new(AtomicBool::new(false)); // start OFF
    let enabled_engine = enabled.clone();

    // Engine thread
    std::thread::spawn(move || {
        let cmudict = std::fs::read_to_string("data/cmudict.dict")
            .expect("data/cmudict.dict not found — run from project root");
        let freq = std::fs::read_to_string("data/en_freq.txt")
            .expect("data/en_freq.txt not found — run from project root");

        let raw_table = table_gen::generate(&cmudict, &freq);

        let mut syllables = chord_map::SyllableTable::new();
        for (key_bits, label) in &raw_table {
            syllables.insert(chord_map::ChordKey(*key_bits), label.clone());
        }

        // Generate word briefs from frequency data
        let brief_map = briefs::generate_briefs(&cmudict, &freq, &raw_table);
        briefs::print_coverage(&brief_map, &freq);

        let mut brief_table = chord_map::SyllableTable::new();
        for (key_bits, word) in &brief_map {
            brief_table.insert(chord_map::ChordKey(*key_bits), word.clone());
        }

        let mut interp = interpreter::Interpreter::new(syllables, brief_table);

        eprintln!("rhe: loaded {} syllables. click menu bar icon to enable.", raw_table.len());

        #[cfg(target_os = "macos")]
        let out = output::macos::MacOSOutput::new();

        let mut input = input::rdev_backend::RdevInput::start_grab(enabled_engine)
            .expect("failed to start key capture");
        let mut sm = state_machine::StateMachine::new();

        loop {
            let Some(event) = input.next_event() else { break };

            for sm_event in sm.feed(event) {
                match &sm_event {
                    state_machine::Event::Chord(chord) => {
                        let key = chord_map::ChordKey::from_chord(chord);
                        eprintln!("  chord: key={} R:{:04b} L:{:04b} {:?} ctrl={}",
                            key.0, chord.right.0, chord.left.0,
                            chord.mode, chord.thumbs.has_ctrl());
                    }
                    state_machine::Event::SpaceUp => eprintln!("  space-up"),
                }

                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            eprintln!("  emit: {}", text);
                            out.emit(text);
                        }
                    }
                }
            }
        }
    });

    // Tray on main thread (macOS requires UI on main thread)
    tray::run_tray(enabled);
}
