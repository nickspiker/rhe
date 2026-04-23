//! CLI entry point and subcommand dispatch.

mod briefs;
mod briefs_data;
mod number_data;
mod ordered_briefs_data;
mod suffixes_data;
mod chord_map;
mod data;
mod hand;
mod input;
mod key_mask;
mod scan;
mod interpreter;
mod output;
mod state_machine;
mod table_gen;
mod tray;
mod tutor;
mod wiki;
mod word_lookup;

#[cfg(target_os = "macos")]
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
        Some("tutor") => tutor::run_tutor(false),
        Some("test") => tutor::run_tutor(true),
        Some("rollover") => rollover_test(),
        Some("bench") => tutor::run_bench(),
        _ => {
            println!("rhe v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("usage:");
            println!("  rhe map       — show phoneme-to-chord mapping");
            println!("  rhe briefs    — show word brief assignments");
            println!("  rhe listen    — show raw key events + chords");
            println!("  rhe run       — menu bar app + full engine");
            println!("  rhe tutor     — interactive typing tutor (Wikipedia text)");
            println!("  rhe test      — tutor with curated homophone/ordering drills");
            println!("  rhe bench     — measure chord speed per finger combo");
            println!("  rhe rollover  — test simultaneous key count");
        }
    }
}

fn show_map() {
    let finger = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 {
            return "-".to_string();
        }
        (0..4)
            .filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger[i])
            .collect::<Vec<_>>()
            .join("+")
    };

    println!("=== CONSONANTS (right hand) ===\n");
    println!("{:<10} {:<5} {:<6} {}", "Fingers", "⌘", "IPA", "Phoneme");
    println!("{}", "-".repeat(35));

    use chord_map::Phoneme;
    let consonants = [
        Phoneme::T,
        Phoneme::S,
        Phoneme::K,
        Phoneme::P,
        Phoneme::N,
        Phoneme::R,
        Phoneme::L,
        Phoneme::H,
        Phoneme::F,
        Phoneme::W,
        Phoneme::Th,
        Phoneme::Sh,
        Phoneme::Ch,
        Phoneme::Ng,
        Phoneme::Y,
        Phoneme::D,
        Phoneme::Z,
        Phoneme::G,
        Phoneme::B,
        Phoneme::M,
        Phoneme::Dh,
        Phoneme::V,
        Phoneme::Zh,
        Phoneme::Jh,
    ];

    for p in consonants {
        let key = p.chord_key();
        let mod_str = if key.has_mod() { "⌘" } else { "" };
        println!(
            "{:<10} {:<5} {:<6} {:?}",
            combo_label(key.right_bits()),
            mod_str,
            p.to_ipa(),
            p
        );
    }

    println!("\n=== VOWELS (left hand) ===\n");
    println!("{:<10} {:<6} {}", "Fingers", "IPA", "Example");
    println!("{}", "-".repeat(35));

    let vowels = [
        (Phoneme::Ah, "but/about"),
        (Phoneme::Ih, "sit"),
        (Phoneme::Eh, "bed"),
        (Phoneme::Ae, "cat"),
        (Phoneme::Iy, "see"),
        (Phoneme::Aa, "father"),
        (Phoneme::Ey, "say"),
        (Phoneme::Er, "bird"),
        (Phoneme::Ay, "my"),
        (Phoneme::Ow, "go"),
        (Phoneme::Ao, "thought"),
        (Phoneme::Uw, "blue"),
        (Phoneme::Aw, "cow"),
        (Phoneme::Uh, "book"),
        (Phoneme::Oy, "boy"),
    ];

    for (p, example) in vowels {
        let key = p.chord_key();
        println!(
            "{:<10} {:<6} {}",
            combo_label(key.left_bits()),
            p.to_ipa(),
            example
        );
    }

    // 9-bit chord space (4R + 4L + mod) = 512 slots, 39 phonemes assigned.
    println!("\n39 phonemes mapped. {} slots free for briefs.", 512 - 39);
}

fn show_briefs() {
    let brief_table = briefs::load_briefs();

    let finger = ["I", "M", "R", "P"];
    let combo_label = |bits: u8| -> String {
        if bits == 0 {
            return "-".to_string();
        }
        (0..4)
            .filter(|&i| bits & (1 << i) != 0)
            .map(|i| finger[i])
            .collect::<Vec<_>>()
            .join("+")
    };

    println!("\n=== BRIEFS (both-hands combos) ===\n");
    println!("{:<10} {:<10} {:<5} {}", "Right", "Left", "⌘", "Word");
    println!("{}", "-".repeat(40));

    let mut entries: Vec<_> = brief_table
        .iter()
        .filter(|(key, _, _)| key.right_bits() != 0 && key.left_bits() != 0)
        .collect();
    entries.sort_by(|a, b| a.2.cmp(b.2));
    let mut count = 0;
    for (key, _first_down, word) in entries {
        let right = key.right_bits();
        let left = key.left_bits();
        let mod_str = if key.has_mod() { "⌘" } else { "" };
        println!(
            "{:<10} {:<10} {:<5} {}",
            combo_label(right),
            combo_label(left),
            mod_str,
            word.trim()
        );
        count += 1;
        if count >= 50 {
            break;
        }
    }
}

