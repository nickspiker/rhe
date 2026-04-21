//! Interactive typing tutor and bench mode.

use std::io;

use crossterm::ExecutableCommand;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::chord_map::{BriefTable, Phoneme, PhonemeTable};
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent as RheKeyEvent, PhysicalKey};
use crate::input::HidEvent;
#[cfg(target_os = "linux")]
use crate::input::evdev_backend::EvdevInput as GrabInput;
#[cfg(target_os = "macos")]
use crate::input::iohid_backend::IoHidInput as GrabInput;
use crate::interpreter::Interpreter;
#[cfg(target_os = "linux")]
use crate::output::linux::LinuxOutput as PlatformOutput;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use crate::output::NullOutput as PlatformOutput;
use crate::output::TextOutput;
#[cfg(target_os = "macos")]
use crate::output::macos::MacOSOutput as PlatformOutput;
use crate::state_machine::StateMachine;
use crate::table_gen::PhonemeDictionary;
use crate::word_lookup::WordLookup;

// ─── Target: what keys should be pressed ───
// right = 5 bits (4 fingers + thumb/spacebar as bit 4)
// left = 4 bits (4 fingers)
// word = left ⌘ held

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Target {
    right: u8, // 5 bits: index=0, middle=1, ring=2, pinky=3, thumb=4
    left: u8,  // 4 bits: pinky=3, ring=2, middle=1, index=0
    word: bool,
}

impl Target {
    /// Does the live key state have any extra key on the TARGET's hand(s)?
    fn has_extra(&self, state: &KeyState) -> bool {
        let extra_word = state.word && !self.word;

        if self.right != 0 && self.left != 0 {
            // Both hands (brief): check both
            let extra_right = state.right_bits() & !self.right;
            let extra_left = state.left_bits() & !self.left;
            extra_right != 0 || extra_left != 0 || extra_word
        } else if self.right != 0 {
            // Right-hand step: check right hand only
            let extra_right = state.right_bits() & !self.right;
            extra_right != 0 || extra_word
        } else if self.left != 0 {
            // Left-hand step: check left hand only
            let extra_left = state.left_bits() & !self.left;
            extra_left != 0 || extra_word
        } else {
            // Commit step: any key down is extra
            state.right_bits() != 0 || state.left_bits() != 0 || extra_word
        }
    }

    /// Does the live key state match this target?
    fn matches(&self, state: &KeyState) -> bool {
        let word_ok = state.word == self.word;

        if self.right != 0 && self.left != 0 {
            // Both hands (brief): check both exactly
            state.right_bits() == self.right && state.left_bits() == self.left && word_ok
        } else if self.right != 0 {
            state.right_bits() == self.right && word_ok
        } else if self.left != 0 {
            state.left_bits() == self.left && word_ok
        } else {
            // Commit: everything zero + word matches
            state.right_bits() == 0 && state.left_bits() == 0 && word_ok
        }
    }
}

// ─── Steps for a word ───

struct Step {
    target: Target,
    phoneme: Option<Phoneme>,
    /// If true, only check space state — ignore fingers/mod (used for "commit" step)
    space_only: bool,
}

struct PracticeWord {
    word: String,
    phoneme_steps: Vec<Step>,       // word held + phoneme sequence + commit
    brief_steps: Option<Vec<Step>>, // single chord without word + all-off (if brief exists)
    suffix_steps: Option<Vec<Step>>, // roll(base) + suffix chord + all-off (if shorter)
    suffix_label: Option<String>,   // e.g. "~ing" for display
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordMode {
    Brief,
    Phoneme,
    Suffix,
}

#[derive(Default, Clone)]
struct KeyState {
    left: [bool; 4],
    right: [bool; 5], // 4 fingers + thumb (spacebar)
    word: bool,       // left ⌘
}

fn finger_bit(finger: Finger) -> u8 {
    match finger {
        Finger::Index => 0,
        Finger::Middle => 1,
        Finger::Ring => 2,
        Finger::Pinky => 3,
        Finger::Thumb => 4,
    }
}

impl KeyState {
    fn left_bits(&self) -> u8 {
        (self.left[0] as u8) << 3
            | (self.left[1] as u8) << 2
            | (self.left[2] as u8) << 1
            | self.left[3] as u8
    }

    fn right_bits(&self) -> u8 {
        self.right[0] as u8
            | (self.right[1] as u8) << 1
            | (self.right[2] as u8) << 2
            | (self.right[3] as u8) << 3
            | (self.right[4] as u8) << 4
    }
}

struct Practice {
    sentences: Vec<Vec<PracticeWord>>,
    sentence_idx: usize,
    word_idx: usize,
    step_idx: usize,
    mode: WordMode,
}

impl Practice {
    fn current_word(&self) -> Option<&PracticeWord> {
        self.sentences.get(self.sentence_idx)?.get(self.word_idx)
    }

    fn current_steps(&self) -> Option<&[Step]> {
        let word = self.current_word()?;
        match self.mode {
            WordMode::Brief => word.brief_steps.as_deref()
                .or(word.suffix_steps.as_deref())
                .or(Some(&word.phoneme_steps)),
            WordMode::Suffix => word.suffix_steps.as_deref().or(Some(&word.phoneme_steps)),
            WordMode::Phoneme => Some(&word.phoneme_steps),
        }
    }

    fn current_step(&self) -> Option<&Step> {
        self.current_steps()?.get(self.step_idx)
    }

    fn current_target(&self) -> Option<&Target> {
        Some(&self.current_step()?.target)
    }

    fn advance_step(&mut self) {
        self.step_idx += 1;
        if let Some(steps) = self.current_steps() {
            if self.step_idx >= steps.len() {
                self.next_word();
            }
        }
    }

    fn next_word(&mut self) {
        self.step_idx = 0;
        if let Some(sentence) = self.sentences.get(self.sentence_idx) {
            self.word_idx += 1;
            if self.word_idx >= sentence.len() {
                self.word_idx = 0;
                self.sentence_idx += 1;
                if self.sentence_idx >= self.sentences.len() {
                    self.sentence_idx = 0;
                }
            }
        }
        self.mode = self.default_mode();
    }

