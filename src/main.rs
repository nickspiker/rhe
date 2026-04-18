mod chord_map;
mod chord_state;
mod hand;
mod input;
mod interpreter;
mod output;
mod state_machine;
mod table_gen;
mod tray;

use input::KeyInput;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("generate") => generate_table(),
        Some("listen") => listen(),
        Some("run") => run(),
        _ => {
            println!("rhe v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("usage:");
            println!("  rhe generate  — generate syllable table from data/");
            println!("  rhe listen    — show raw key events + chords (debug)");
            println!("  rhe run       — menu bar app + full engine");
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
                state_machine::Event::WordStart => println!("  >>> WORD START"),
                state_machine::Event::WordEnd => println!("  >>> WORD END"),
                state_machine::Event::Undo => println!("  >>> UNDO"),
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
        let mut briefs = chord_map::SyllableTable::new();
        for (key_bits, label) in &raw_table {
            syllables.insert(chord_map::ChordKey(*key_bits), label.clone());
            // Use same table for briefs until we build proper word briefs
            briefs.insert(chord_map::ChordKey(*key_bits), label.clone());
        }

        let mut interp = interpreter::Interpreter::new(syllables, briefs);

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
                    state_machine::Event::WordStart => eprintln!("  word-start"),
                    state_machine::Event::WordEnd => eprintln!("  word-end"),
                    state_machine::Event::Undo => eprintln!("  undo"),
                }

                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            eprintln!("  emit: {}", text);
                            out.emit(text);
                        }
                        interpreter::Action::Undo => {
                            eprintln!("  emit: [UNDO]");
                            out.emit("[UNDO]");
                        }
                    }
                }
            }
        }
    });

    // Tray on main thread (macOS requires UI on main thread)
    tray::run_tray(enabled);
}