#[cfg(target_os = "macos")]
fn listen() {
    println!("rhe listen — press home row keys, ctrl-C to quit");
    println!("right hand = consonants | left hand = vowels | ⌘ = mod | space = word");
    println!();

    let phonemes = chord_map::PhonemeTable::new();
    let mut input =
        input::rdev_backend::RdevInput::start_listen().expect("failed to start key capture");
    let mut sm = state_machine::StateMachine::new();

    loop {
        let Some(event) = input.next_event() else {
            break;
        };
        println!("  key: {:?}", event);

        for sm_event in sm.feed(event) {
            match &sm_event {
                state_machine::Event::Chord { key, .. } => {
                    let phoneme = phonemes.lookup(*key);
                    let label = phoneme.map(|p| p.to_ipa()).unwrap_or("?");
                    println!(
                        "  >>> CHORD R:{:04b} L:{:04b} mod={} → {}",
                        key.right_bits(),
                        key.left_bits(),
                        key.has_mod(),
                        label
                    );
                }
                state_machine::Event::SpaceUp => println!("  >>> SPACE UP"),
                state_machine::Event::Backspace => println!("  >>> BACKSPACE"),
                state_machine::Event::ModTap => println!("  >>> MOD TAP"),
                state_machine::Event::UndoPhoneme => println!("  >>> UNDO PHONEME"),
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn listen() {
    eprintln!("rhe listen: not yet supported on this platform.");
    eprintln!("use `rhe tutor` to see chord recognition in action.");
}

/// Full engine with menu bar app.
#[cfg(target_os = "macos")]
fn run() {
    eprintln!("rhe — loading...");

    let enabled = Arc::new(AtomicBool::new(true)); // start in rhe mode
    let quit = Arc::new(AtomicBool::new(false));
    let fallback = interpreter::FallbackMode::new_shared_from_env();
    let enabled_engine = enabled.clone();
    let fallback_engine = fallback.clone();
    let _quit_engine = quit.clone();

    // Build the tray's event loop on the main thread so its proxy can be
    // handed to the engine thread before it spawns.
    let (event_loop, _proxy) = tray::build();

    std::thread::spawn(move || {
        let cmudict = data::load_cmudict();
        let freq = data::load_word_freq();

        let phoneme_table = chord_map::PhonemeTable::new();
        let dictionary = table_gen::PhonemeDictionary::build(&cmudict, &freq);
        let brief_table = briefs::load_briefs();

        let mut interp = interpreter::Interpreter::with_fallback(
            phoneme_table,
            brief_table,
            dictionary,
            fallback_engine,
        );

        eprintln!("rhe: ready. click menu bar icon to enable.");

        let out = output::macos::MacOSOutput::new();

        let input = input::iohid_backend::IoHidInput::start_grab(enabled_engine, false)
            .expect("failed to start key capture");
        let mut sm = state_machine::StateMachine::new();

        loop {
            let event = match input.rx.recv() {
                Ok(input::HidEvent::Key(ev)) => ev,
                Ok(input::HidEvent::Quit) => break,
                Err(_) => break,
            };

            for sm_event in sm.feed(event) {
                match &sm_event {
                    state_machine::Event::Chord { key, .. } => {
                        eprintln!(
                            "  chord: R:{:04b} L:{:04b} mod={}",
                            key.right_bits(),
                            key.left_bits(),
                            key.has_mod()
                        );
                    }
                    state_machine::Event::SpaceUp => eprintln!("  space-up"),
                    state_machine::Event::Backspace => eprintln!("  backspace"),
                    state_machine::Event::ModTap => eprintln!("  mod-tap"),
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
                        interpreter::Action::Suffix(ref text) => {
                            eprintln!("  emit: suffix {}", text);
                            out.backspace(1);
                            out.emit(text);
                        }
                    }
                }
            }
        }
    });

    tray::run_tray(event_loop, enabled, quit, fallback);
}

/// Full engine on Linux — evdev grab + uinput output + tray menu.
/// Engine runs in a background thread; the tray event loop owns the main
/// thread (tray-icon's DBus/StatusNotifierItem machinery requires that).
#[cfg(target_os = "linux")]
fn run() {
    eprintln!("rhe — loading...");

    let enabled = Arc::new(AtomicBool::new(true));
    let quit = Arc::new(AtomicBool::new(false));
    let fallback = interpreter::FallbackMode::new_shared_from_env();
    let enabled_engine = enabled.clone();
    let fallback_engine = fallback.clone();
    let quit_engine = quit.clone();

    // Build the tray's event loop on the main thread so its proxy can be
    // handed to the engine thread before it spawns. The evdev reader wakes
    // the tray via this proxy whenever caps-tap toggles enabled, so the
    // tray icon/check item refresh without polling.
    let (event_loop, proxy) = tray::build();
    let toggle_proxy = proxy.clone();
    let on_toggle: input::evdev_backend::ToggleHook =
        Arc::new(move || {
            let _ = toggle_proxy.send_event(tray::TrayEvent::StateChanged);
        });

    std::thread::spawn(move || {
        let cmudict = data::load_cmudict();
        let freq = data::load_word_freq();

        let phoneme_table = chord_map::PhonemeTable::new();
        let dictionary = table_gen::PhonemeDictionary::build(&cmudict, &freq);
        let brief_table = briefs::load_briefs();

        let mut interp = interpreter::Interpreter::with_fallback(
            phoneme_table,
            brief_table,
            dictionary,
            fallback_engine,
        );

        // Input before output so the keyboard scan completes before any
        // rhe-owned uinput devices show up in /dev/input/event*.
        let input = input::evdev_backend::EvdevInput::start_grab(
            enabled_engine,
            input::evdev_backend::QuitTrigger::CapsLockPlusEsc,
            Some(on_toggle),
        )
        .expect("failed to start key capture");
        let out = output::linux::LinuxOutput::new();
        let mut sm = state_machine::StateMachine::new();

        eprintln!(
            "rhe: ready. Tray icon in system panel. \
             Caps tap to toggle, CapsLock+Esc to quit."
        );

        loop {
            if quit_engine.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            // Timeout keeps us responsive to tray-initiated quit even when
            // no key events are arriving.
            let event =
                match input.rx.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(input::HidEvent::Key(ev)) => ev,
                    Ok(input::HidEvent::Quit) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                };

            for sm_event in sm.feed(event) {
                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => out.emit(text),
                        interpreter::Action::Backspace(n) => out.backspace(n),
                        interpreter::Action::Suffix(ref text) => {
                            out.backspace(1);
                            out.emit(text);
                        }
                    }
                }
            }
        }
        // Signal tray to exit as well.
        quit_engine.store(true, std::sync::atomic::Ordering::Relaxed);
        // Nudge the tray event loop so it notices the quit flag even if it
        // is currently asleep in Wait.
        let _ = proxy.send_event(tray::TrayEvent::StateChanged);
    });

    tray::run_tray(event_loop, enabled, quit, fallback);
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn run() {
    eprintln!("rhe run: not yet supported on this platform.");
    eprintln!("use `rhe tutor` to practice chords.");
}

#[cfg(target_os = "macos")]
fn rollover_test() {
    use hand::KeyDirection;
    use input::HidEvent;
    use input::iohid_backend::IoHidInput;

    println!("rhe rollover test — press as many home row keys as possible");
    println!("shows how many keys register simultaneously. Esc to quit.");
    println!();

    let enabled = Arc::new(AtomicBool::new(true));
    let input = IoHidInput::start_grab(enabled, true).expect("failed to start key capture");

    let mut held: Vec<&str> = Vec::new();
    let mut max_held: usize = 0;

    loop {
        let event = match input.rx.recv() {
            Ok(HidEvent::Quit) => break,
            Ok(HidEvent::Key(ev)) => ev,
            Err(_) => break,
        };

        let name = scan::label(event.scan);
        match event.direction {
            KeyDirection::Down => {
                if !held.contains(&name) {
                    held.push(name);
                }
            }
            KeyDirection::Up => {
                held.retain(|&n| n != name);
            }
        }

        if held.len() > max_held {
            max_held = held.len();
        }

        // Clear line and print current state
        print!(
            "\r\x1b[K  held: {} (max: {})  [{}]",
            held.len(),
            max_held,
            held.join(" + ")
        );
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    println!("\n\nMax simultaneous keys: {}", max_held);
}

#[cfg(not(target_os = "macos"))]
fn rollover_test() {
    eprintln!("rhe rollover: not yet supported on this platform.");
}