    fn reset_word(&mut self) {
        self.step_idx = 0;
        self.mode = self.default_mode();
    }

    fn prev_word(&mut self) {
        self.step_idx = 0;
        if self.word_idx > 0 {
            self.word_idx -= 1;
        } else if self.sentence_idx > 0 {
            self.sentence_idx -= 1;
            if let Some(sentence) = self.sentences.get(self.sentence_idx) {
                self.word_idx = sentence.len().saturating_sub(1);
            }
        }
        self.mode = self.default_mode();
    }

    fn default_mode(&self) -> WordMode {
        if let Some(w) = self.current_word() {
            if w.brief_steps.is_some() {
                WordMode::Brief
            } else if w.suffix_steps.is_some() {
                WordMode::Suffix
            } else {
                WordMode::Phoneme
            }
        } else {
            WordMode::Phoneme
        }
    }
}

// ─── Main loop ───

pub fn run_tutor() {
    let cmudict = crate::data::load_cmudict();
    let freq_data = crate::data::load_word_freq();
    let lookup = WordLookup::new(&cmudict);
    let brief_table = crate::briefs::load_briefs();
    let practice = build_practice(&lookup, &brief_table);

    terminal::enable_raw_mode().unwrap();
    io::stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

    let grab_enabled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let input = GrabInput::start_grab(grab_enabled).expect("failed to start key capture");

    // Real output pipeline: same events → state machine → interpreter → text injection
    let mut sm = StateMachine::new();
    let dict = PhonemeDictionary::build(&cmudict, &freq_data);
    let interp_briefs = crate::briefs::load_briefs();
    let mut interp = Interpreter::new(PhonemeTable::new(), interp_briefs, dict);
    let output = PlatformOutput::new();

    let mut log: Vec<String> = Vec::new();
    let mut key_state = KeyState::default();
    let mut practice = practice;
    let mut errored = false;
    // Per-hand accumulator (OR of every finger bit pressed during the current
    // chord session). Resets only when that hand goes to zero, so a flicker
    // on a held key is an idempotent no-op and a 6KRO roll can complete even
    // when all keys aren't held simultaneously.
    let mut chord_right_acc: u8 = 0;
    let mut chord_left_acc: u8 = 0;
    // "Touched this step" — bits pressed by *new key-down events during the
    // current step*, ignoring carryover reseeded from the previous step.
    // Drives hand_touched so abandon detection only fires if the user
    // actually attempted the chord (rather than just releasing old fingers).
    let mut touched_right: u8 = 0;
    let mut touched_left: u8 = 0;
    let mut fingers_during_word = false;
    let mut last_was_botch = false; // true if last word output was IPA fallback

    // Initial draw
    terminal
        .draw(|f| draw(f, &practice, false, &key_state))
        .ok();

    loop {
        // Block on next event
        let hid_event = match input.rx.recv() {
            Ok(ev) => ev,
            Err(_) => break,
        };
        let rhe_event = match hid_event {
            HidEvent::Quit => break,
            HidEvent::Key(ev) => ev,
        };

        update_key_state(&mut key_state, &rhe_event);

        // Snapshot step/mode before processing so we can detect a step
        // transition and re-seed the accumulator with target-relevant
        // currently-held bits (handles consecutive same-hand phonemes:
        // carryover bits that aren't in the new target don't poison the
        // next accumulator, while bits that ARE still count as pressed).
        let prev_step_idx = practice.step_idx;
        let prev_mode = practice.mode;

        // OR the pressed bit into the accumulator. Hand-zero reset happens
        // at the END of the loop so the step handler can still see pre-release
        // state when a hand goes to zero (for abandon detection).
        if rhe_event.direction == KeyDirection::Down {
            match rhe_event.key {
                PhysicalKey::Finger(Hand::Right, finger) => {
                    chord_right_acc |= 1u8 << finger_bit(finger);
                }
                PhysicalKey::Finger(Hand::Left, finger) => {
                    chord_left_acc |= 1u8 << finger_bit(finger);
                }
                _ => {}
            }
        }

        // Feed real output pipeline
        for sm_event in sm.feed(rhe_event) {
            if let Some(action) = interp.process(&sm_event) {
                match action {
                    crate::interpreter::Action::Emit(text) => output.emit(&text),
                    crate::interpreter::Action::Backspace(n) => output.backspace(n),
                    crate::interpreter::Action::Suffix(text) => {
                        output.backspace(1); // remove trailing space
                        output.emit(&text);  // emit suffix + space
                    }
                }
            }
        }

        let all_off = key_state.right_bits() == 0 && key_state.left_bits() == 0 && !key_state.word;

        // Log every key change
        let current_word_name = practice.current_word().map(|w| w.word.as_str()).unwrap_or("?");
        let mode_str = match practice.mode { WordMode::Brief => "brief", WordMode::Suffix => "suf", WordMode::Phoneme => "phon" };
        log.push(format!(
            "state L:{:04b} R:{:05b} word={} | \"{}\" [{}] step={} err={}",
            key_state.left_bits(),
            key_state.right_bits(),
            key_state.word,
            current_word_name,
            mode_str,
            practice.step_idx,
            errored
        ));

        if errored {
            if all_off {
                log.push("  → ERROR CLEAR".to_string());
                errored = false;
            }
        } else {
            let is_key_down = rhe_event.direction == KeyDirection::Down;

            // Word key down → switch to phoneme mode
            if rhe_event.key == PhysicalKey::Word && rhe_event.direction == KeyDirection::Down {
                practice.mode = WordMode::Phoneme;
                practice.step_idx = 0;
                fingers_during_word = false;
            }
            // Word key up at step 0 (no progress)
            else if rhe_event.key == PhysicalKey::Word
                && rhe_event.direction == KeyDirection::Up
                && practice.mode == WordMode::Phoneme
                && practice.step_idx == 0
            {
                if !fingers_during_word {
                    // Solo word tap = backspace
                    if last_was_botch {
                        // Last output was IPA garbage — just delete it, stay on current word
                        last_was_botch = false;
                    } else {
                        // Last output was correct — go back one word in tutor
                        practice.prev_word();
                    }
                }
                practice.mode = practice.default_mode();
                practice.step_idx = 0;
            }
            // Track if any finger pressed during word hold
            if key_state.word && is_key_down && rhe_event.key != PhysicalKey::Word {
                fingers_during_word = true;
            }

            if let Some(target) = practice.current_target() {
                let target = *target;
                let step = practice.current_step().unwrap();

                if step.space_only {
                    // Phoneme commit step: word release advances, any finger is error.
                    if is_key_down && rhe_event.key != PhysicalKey::Word {
                        log.push("  → RESET (finger during commit)".to_string());
                        practice.reset_word();
                        errored = true;
                    } else if !key_state.word {
                        log.push("  → MATCH (commit)".to_string());
                        last_was_botch = false;
                        practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && !target.word {
                    // Brief commit step (all-off target): any new key = error, all-off = advance.
                    if is_key_down {
                        log.push("  → RESET (finger during commit)".to_string());
                        practice.reset_word();
                        errored = true;
                    } else if all_off {
                        log.push("  → MATCH (commit)".to_string());
                        last_was_botch = false;
                        practice.advance_step();
                    }
                } else {
                    // Update "touched this step" trackers on each key-down.
                    // These differ from chord_*_acc because acc can be reseeded
                    // from carryover at step-transition time — we only want
                    // hand_touched to reflect *fresh* key-downs in this step,
                    // otherwise abandon fires on a pure release of carryover.
                    if is_key_down {
                        match rhe_event.key {
                            PhysicalKey::Finger(Hand::Right, finger) => {
                                touched_right |= 1u8 << finger_bit(finger);
                            }
                            PhysicalKey::Finger(Hand::Left, finger) => {
                                touched_left |= 1u8 << finger_bit(finger);
                            }
                            _ => {}
                        }
                    }

                    // hand_touched: user pressed a target-hand key in *this* step.
                    let hand_touched = (target.right != 0 && touched_right != 0)
                        || (target.left != 0 && touched_left != 0);

                    // "Target hands empty" — only the hands the target cares about.
                    let target_hands_empty = (target.right == 0
                        || key_state.right_bits() == 0)
                        && (target.left == 0 || key_state.left_bits() == 0);

                    // Match only checks hands named by the target. A carryover
                    // bit on a non-target hand is ignored (the user is between
                    // phonemes, still releasing their previous chord).
                    let acc_matches = (target.right == 0
                        || chord_right_acc == target.right)
                        && (target.left == 0 || chord_left_acc == target.left)
                        && key_state.word == target.word;

                    // Same gating for the extra-key check — a leftover bit on
                    // a non-target hand isn't "extra", it's carryover.
                    let has_extra_acc = (target.right != 0
                        && (chord_right_acc & !target.right) != 0)
                        || (target.left != 0
                            && (chord_left_acc & !target.left) != 0)
                        || (key_state.word && !target.word);

                    let space_dropped = rhe_event.key == PhysicalKey::Word
                        && rhe_event.direction == KeyDirection::Up
                        && target.word
                        && practice.step_idx > 0;

                    if space_dropped {
                        log.push("  → RESET (space released mid-word)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
                        if !all_off {
                            errored = true;
                        }
                    } else if is_key_down && has_extra_acc {
                        log.push("  → RESET (extra key down)".to_string());
                        practice.reset_word();
                        if !all_off {
                            errored = true;
                        }
                    } else if acc_matches && hand_touched {
                        // Chord complete — union of pressed keys equals target.
                        // Advances to the following commit step where the user
                        // must release everything to finalise the word.
                        log.push("  → MATCH".to_string());
                        practice.advance_step();
                    } else if hand_touched && target_hands_empty && !is_key_down {
                        log.push("  → RESET (chord abandoned)".to_string());
                        practice.reset_word();
                        if !all_off {
                            errored = true;
                        }
                    }
                }
            }
        }

        // Step transition reseed: on MATCH/reset/mode-change, rebuild the
        // accumulator from currently held keys intersected with the new
        // target (stale carryover drops; legitimate head-starts stay), and
        // wipe the "touched this step" trackers so abandon detection won't
        // fire on release of carryover alone.
        if practice.step_idx != prev_step_idx || practice.mode != prev_mode {
            if let Some(new_target) = practice.current_target() {
                chord_right_acc = key_state.right_bits() & new_target.right;
                chord_left_acc = key_state.left_bits() & new_target.left;
            } else {
                chord_right_acc = 0;
                chord_left_acc = 0;
            }
            touched_right = 0;
            touched_left = 0;
        }

        // Hand-zero accumulator reset runs AFTER the step handler so abandon
        // detection still sees the partial-attempt bits on the final key-up.
        if key_state.right_bits() == 0 {
            chord_right_acc = 0;
        }
        if key_state.left_bits() == 0 {
            chord_left_acc = 0;
        }

        // Draw after every event
        terminal
            .draw(|f| draw(f, &practice, errored, &key_state))
            .ok();
    }

    terminal::disable_raw_mode().ok();
    io::stdout().execute(LeaveAlternateScreen).ok();

    let log_path = "tutor_debug.log";
    std::fs::write(log_path, log.join("\n")).ok();
    println!("Debug log written to {}", log_path);
}

// ─── Build practice steps ───

fn build_practice(lookup: &WordLookup, brief_table: &BriefTable) -> Practice {
    let common_text = [
        "alice was beginning to get very tired of sitting by her sister on the bank",
        "and of having nothing to do once or twice she had into the book",
        "her sister was reading but it had no pictures or conversations in it",
        "and what is the use of a book thought alice without pictures or conversations",
        "so she was considering in her own mind as well as she could",
        "for the hot day made her feel very and stupid",
        "whether the pleasure of making a daisy chain would be worth the trouble",
        "of getting up and picking the when suddenly a white rabbit",
        "with pink eyes ran close by her there was nothing so very remarkable in that",
        "nor did alice think it so very much out of the way to hear the rabbit say",
        "oh dear oh dear i shall be late when she thought it over afterwards",
        "it occurred to her that she ought to have wondered at this",
        "but at the time it all seemed quite natural",
        "but when the rabbit actually took a watch out of its pocket and looked at it",
        "and then hurried on alice started to her feet",
        "for it flashed across her mind that she had never before seen a rabbit",
        "with either a pocket or a watch to take out of it",
        "and burning with curiosity she ran across the field after it",
        "and fortunately was just in time to see it pop down a large rabbit hole",
        "under the hedge in another moment down went alice after it",
    ];

    let mut sentences: Vec<Vec<PracticeWord>> = Vec::new();

    for line in &common_text {
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut skipped = Vec::new();

        for group in words.chunks(8) {
            let mut sentence: Vec<PracticeWord> = Vec::new();

            for &word_str in group {
                let clean: String = word_str
                    .chars()
                    .filter(|c| c.is_alphabetic() || *c == '\'')
                    .collect();

                let Some(phonemes) = lookup.lookup(&clean) else {
                    skipped.push(clean);
                    continue;
                };

                // Build phoneme steps (word held)
                // Insert release steps when the same hand is reused after a gap.
                let mut phoneme_steps: Vec<Step> = Vec::new();
                let mut last_right_used = false;
                let mut last_left_used = false;

                for &phoneme in phonemes {
                    let key = phoneme.chord_key();
                    let right = key.right_bits() | if key.has_mod() { 1 << 4 } else { 0 };
                    let left = key.left_bits();
                    let is_right = right != 0;
                    let is_left = left != 0;

                    // If this step uses the right hand and the right hand was used
                    // before (with left-hand steps in between), insert a release step
                    if is_right && last_right_used && !last_left_used {
                        // Consecutive right — no release needed (hand_abandoned handles it)
                    } else if is_right && last_right_used {
                        // Right reused after left steps — insert right=0 release
                        phoneme_steps.push(Step {
                            target: Target { right: 0, left: 0, word: true },
                            phoneme: None,
                            space_only: false,
                        });
                    }
                    if is_left && last_left_used && !last_right_used {
                        // Consecutive left — same
                    } else if is_left && last_left_used {
                        // Left reused after right steps — insert left=0 release
                        phoneme_steps.push(Step {
                            target: Target { right: 0, left: 0, word: true },
                            phoneme: None,
                            space_only: false,
                        });
                    }

                    phoneme_steps.push(Step {
                        target: Target { right, left, word: true },
                        phoneme: Some(phoneme),
                        space_only: false,
                    });

                    last_right_used = is_right;
                    last_left_used = is_left;
                }
                phoneme_steps.push(Step {
                    target: Target {
                        word: false,
                        ..Target::default()
                    },
                    phoneme: None,
                    space_only: true,
                });

                // Build brief steps (no word key) if brief exists
                let brief_steps = {
                    use crate::chord_map::ChordKey;
                    let mut found = None;
                    for k in 0..ChordKey::MAX {
                        if let Some(brief_word) = brief_table.lookup(ChordKey(k)) {
                            if brief_word.trim() == clean {
                                // Found brief for this word
                                let right =
                                    (k & 0xF) as u8 | if k & (1 << 8) != 0 { 1u8 << 4 } else { 0 };
                                let left = ((k >> 4) & 0xF) as u8;
                                found = Some(vec![
                                    Step {
                                        target: Target {
                                            right,
                                            left,
                                            word: false,
                                        },
                                        phoneme: None,
                                        space_only: false,
                                    },
                                    Step {
                                        target: Target::default(), // all off
                                        phoneme: None,
                                        space_only: false,
                                    },
                                ]);
                                break;
                            }
                        }
                    }
                    found
                };

                // Build suffix steps: roll(base) + suffix chord + all-off
                let (suffix_steps, suffix_label) = {
                    use crate::suffixes_data::SUFFIXES;
                    use crate::chord_map::ChordKey;
                    let phoneme_count = phoneme_steps.iter().filter(|s| s.phoneme.is_some()).count();
                    let mut found = (None, None);

                    // Only worth it if phoneme path is 3+ movements
                    if phoneme_count > 2 {
                        // Try suffixes longest first
                        let mut sorted_suffixes: Vec<(u8, &str)> = SUFFIXES.to_vec();
                        sorted_suffixes.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                        for &(suffix_bits, suffix_str) in &sorted_suffixes {
                            if !clean.ends_with(suffix_str) { continue; }
                            if clean.len() <= suffix_str.len() { continue; }

                            // Try base and base+"e"
                            let bases = vec![
                                clean[..clean.len() - suffix_str.len()].to_string(),
                                format!("{}e", &clean[..clean.len() - suffix_str.len()]),
                            ];

                            for base in &bases {
                                // Find roll for base
                                for k in 0..ChordKey::MAX {
                                    if let Some(brief_word) = brief_table.lookup(ChordKey(k)) {
                                        if brief_word.trim() == base {
                                            let r = (k & 0xF) as u8
                                                | if k & (1 << 8) != 0 { 1u8 << 4 } else { 0 };
                                            let l = ((k >> 4) & 0xF) as u8;
                                            let steps = vec![
                                                // Roll for base (no word key)
                                                Step {
                                                    target: Target { right: r, left: l, word: false },
                                                    phoneme: None,
                                                    space_only: false,
                                                },
                                                // All off between roll and suffix
                                                Step {
                                                    target: Target::default(),
                                                    phoneme: None,
                                                    space_only: false,
                                                },
                                                // Suffix chord (left hand only, no word key)
                                                Step {
                                                    target: Target { right: 0, left: suffix_bits, word: false },
                                                    phoneme: None,
                                                    space_only: false,
                                                },
                                                // All off
                                                Step {
                                                    target: Target::default(),
                                                    phoneme: None,
                                                    space_only: false,
                                                },
                                            ];
                                            found = (Some(steps), Some(format!("~{}", suffix_str)));
                                            break;
                                        }
                                    }
                                }
                                if found.0.is_some() { break; }
                            }
                            if found.0.is_some() { break; }
                        }
                    }
                    found
                };

                sentence.push(PracticeWord {
                    word: clean,
                    phoneme_steps,
                    brief_steps,
                    suffix_steps,
                    suffix_label,
                });
            }

            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }

        if !skipped.is_empty() {
            eprintln!("  SKIP unknown: {}", skipped.join(", "));
        }
    }

    let first_has_brief = sentences
        .first()
        .and_then(|s| s.first())
        .map_or(false, |w| w.brief_steps.is_some());

    Practice {
        sentences,
        sentence_idx: 0,
        word_idx: 0,
        step_idx: 0,
        mode: if first_has_brief { WordMode::Brief } else { WordMode::Phoneme },
    }
}

// ─── Key state update ───

/// Check if a word can be typed as base_roll + suffix, and if so return the suffix.
/// Only suggests if the suffix path is shorter than the phoneme path.
fn update_key_state(state: &mut KeyState, event: &RheKeyEvent) {
    let pressed = event.direction == KeyDirection::Down;
    match event.key {
        PhysicalKey::Finger(Hand::Left, Finger::Pinky) => state.left[0] = pressed,
        PhysicalKey::Finger(Hand::Left, Finger::Ring) => state.left[1] = pressed,
        PhysicalKey::Finger(Hand::Left, Finger::Middle) => state.left[2] = pressed,
        PhysicalKey::Finger(Hand::Left, Finger::Index) => state.left[3] = pressed,
        PhysicalKey::Finger(Hand::Right, Finger::Index) => state.right[0] = pressed,
        PhysicalKey::Finger(Hand::Right, Finger::Middle) => state.right[1] = pressed,
        PhysicalKey::Finger(Hand::Right, Finger::Ring) => state.right[2] = pressed,
        PhysicalKey::Finger(Hand::Right, Finger::Pinky) => state.right[3] = pressed,
        PhysicalKey::Finger(Hand::Right, Finger::Thumb) => state.right[4] = pressed,
        PhysicalKey::Finger(Hand::Left, Finger::Thumb) => {} // left has no thumb key
        PhysicalKey::Word => state.word = pressed,
    }
}

// ─── Drawing ───

fn draw(frame: &mut Frame, practice: &Practice, errored: bool, key_state: &KeyState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // title
            Constraint::Length(3),  // sentence
            Constraint::Length(4),  // word detail
            Constraint::Length(11), // keyboard (target + held)
            Constraint::Min(0),     // padding
        ])
        .split(area);

    // Title
    frame.render_widget(
        Paragraph::new(" rhe tutor  [Esc to quit]").style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );

    // Sentence
    if let Some(sentence) = practice.sentences.get(practice.sentence_idx) {
        let mut spans = vec![Span::raw(" ")];
        for (i, w) in sentence.iter().enumerate() {
            let style = if i == practice.word_idx {
                Style::default().fg(Color::White).bold().underlined()
            } else if i < practice.word_idx {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Gray)
            };
            spans.push(Span::styled(&w.word, style));
            spans.push(Span::raw(" "));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::BOTTOM)),
            chunks[1],
        );
    }

    // Word detail + phoneme hint + suffix suggestion
    if let Some(pw) = practice.current_word() {
        let phoneme_label = practice
            .current_step()
            .and_then(|s| s.phoneme)
            .map(|p| format!(" {}", p.to_ipa()))
            .unwrap_or_default();

        let mut word_spans = vec![
            Span::styled(" ", Style::default()),
            Span::styled(&pw.word, Style::default().fg(Color::White).bold()),
        ];
        if let Some(label) = &pw.suffix_label {
            word_spans.push(Span::styled(
                format!("  {}", label),
                Style::default().fg(Color::Rgb(100, 100, 100)),
            ));
        }

        let lines = vec![
            Line::from(word_spans),
            Line::from(vec![Span::styled(
                phoneme_label,
                Style::default().fg(Color::Gray),
            )]),
        ];

        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );

        // Keyboard: all dark when errored, otherwise show target
        let (tr, tl, tw) = if errored {
            (0u8, 0u8, false)
        } else if let Some(step) = practice.current_step() {
            (step.target.right, step.target.left, step.target.word)
        } else {
            (0, 0, false)
        };
        draw_keyboard(
            frame,
            chunks[3],
            tr,
            tl,
            tw,
            key_state.right_bits(),
            key_state.left_bits(),
            key_state.word,
        );
    }
}

