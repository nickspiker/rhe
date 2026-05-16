//! Tray-app entry point.

mod briefs;
mod crypto;
mod data;
mod hand;
mod input;
mod interpreter;
mod key_mask;
mod log;
mod output;
mod phoneme_dict;
mod layout;
mod scan;
mod state_machine;
mod tray;
mod tutor;
mod word_lookup;

#[cfg(target_os = "macos")]
use input::KeyInput;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn main() {
    // Self-verification is informational, not gating: the binary always runs.
    // Tell the user which state they're in so they can act on it if they care.
    match crypto::self_verify::verify_binary_hash() {
        Ok(Some(_)) => {}
        Ok(None) => {
            info!(
                "unsigned build (cargo install or local) — integrity not cryptographically verified."
            );
        }
        Err(e) => {
            rerror!(
                "signature check failed — {} (binary appears modified or corrupted; continuing anyway).",
                e
            );
        }
    }

    run();
}

/// Full engine with menu bar app.
#[cfg(target_os = "macos")]
fn run() {
    crate::log::init();
    info!("loading...");

    let enabled = Arc::new(AtomicBool::new(true)); // start in rhe mode
    let quit = Arc::new(AtomicBool::new(false));
    let fallback = interpreter::FallbackMode::new_shared_from_env();
    let mode_flags = interpreter::new_shared_mode_flags();
    let enabled_engine = enabled.clone();
    let fallback_engine = fallback.clone();
    let mode_flags_engine = mode_flags.clone();

    // Build the tray's event loop on the main thread so its proxy can be
    // handed to the engine thread before it spawns.
    let (event_loop, proxy) = tray::build();
    let drill_proxy = proxy.clone();
    let caps_proxy = proxy.clone();

    std::thread::spawn(move || {
        let phoneme_table = crate::layout::chords::PhonemeTable::new();
        // English: PhonemeDictionary built from CMU + frequency data so the
        // interpreter can resolve homographs (`to/two`) by phoneme path.
        // Māori: 1:1 grapheme→phoneme, no equivalent dictionary needed —
        // engine emits via the autospell (phoneme→grapheme) fallback.
        #[cfg(feature = "lang-en")]
        let dictionary = {
            let cmudict = data::load_cmudict();
            let freq = data::load_word_freq();
            phoneme_dict::PhonemeDictionary::build(&cmudict, &freq)
        };
        #[cfg(feature = "lang-mri")]
        let dictionary = phoneme_dict::PhonemeDictionary::empty();
        let brief_table = crate::briefs::load_briefs();

        let mut interp = interpreter::Interpreter::with_fallback_and_modes(
            phoneme_table,
            brief_table,
            dictionary,
            fallback_engine,
            mode_flags_engine,
        );

        info!("ready. click menu bar icon to enable.");

        let out = output::macos::MacOSOutput::new();

        let input = input::cgevent_backend::CgEventInput::start_grab(
            enabled_engine,
            false,
            Some(caps_proxy),
        )
        .expect("failed to start key capture");
        let mut sm = state_machine::StateMachine::new();

        loop {
            let event = match input.rx.recv() {
                Ok(input::HidEvent::Key(ev)) => ev,
                Ok(input::HidEvent::Quit) => break,
                Err(_) => break,
            };

            // Tee to the tutor window so the drill state machine sees
            // the same events the interpreter does. Tray drops the
            // event on the floor when no tutor window is open.
            let _ = drill_proxy.send_event(tray::TrayEvent::DrillKey(event));

            for sm_event in sm.feed(event) {
                match &sm_event {
                    state_machine::Event::Chord {
                        key, first_down, ..
                    } => tlog!(
                        "engine chord: R:{:04b} L:{:04b} mod={} first_down={:?}",
                        key.right_bits(),
                        key.left_bits(),
                        key.has_mod(),
                        first_down
                    ),
                    state_machine::Event::SpaceUp => tlog!("engine: SpaceUp"),
                    state_machine::Event::Backspace => tlog!("engine: Backspace"),
                    state_machine::Event::Mod { activity_in_session } => {
                        tlog!("engine: Mod (activity_in_session={})", activity_in_session)
                    }
                    state_machine::Event::UndoPhoneme => tlog!("engine: UndoPhoneme"),
                    state_machine::Event::SymbolMode => tlog!("engine: SymbolMode"),
                }

                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            tlog!("engine emit: {:?}", text);
                            out.emit(text);
                        }
                        interpreter::Action::Backspace(n) => {
                            tlog!("engine emit: backspace x{}", n);
                            out.backspace(n);
                        }
                        interpreter::Action::Replace {
                            ref before,
                            ref after,
                        } => {
                            tlog!("engine emit: replace(-{:?}) {:?}", before, after);
                            out.backspace(before.chars().count());
                            out.emit(after);
                        }
                    }
                }
            }
        }
    });

    tray::run_tray(event_loop, enabled, quit, fallback, mode_flags);
}

