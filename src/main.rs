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
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("map") => show_map(),
        Some("briefs") => show_briefs(),
        Some("listen") => listen(),
        Some("run") => run(),
        Some("tutor") => tutor::run_tutor(),
        _ => {
            println!("rhe v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("usage:");
            println!("  rhe map       — show phoneme-to-chord mapping");
            println!("  rhe briefs    — show word brief assignments");
            println!("  rhe listen    — show raw key events + chords");
            println!("  rhe run       — menu bar app + full engine");
            println!("  rhe tutor     — interactive typing tutor");
        }
    }
}

fn show_map() {
    let finger = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 { return "-".to_string(); }
        (0..4).filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger[i])
            .collect::<Vec<_>>()
            .join("+")
    };

    println!("=== CONSONANTS (right hand) ===\n");
    println!("{:<10} {:<5} {:<6} {}", "Fingers", "⌘", "IPA", "Phoneme");
    println!("{}", "-".repeat(35));

    use chord_map::Phoneme;
    let consonants = [
        Phoneme::T, Phoneme::S, Phoneme::K, Phoneme::P,
        Phoneme::N, Phoneme::R, Phoneme::L, Phoneme::H,
        Phoneme::F, Phoneme::W, Phoneme::Th, Phoneme::Sh,
        Phoneme::Ch, Phoneme::Ng, Phoneme::Y,
        Phoneme::D, Phoneme::Z, Phoneme::G, Phoneme::B,
        Phoneme::M, Phoneme::Dh, Phoneme::V, Phoneme::Zh,
        Phoneme::Jh,
    ];

    for p in consonants {
        let key = p.chord_key();
        let mod_str = if key.has_mod() { "⌘" } else { "" };
        println!("{:<10} {:<5} {:<6} {:?}",
            combo_label(key.right_bits()), mod_str, p.to_ipa(), p);
    }

    println!("\n=== VOWELS (left hand) ===\n");
    println!("{:<10} {:<6} {}", "Fingers", "IPA", "Example");
    println!("{}", "-".repeat(35));

    let vowels = [
        (Phoneme::Ah, "but/about"), (Phoneme::Ih, "sit"), (Phoneme::Eh, "bed"),
        (Phoneme::Ae, "cat"), (Phoneme::Iy, "see"), (Phoneme::Aa, "father"),
        (Phoneme::Ey, "say"), (Phoneme::Er, "bird"), (Phoneme::Ay, "my"),
        (Phoneme::Ow, "go"), (Phoneme::Ao, "thought"), (Phoneme::Uw, "blue"),
        (Phoneme::Aw, "cow"), (Phoneme::Uh, "book"), (Phoneme::Oy, "boy"),
    ];

    for (p, example) in vowels {
        let key = p.chord_key();
        println!("{:<10} {:<6} {}",
            combo_label(key.left_bits()), p.to_ipa(), example);
    }

    println!("\n39 phonemes mapped. {} slots free for briefs.",
        chord_map::ChordKey::MAX as usize - 39);
}

fn show_briefs() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict").unwrap();
    let freq = std::fs::read_to_string("data/en_freq.txt").unwrap();
    let brief_table = briefs::generate_briefs(&cmudict, &freq);
    briefs::print_coverage(&brief_table, &freq);

    let finger = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 { return "-".to_string(); }
        (0..4).filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger[i])
            .collect::<Vec<_>>()
            .join("+")
    };

    println!("\n=== BRIEFS (both-hands combos) ===\n");
    println!("{:<10} {:<10} {:<5} {}", "Right", "Left", "⌘", "Word");
    println!("{}", "-".repeat(40));

    let mut count = 0;
    for key_val in 0..chord_map::ChordKey::MAX {
        let key = chord_map::ChordKey(key_val);
        if let Some(word) = brief_table.lookup(key) {
            let right = key.right_bits();
            let left = key.left_bits();
            if right != 0 && left != 0 {
                let mod_str = if key.has_mod() { "⌘" } else { "" };
                println!("{:<10} {:<10} {:<5} {}",
                    combo_label(right), combo_label(left), mod_str, word.trim());
                count += 1;
                if count >= 50 { break; }
            }
        }
    }
}

fn listen() {
    println!("rhe listen — press home row keys, ctrl-C to quit");
    println!("right hand = consonants | left hand = vowels | ⌘ = mod | space = word");
    println!();

    let phonemes = chord_map::PhonemeTable::new();
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
                    let phoneme = phonemes.lookup(key);
                    let label = phoneme.map(|p| p.to_ipa()).unwrap_or("?");
                    println!("  >>> CHORD key={} R:{:04b} L:{:04b} mod={} → {}",
                        key.0, chord.right.0, chord.left.0, chord.modkey, label);
                }
                state_machine::Event::SpaceUp => println!("  >>> SPACE UP"),
                state_machine::Event::Backspace => println!("  >>> BACKSPACE"),
                state_machine::Event::UndoPhoneme => println!("  >>> UNDO PHONEME"),
            }
        }
    }
}

/// Full engine with menu bar app.
fn run() {
    eprintln!("rhe — loading...");

    let enabled = Arc::new(AtomicBool::new(false));
    let enabled_engine = enabled.clone();

    std::thread::spawn(move || {
        let cmudict = std::fs::read_to_string("data/cmudict.dict")
            .expect("data/cmudict.dict not found — run from project root");
        let freq = std::fs::read_to_string("data/en_freq.txt")
            .expect("data/en_freq.txt not found — run from project root");

        let phoneme_table = chord_map::PhonemeTable::new();
        let dictionary = table_gen::PhonemeDictionary::build(&cmudict, &freq);
        let brief_table = briefs::generate_briefs(&cmudict, &freq);
        briefs::print_coverage(&brief_table, &freq);

        let mut interp = interpreter::Interpreter::new(phoneme_table, brief_table, dictionary);

        eprintln!("rhe: ready. click menu bar icon to enable.");

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
                        eprintln!("  chord: key={} R:{:04b} L:{:04b} mod={}",
                            key.0, chord.right.0, chord.left.0, chord.modkey);
                    }
                    state_machine::Event::SpaceUp => eprintln!("  space-up"),
                    state_machine::Event::Backspace => eprintln!("  backspace"),
                    state_machine::Event::UndoPhoneme => eprintln!("  undo-phoneme"),
                }

                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            eprintln!("  emit: {}", text);
                            out.emit(text);
                        }
                        interpreter::Action::Backspace(n) => {
                            eprintln!("  emit: backspace x{}", n);
                            out.backspace(n);
                        }
                    }
                }
            }
        }
    });

    tray::run_tray(enabled);
}