fn draw_keyboard(
    frame: &mut Frame,
    area: Rect,
    target_right: u8, // 5 bits
    target_left: u8,  // 4 bits
    target_word: bool,
    held_right: u8, // 5 bits
    held_left: u8,  // 4 bits
    held_word: bool,
) {
    // Labels depend on live state:
    //   word held   → phoneme mode
    //   word free   → roll/suffix mode
    // Thumb held on right-hand swaps consonants to their voiced pair.
    let phoneme_mode = held_word;
    let voiced = held_right & (1 << 4) != 0;

    // 10 cells left→right: L pinky/ring/middle/idx-outer/idx-inner,
    // R idx-inner/idx-outer/middle/ring/pinky. Inner-index cells are
    // placeholders for future digit-mode support.
    // Gradient from cyan-ish to yellow-ish for visual position hint.
    let key_colors = [
        Color::Rgb(0x66, 0xFF, 0xFF), //  0  L pinky
        Color::Rgb(0x77, 0xEE, 0xEE), //  1  L ring
        Color::Rgb(0x88, 0xDD, 0xDD), //  2  L middle
        Color::Rgb(0x99, 0xCC, 0xCC), //  3  L idx-outer
        Color::Rgb(0xAA, 0xBB, 0xBB), //  4  L idx-inner (future)
        Color::Rgb(0xBB, 0xAA, 0xAA), //  5  R idx-inner (future)
        Color::Rgb(0xCC, 0x99, 0x99), //  6  R idx-outer
        Color::Rgb(0xDD, 0x88, 0x88), //  7  R middle
        Color::Rgb(0xEE, 0x77, 0x77), //  8  R ring
        Color::Rgb(0xFF, 0x66, 0x66), //  9  R pinky
    ];
    let dot_colors = [
        Color::Rgb(0x33, 0x7F, 0x7F),
        Color::Rgb(0x3B, 0x77, 0x77),
        Color::Rgb(0x44, 0x6E, 0x6E),
        Color::Rgb(0x4C, 0x66, 0x66),
        Color::Rgb(0x55, 0x5D, 0x5D),
        Color::Rgb(0x5D, 0x55, 0x55),
        Color::Rgb(0x66, 0x4C, 0x4C),
        Color::Rgb(0x6E, 0x44, 0x44),
        Color::Rgb(0x77, 0x3B, 0x3B),
        Color::Rgb(0x7F, 0x33, 0x33),
    ];

    let left_labels: [&str; 5] = if phoneme_mode {
        ["æ", "ɛ", "ɪ", "ʌ", ""]
    } else {
        ["-'s", "-ed", "-ing", "-s", ""]
    };
    let right_labels: [&str; 5] = if phoneme_mode {
        if voiced {
            ["", "d", "z", "g", "b"]
        } else {
            ["", "t", "s", "k", "p"]
        }
    } else if voiced {
        ["", "there", "here", "my", "because"]
    } else {
        ["", "you", "and", "that", "for"]
    };

    // Cell → target bit mapping. None = inner-index placeholder (never lit yet).
    let left_bit = |i: usize| -> Option<u8> {
        match i { 0 => Some(3), 1 => Some(2), 2 => Some(1), 3 => Some(0), _ => None }
    };
    let right_bit = |i: usize| -> Option<u8> {
        match i { 1 => Some(0), 2 => Some(1), 3 => Some(2), 4 => Some(3), _ => None }
    };

    const CELL: usize = 9;
    let border_style = Style::default().fg(Color::Rgb(0x50, 0x50, 0x50));
    let dashes = "─".repeat(CELL);
    // Bordered box = the 4 resting finger keys per hand.
    let box_top = format!("┌{0}┬{0}┬{0}┬{0}┐", dashes);
    let box_bot = format!("└{0}┴{0}┴{0}┴{0}┘", dashes);
    // Inner-index cells sit between the two boxes with no outlines —
    // a bare colored strip in the "stretched" column slot.
    let empty_cell = " ".repeat(CELL);

    let mut lines: Vec<Line> = Vec::new();

    // Top border row (inner-index cells get blank space where their border would be)
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(box_top.clone(), border_style),
        Span::raw("  "),
        Span::raw(empty_cell.clone()),
        Span::raw("  "),
        Span::raw(empty_cell.clone()),
        Span::raw("  "),
        Span::styled(box_top, border_style),
    ]));

    // Label row — 4 bordered cells, then two unbordered inner cells, then 4 more.
    let make_cell_style = |i: usize, target: bool| -> Style {
        if target {
            let color = key_colors[i];
            Style::default().fg(color).bg(color)
        } else {
            Style::default().fg(Color::Rgb(0x40, 0x40, 0x40))
        }
    };

    let mut row: Vec<Span> = vec![Span::raw("  ")];
    for i in 0..4 {
        let target = left_bit(i).map(|b| target_left & (1 << b) != 0).unwrap_or(false);
        row.push(Span::styled("│", border_style));
        row.push(Span::styled(
            format!("{:^1$}", left_labels[i], CELL),
            make_cell_style(i, target),
        ));
    }
    row.push(Span::styled("│", border_style));
    // Left inner-index placeholder cell (no border)
    row.push(Span::raw("  "));
    row.push(Span::styled(
        format!("{:^1$}", left_labels[4], CELL),
        make_cell_style(4, false),
    ));
    row.push(Span::raw("  "));
    // Right inner-index placeholder cell
    row.push(Span::styled(
        format!("{:^1$}", right_labels[0], CELL),
        make_cell_style(5, false),
    ));
    row.push(Span::raw("  "));
    // Right bordered box
    row.push(Span::styled("│", border_style));
    for i in 1..5 {
        let target = right_bit(i).map(|b| target_right & (1 << b) != 0).unwrap_or(false);
        row.push(Span::styled(
            format!("{:^1$}", right_labels[i], CELL),
            make_cell_style(5 + i, target),
        ));
        row.push(Span::styled("│", border_style));
    }
    lines.push(Line::from(row));

    // Bottom border row
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(box_bot.clone(), border_style),
        Span::raw("  "),
        Span::raw(empty_cell.clone()),
        Span::raw("  "),
        Span::raw(empty_cell),
        Span::raw("  "),
        Span::styled(box_bot, border_style),
    ]));

    // Held row — dots coloured at half the key-colour brightness.
    let dot_span = |cell_idx: usize, held: bool| -> Span<'static> {
        let dot = if held { format!("{:^1$}", "●", CELL) } else { " ".repeat(CELL) };
        Span::styled(dot, Style::default().fg(dot_colors[cell_idx]))
    };
    let mut row2: Vec<Span> = vec![Span::raw("   ")];
    for i in 0..4 {
        let held = left_bit(i).map(|b| held_left & (1 << b) != 0).unwrap_or(false);
        row2.push(dot_span(i, held));
        row2.push(Span::raw(" "));
    }
    // Inner-index dots — no border spacing, just the dot + gap
    row2.push(Span::raw(" "));
    row2.push(dot_span(4, false)); // inner-L: never held yet
    row2.push(Span::raw("  "));
    row2.push(dot_span(5, false)); // inner-R: never held yet
    row2.push(Span::raw("  "));
    for i in 1..5 {
        let held = right_bit(i).map(|b| held_right & (1 << b) != 0).unwrap_or(false);
        row2.push(dot_span(5 + i, held));
        row2.push(Span::raw(" "));
    }
    lines.push(Line::from(row2));

    // Thumbs target — word = purple filled block, mod = cyan filled block.
    let word_active = Style::default()
        .fg(Color::Rgb(0x40, 0x00, 0x70))
        .bg(Color::Rgb(0x40, 0x00, 0x70));
    let mod_active = Style::default()
        .fg(Color::Rgb(0, 255, 0))
        .bg(Color::Rgb(0, 255, 0));
    let dim_style = Style::default().fg(Color::Rgb(0x40, 0x40, 0x40));
    let word_t = if target_word { word_active } else { dim_style };
    let mod_t = if target_right & (1 << 4) != 0 { mod_active } else { dim_style };

    // New layout total ≈ 108 cols. Word under left idx area, mod under right idx area.
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(38)),
        Span::styled(" word ", word_t),
        Span::raw(" ".repeat(22)),
        Span::styled(" mod ", mod_t),
    ]));

    // Thumbs held at half brightness, matching the dot style.
    let word_dot = Style::default().fg(Color::Rgb(0x20, 0x00, 0x38));
    let mod_dot = Style::default().fg(Color::Rgb(0, 0x7F, 0));
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(38)),
        Span::styled(if held_word { "  ●   " } else { "      " }, word_dot),
        Span::raw(" ".repeat(22)),
        Span::styled(
            if held_right & (1 << 4) != 0 { "  ●  " } else { "     " },
            mod_dot,
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}