/// Full engine on Linux — evdev grab + uinput output + tray menu. Engine runs in a background thread; the tray event loop owns the main thread (tray-icon's DBus/StatusNotifierItem machinery requires that).
#[cfg(target_os = "linux")]
fn run() {
    crate::log::init();
    info!("loading...");

    let enabled = Arc::new(AtomicBool::new(true));
    let quit = Arc::new(AtomicBool::new(false));
    let fallback = interpreter::FallbackMode::new_shared_from_env();
    let mode_flags = interpreter::new_shared_mode_flags();
    let enabled_engine = enabled.clone();
    let fallback_engine = fallback.clone();
    let mode_flags_engine = mode_flags.clone();
    let quit_engine = quit.clone();

    // Build the tray's event loop on the main thread so its proxy can be
    // handed to the engine thread before it spawns. The evdev reader wakes
    // the tray via this proxy whenever a solo caps press toggles enabled,
    // so the tray icon/check item refresh without polling.
    let (event_loop, proxy) = tray::build();
    let toggle_proxy = proxy.clone();
    let drill_proxy = proxy.clone();
    let on_toggle: input::evdev_backend::ToggleHook = Arc::new(move || {
        let _ = toggle_proxy.send_event(tray::TrayEvent::StateChanged);
    });

    std::thread::spawn(move || {
        let phoneme_table = crate::layout::chords::PhonemeTable::new();
        // English: PhonemeDictionary built from CMU + frequency data so the
        // interpreter can resolve homographs (`to/two`) by phoneme path.
        // Māori: 1:1 grapheme→phoneme, no equivalent dictionary needed —
        // engine emits via the autospell (phoneme→grapheme) fallback.
        #[cfg(feature = "lang-en")]
        let dictionary = {
            let cmudict = data::load_cmudict();
            let freq = data::load_word_freq();
            phoneme_dict::PhonemeDictionary::build(&cmudict, &freq)
        };
        #[cfg(feature = "lang-mri")]
        let dictionary = phoneme_dict::PhonemeDictionary::empty();
        let brief_table = crate::briefs::load_briefs();

        let mut interp = interpreter::Interpreter::with_fallback_and_modes(
            phoneme_table,
            brief_table,
            dictionary,
            fallback_engine,
            mode_flags_engine,
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

        info!(
            "ready. Tray icon in system panel. Solo Caps press to toggle, CapsLock+Esc to quit."
        );

        loop {
            if quit_engine.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            // Timeout keeps us responsive to tray-initiated quit even when
            // no key events are arriving.
            let event = match input.rx.recv_timeout(std::time::Duration::from_millis(250)) {
                Ok(input::HidEvent::Key(ev)) => ev,
                Ok(input::HidEvent::Quit) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };

            // Tee to the tutor window — tray ignores when closed.
            let _ = drill_proxy.send_event(tray::TrayEvent::DrillKey(event));

            for sm_event in sm.feed(event) {
                match &sm_event {
                    state_machine::Event::Chord {
                        key, first_down, ..
                    } => tlog!(
                        "engine chord: R:{:04b} L:{:04b} mod={} first_down={:?}",
                        key.right_bits(),
                        key.left_bits(),
                        key.has_mod(),
                        first_down
                    ),
                    state_machine::Event::SpaceUp => tlog!("engine: SpaceUp"),
                    state_machine::Event::Backspace => tlog!("engine: Backspace"),
                    state_machine::Event::Mod { activity_in_session } => {
                        tlog!("engine: Mod (activity_in_session={})", activity_in_session)
                    }
                    state_machine::Event::UndoPhoneme => tlog!("engine: UndoPhoneme"),
                    state_machine::Event::SymbolMode => tlog!("engine: SymbolMode"),
                }
                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            tlog!("engine emit: {:?}", text);
                            out.emit(text);
                        }
                        interpreter::Action::Backspace(n) => {
                            tlog!("engine emit: backspace x{}", n);
                            out.backspace(n);
                        }
                        interpreter::Action::Replace {
                            ref before,
                            ref after,
                        } => {
                            tlog!("engine emit: replace(-{:?}) {:?}", before, after);
                            out.backspace(before.chars().count());
                            out.emit(after);
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

    tray::run_tray(event_loop, enabled, quit, fallback, mode_flags);
}

/// Full engine on Windows — rdev::grab + SendInput output + tray menu.
#[cfg(target_os = "windows")]
fn run() {
    crate::log::init();
    info!("loading...");

    let enabled = Arc::new(AtomicBool::new(true));
    let quit = Arc::new(AtomicBool::new(false));
    let fallback = interpreter::FallbackMode::new_shared_from_env();
    let mode_flags = interpreter::new_shared_mode_flags();
    let enabled_engine = enabled.clone();
    let fallback_engine = fallback.clone();
    let mode_flags_engine = mode_flags.clone();
    let quit_engine = quit.clone();

    let (event_loop, proxy) = tray::build();
    let toggle_proxy = proxy.clone();
    let drill_proxy = proxy.clone();
    let on_toggle: input::windows_backend::ToggleHook = Arc::new(move || {
        let _ = toggle_proxy.send_event(tray::TrayEvent::StateChanged);
    });

    std::thread::spawn(move || {
        let phoneme_table = crate::layout::chords::PhonemeTable::new();
        // English: PhonemeDictionary built from CMU + frequency data so the
        // interpreter can resolve homographs (`to/two`) by phoneme path.
        // Māori: 1:1 grapheme→phoneme, no equivalent dictionary needed —
        // engine emits via the autospell (phoneme→grapheme) fallback.
        #[cfg(feature = "lang-en")]
        let dictionary = {
            let cmudict = data::load_cmudict();
            let freq = data::load_word_freq();
            phoneme_dict::PhonemeDictionary::build(&cmudict, &freq)
        };
        #[cfg(feature = "lang-mri")]
        let dictionary = phoneme_dict::PhonemeDictionary::empty();
        let brief_table = crate::briefs::load_briefs();

        let mut interp = interpreter::Interpreter::with_fallback_and_modes(
            phoneme_table,
            brief_table,
            dictionary,
            fallback_engine,
            mode_flags_engine,
        );

        let input = input::windows_backend::WindowsInput::start_grab(
            enabled_engine,
            input::windows_backend::QuitTrigger::EscOrCapsPlusEsc,
            Some(on_toggle),
        )
        .expect("failed to start key capture");
        let out = output::windows::WindowsOutput::new();
        let mut sm = state_machine::StateMachine::new();

        info!("ready. Tray icon in system panel. Esc to quit.");

        loop {
            if quit_engine.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let event = match input.rx.recv_timeout(std::time::Duration::from_millis(250)) {
                Ok(input::HidEvent::Key(ev)) => ev,
                Ok(input::HidEvent::Quit) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };

            let _ = drill_proxy.send_event(tray::TrayEvent::DrillKey(event));

            for sm_event in sm.feed(event) {
                match &sm_event {
                    state_machine::Event::Chord {
                        key, first_down, ..
                    } => tlog!(
                        "engine chord: R:{:04b} L:{:04b} mod={} first_down={:?}",
                        key.right_bits(),
                        key.left_bits(),
                        key.has_mod(),
                        first_down
                    ),
                    state_machine::Event::SpaceUp => tlog!("engine: SpaceUp"),
                    state_machine::Event::Backspace => tlog!("engine: Backspace"),
                    state_machine::Event::Mod { activity_in_session } => {
                        tlog!("engine: Mod (activity_in_session={})", activity_in_session)
                    }
                    state_machine::Event::UndoPhoneme => tlog!("engine: UndoPhoneme"),
                    state_machine::Event::SymbolMode => tlog!("engine: SymbolMode"),
                }
                if let Some(action) = interp.process(&sm_event) {
                    use output::TextOutput;
                    match action {
                        interpreter::Action::Emit(ref text) => {
                            tlog!("engine emit: {:?}", text);
                            out.emit(text);
                        }
                        interpreter::Action::Backspace(n) => {
                            tlog!("engine emit: backspace x{}", n);
                            out.backspace(n);
                        }
                        interpreter::Action::Replace {
                            ref before,
                            ref after,
                        } => {
                            tlog!("engine emit: replace(-{:?}) {:?}", before, after);
                            out.backspace(before.chars().count());
                            out.emit(after);
                        }
                    }
                }
            }
        }
        quit_engine.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = proxy.send_event(tray::TrayEvent::StateChanged);
    });

    tray::run_tray(event_loop, enabled, quit, fallback, mode_flags);
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn run() {
    rerror!("run: not yet supported on this platform.");
    info!("use `rhe tutor` to practice chords.");
}