// ─── Bench mode: measure chord speed per finger combo ───

pub fn run_bench() {
    terminal::enable_raw_mode().unwrap();
    io::stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

    let grab_enabled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let input = GrabInput::start_grab(grab_enabled).expect("failed to start key capture");

    let mut key_state = KeyState::default();

    use std::time::{Instant, SystemTime};
    use std::collections::HashMap;

    let mut right_chords: Vec<u8> = (1..32u8).collect(); // 1-31 (5 bits)
    let mut left_chords: Vec<u8> = (1..16u8).collect();  // 1-15 (4 bits)

    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    // Build sequence: all right-hand chords shuffled, then all left-hand shuffled
    fn build_round(right: &mut Vec<u8>, left: &mut Vec<u8>, seed: &mut u32) -> Vec<(char, u8)> {
        shuffle(right, *seed);
        *seed = seed.wrapping_mul(2654435761);
        shuffle(left, *seed);
        *seed = seed.wrapping_mul(2654435761);

        let mut seq = Vec::new();
        for &r in right.iter() {
            seq.push(('R', r));
        }
        for &l in left.iter() {
            seq.push(('L', l));
        }
        seq
    }

    let mut test_seq = build_round(&mut right_chords, &mut left_chords, &mut seed);
    let total = test_seq.len();
    let mut all_results: Vec<(char, u8, u128)> = Vec::new();
    let mut current = 0;
    let mut round = 1;
    let mut phase = BenchPhase::WaitChord;
    let mut chord_start = Instant::now();
    let mut accum: u8 = 0; // accumulated bits for target hand since chord started

    // Initial draw
    terminal
        .draw(|f| draw_bench(f, &test_seq, current, total, &all_results, &key_state, &phase, round))
        .ok();

    loop {
        let hid_event = match input.rx.recv() {
            Ok(ev) => ev,
            Err(_) => break,
        };
        let rhe_event = match hid_event {
            HidEvent::Quit => break,
            HidEvent::Key(ev) => ev,
        };

        update_key_state(&mut key_state, &rhe_event);

        let all_off = key_state.right_bits() == 0 && key_state.left_bits() == 0 && !key_state.word;

        if current >= total {
            // Round complete — reshuffle and start next round
            if all_off {
                test_seq = build_round(&mut right_chords, &mut left_chords, &mut seed);
                current = 0;
                round += 1;
                chord_start = Instant::now();
                accum = 0;
                phase = BenchPhase::WaitChord;
            }
            terminal
                .draw(|f| draw_bench(f, &test_seq, current, total, &all_results, &key_state, &phase, round))
                .ok();
            continue;
        }

        let (hand, target_bits) = test_seq[current];

        // Accumulate bits on key-down for target hand
        if phase == BenchPhase::Timing && rhe_event.direction == KeyDirection::Down {
            match (hand, rhe_event.key) {
                ('R', PhysicalKey::Finger(Hand::Right, _)) => {
                    accum |= key_state.right_bits();
                }
                ('L', PhysicalKey::Finger(Hand::Left, _)) => {
                    accum |= key_state.left_bits();
                }
                _ => {}
            }
        }

        match phase {
            BenchPhase::WaitClean => {
                // Error recovery: wait for all keys off, then restart
                if all_off {
                    chord_start = Instant::now();
                    accum = 0;
                    phase = BenchPhase::WaitChord;
                }
            }
            BenchPhase::WaitChord => {
                // Clock is running. First key down → start accumulating.
                if rhe_event.direction == KeyDirection::Down && rhe_event.key != PhysicalKey::Word {
                    // Accumulate this first press
                    match (hand, rhe_event.key) {
                        ('R', PhysicalKey::Finger(Hand::Right, _)) => {
                            accum |= key_state.right_bits();
                        }
                        ('L', PhysicalKey::Finger(Hand::Left, _)) => {
                            accum |= key_state.left_bits();
                        }
                        _ => {}
                    }
                    phase = BenchPhase::Timing;
                }
            }
            BenchPhase::Timing => {
                // All keys off → chord complete. Check accumulated bits.
                if all_off {
                    if accum == target_bits {
                        // Correct chord — save time (includes any fumble time), advance
                        let elapsed = chord_start.elapsed().as_millis();
                        all_results.push((hand, target_bits, elapsed));
                        current += 1;
                        // Reset clock for next chord
                        chord_start = Instant::now();
                        phase = BenchPhase::WaitChord;
                    }
                    // Wrong chord — reset accum, clock keeps running, try again
                    accum = 0;
                }
            }
            BenchPhase::WaitRelease => {}
        }

        terminal
            .draw(|f| draw_bench(f, &test_seq, current, total, &all_results, &key_state, &phase, round))
            .ok();
    }

    terminal::disable_raw_mode().ok();
    io::stdout().execute(LeaveAlternateScreen).ok();

    // Dump results
    if !all_results.is_empty() {
        // Average times per chord across all rounds
        let mut totals: HashMap<(char, u8), (u128, u32)> = HashMap::new();
        for &(hand, bits, ms) in &all_results {
            let entry = totals.entry((hand, bits)).or_insert((0, 0));
            entry.0 += ms;
            entry.1 += 1;
        }
        let mut averages: Vec<(char, u8, u128, u32)> = totals
            .iter()
            .map(|(&(h, b), &(total, count))| (h, b, total / count as u128, count))
            .collect();

        println!("\nChord benchmark — {} rounds, {} total measurements",
            round - 1, all_results.len());
        println!("{:<6} {:<8} {:<8} {:<6}", "Hand", "Chord", "Avg ms", "N");
        println!("{}", "─".repeat(32));

        println!("\n--- Right hand (fastest → slowest) ---");
        let mut right_avg: Vec<_> = averages.iter().filter(|(h, _, _, _)| *h == 'R').collect();
        right_avg.sort_by_key(|&&(_, _, avg, _)| avg);
        for &&(_, bits, avg, count) in &right_avg {
            println!("  {:05b}  {:<8}  {}ms  (n={})", bits, chord_label_right(bits), avg, count);
        }

        println!("\n--- Left hand (fastest → slowest) ---");
        let mut left_avg: Vec<_> = averages.iter().filter(|(h, _, _, _)| *h == 'L').collect();
        left_avg.sort_by_key(|&&(_, _, avg, _)| avg);
        for &&(_, bits, avg, count) in &left_avg {
            println!("  {:04b}  {:<8}  {}ms  (n={})", bits, chord_label_left(bits), avg, count);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum BenchPhase {
    WaitClean,
    WaitChord,
    Timing,
    WaitRelease,
}

fn shuffle(v: &mut Vec<u8>, mut seed: u32) {
    for i in (1..v.len()).rev() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let j = (seed as usize) % (i + 1);
        v.swap(i, j);
    }
}

fn chord_label_right(bits: u8) -> String {
    let names = ["I", "M", "R", "P", "T"];
    (0..5)
        .filter(|&i| bits & (1 << i) != 0)
        .map(|i| names[i])
        .collect::<Vec<_>>()
        .join("+")
}

fn chord_label_left(bits: u8) -> String {
    let names = ["I", "M", "R", "P"];
    (0..4)
        .filter(|&i| bits & (1 << i) != 0)
        .map(|i| names[i])
        .collect::<Vec<_>>()
        .join("+")
}

fn draw_bench(
    frame: &mut Frame,
    test_seq: &[(char, u8)],
    current: usize,
    total: usize,
    results: &[(char, u8, u128)],
    key_state: &KeyState,
    phase: &BenchPhase,
    round: usize,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // title
            Constraint::Length(3),  // progress (same slot as sentence)
            Constraint::Length(4),  // target detail (same slot as word detail)
            Constraint::Length(11), // keyboard (same as tutor)
            Constraint::Min(0),    // recent results
        ])
        .split(area);

    // Title
    frame.render_widget(
        Paragraph::new(" rhe bench  [Esc to quit]").style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );

    // Progress bar area (where sentence normally goes)
    let measured = results.len();
    let progress_text = if current >= total {
        format!(" Round {} complete! {} total measurements. Release all for next round, Esc to exit.", round - 1, measured)
    } else {
        format!(" Round {} — chord {}/{}  ({} total)", round, current + 1, total, measured)
    };
    frame.render_widget(
        Paragraph::new(progress_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::BOTTOM)),
        chunks[1],
    );

    // Target detail (where word detail normally goes)
    if current < total {
        let (hand, bits) = test_seq[current];
        let hand_name = if hand == 'R' { "RIGHT" } else { "LEFT" };
        let label = if hand == 'R' {
            chord_label_right(bits)
        } else {
            chord_label_left(bits)
        };
        let phase_str = match phase {
            BenchPhase::WaitClean => "release all keys",
            BenchPhase::WaitChord => "press now!",
            BenchPhase::Timing => "...",
            BenchPhase::WaitRelease => "release",
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{} hand: {}", hand_name, label),
                    Style::default().fg(Color::White).bold(),
                ),
            ]),
            Line::from(vec![Span::styled(
                format!(" {}", phase_str),
                Style::default().fg(Color::Gray),
            )]),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    " Complete!",
                    Style::default().fg(Color::White).bold(),
                )),
                Line::from(Span::styled(
                    " Results printed on exit",
                    Style::default().fg(Color::Gray),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );
    }

    // Keyboard — same as tutor
    if current < total {
        let (hand, bits) = test_seq[current];
        let (tr, tl) = match hand {
            'R' => (bits, 0u8),
            _ => (0u8, bits),
        };
        draw_keyboard(
            frame,
            chunks[3],
            tr,
            tl,
            false,
            key_state.right_bits(),
            key_state.left_bits(),
            key_state.word,
        );
    } else {
        draw_keyboard(frame, chunks[3], 0, 0, false, 0, 0, false);
    }

    // Recent results (bottom area)
    if !results.is_empty() {
        let recent: Vec<String> = results
            .iter()
            .rev()
            .take(8)
            .map(|(hand, bits, ms)| {
                let label = if *hand == 'R' {
                    format!("R {:<10}", chord_label_right(*bits))
                } else {
                    format!("L {:<10}", chord_label_left(*bits))
                };
                format!("  {} {}ms", label, ms)
            })
            .collect();
        frame.render_widget(
            Paragraph::new(recent.join("\n"))
                .block(Block::default().borders(Borders::TOP).title(" Recent ")),
            chunks[4],
        );
    }
}
