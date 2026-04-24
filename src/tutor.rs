//! Interactive typing tutor and bench mode.

use std::io;

use crossterm::ExecutableCommand;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::chord_map::{BriefTable, ChordKey, Phoneme, PhonemeTable};
use crate::hand::{KeyDirection, KeyEvent as RheKeyEvent};
use crate::input::HidEvent;
#[cfg(target_os = "linux")]
use crate::input::evdev_backend::EvdevInput as GrabInput;
#[cfg(target_os = "macos")]
use crate::input::iohid_backend::IoHidInput as GrabInput;
use crate::interpreter::Interpreter;
use crate::key_mask::KeyMask;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use crate::output::NullOutput as PlatformOutput;
use crate::output::TextOutput;
#[cfg(target_os = "linux")]
use crate::output::linux::LinuxOutput as PlatformOutput;
#[cfg(target_os = "macos")]
use crate::output::macos::MacOSOutput as PlatformOutput;
use crate::scan;
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
    /// Set of scancodes any of which is an acceptable lead finger for
    /// this ordered brief. Empty mask = no ordering constraint (phoneme
    /// steps, commit steps, unordered briefs). A word may list multiple
    /// leads (e.g. "four" on R-idx or R-mid); the tutor accepts any of
    /// them and brightens every one of those cells in the display.
    accepted_leads: KeyMask,
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

#[derive(Default)]
struct Step {
    target: Target,
    phoneme: Option<Phoneme>,
    /// Commit step — matches on word release (phoneme mode) or on
    /// all-off (brief mode). Any finger press during this step
    /// triggers the "finger during commit" reset (except for bounces
    /// of keys already in the prior chord).
    space_only: bool,
    /// Match on `Event::ModTap` instead of a chord. Used for number-
    /// mode entry (first tap of the sequence) and for the decimal
    /// point within a number sequence. Mutually exclusive with
    /// `space_only` and with a finger-valued `target`.
    mod_tap_only: bool,
    /// Hint text for the tutor's word-detail line — the one character
    /// this number-mode step emits ('3', '.', '+', etc.). None for
    /// non-number steps.
    number_glyph: Option<char>,
}

struct PracticeWord {
    word: String,
    phoneme_steps: Vec<Step>,        // word held + phoneme sequence + commit
    brief_steps: Option<Vec<Step>>,  // single chord without word + all-off (if brief exists)
    suffix_steps: Option<Vec<Step>>, // roll(base) + suffix chord + all-off (if shorter)
    suffix_label: Option<String>,    // e.g. "~ing" for display
    /// Steps for a number/symbol sequence: mod-tap entry, one step
    /// per character, word release to commit. Only populated when the
    /// word looks like a number or math expression.
    number_steps: Option<Vec<Step>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordMode {
    Brief,
    Phoneme,
    Suffix,
    Number,
}

#[derive(Default, Clone)]
struct KeyState {
    // [pinky, ring, middle, index, inner-index] — bits 3..=0 are the
    // home fingers, bit 4 is the inner-index (G). Only used by number
    // mode; zero in every other path.
    left: [bool; 5],
    // [index, middle, ring, pinky, thumb, inner-index] — bits 0..=3
    // fingers, bit 4 thumb/mod, bit 5 inner-index (H).
    right: [bool; 6],
    word: bool, // left ⌘
}

impl KeyState {
    fn left_bits(&self) -> u8 {
        (self.left[0] as u8) << 3
            | (self.left[1] as u8) << 2
            | (self.left[2] as u8) << 1
            | self.left[3] as u8
            | (self.left[4] as u8) << 4
    }

    fn right_bits(&self) -> u8 {
        self.right[0] as u8
            | (self.right[1] as u8) << 1
            | (self.right[2] as u8) << 2
            | (self.right[3] as u8) << 3
            | (self.right[4] as u8) << 4
            | (self.right[5] as u8) << 5
    }
}

struct Practice {
    sentences: Vec<Vec<PracticeWord>>,
    sentence_idx: usize,
    word_idx: usize,
    step_idx: usize,
    mode: WordMode,
    /// Set by `next_word` the instant it wraps the last-sentence
    /// boundary back to 0. The tutor main loop watches for this to
    /// trigger a double-buffer swap — if the prefetched Wikipedia
    /// article is ready, Practice is rebuilt from it. Cleared on any
    /// subsequent non-wrapping advance.
    wrapped: bool,
}

impl Practice {
    fn current_word(&self) -> Option<&PracticeWord> {
        self.sentences.get(self.sentence_idx)?.get(self.word_idx)
    }

    fn current_steps(&self) -> Option<&[Step]> {
        let word = self.current_word()?;
        match self.mode {
            WordMode::Brief => word
                .brief_steps
                .as_deref()
                .or(word.suffix_steps.as_deref())
                .or(Some(&word.phoneme_steps)),
            WordMode::Suffix => word.suffix_steps.as_deref().or(Some(&word.phoneme_steps)),
            WordMode::Phoneme => Some(&word.phoneme_steps),
            WordMode::Number => word.number_steps.as_deref(),
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
        self.wrapped = false;
        if let Some(sentence) = self.sentences.get(self.sentence_idx) {
            self.word_idx += 1;
            if self.word_idx >= sentence.len() {
                self.word_idx = 0;
                self.sentence_idx += 1;
                if self.sentence_idx >= self.sentences.len() {
                    self.sentence_idx = 0;
                    self.wrapped = true;
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
            // Pure-number words have no phoneme/brief path — go
            // straight into Number mode so the first draw already
            // shows digit labels and the mod-tap entry step.
            if w.number_steps.is_some() && w.phoneme_steps.is_empty() {
                WordMode::Number
            } else if w.brief_steps.is_some() {
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

pub fn run_tutor(test_mode: bool) {
    let cmudict = crate::data::load_cmudict();
    let freq_data = crate::data::load_word_freq();
    let lookup = WordLookup::new(&cmudict);
    let brief_table = crate::briefs::load_briefs();

    // Source selection: test mode uses the curated drill (deterministic
    // order, loops the same 24 sentences). Non-test uses a double-
    // buffered Wikipedia stream — one article blocking up front, next
    // article already prefetching in the background. On wraparound the
    // main loop swaps to the prefetched article and kicks off the
    // following one.
    let (initial_lines, stream) = if test_mode {
        (TEST_SENTENCES.iter().map(|s| s.to_string()).collect(), None)
    } else {
        let stream = crate::wiki::SentenceStream::new();
        let initial = stream.initial();
        let lines = if !initial.is_empty() {
            initial
        } else {
            ALICE_FALLBACK.iter().map(|s| s.to_string()).collect()
        };
        (lines, Some(stream))
    };
    let mut practice = build_practice(&lookup, &brief_table, initial_lines, test_mode);

    terminal::enable_raw_mode().unwrap();
    io::stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

    let grab_enabled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    #[cfg(target_os = "linux")]
    let input = GrabInput::start_grab(
        grab_enabled,
        crate::input::evdev_backend::QuitTrigger::EscOrCapsPlusEsc,
        None,
    )
    .expect("failed to start key capture");
    #[cfg(target_os = "macos")]
    let input = GrabInput::start_grab(grab_enabled, true).expect("failed to start key capture");

    // Real output pipeline: same events → state machine → interpreter → text injection
    let mut sm = StateMachine::new();
    let dict = PhonemeDictionary::build(&cmudict, &freq_data);
    let mut interp = Interpreter::new(PhonemeTable::new(), crate::briefs::load_briefs(), dict);
    let output = PlatformOutput::new();

    // Separate copies of the lookup tables for the adaptive key-cap display.
    // These get queried every frame with `held ∪ {cell}` to show the user
    // what each cell would produce as part of the current chord.
    let display_phonemes = PhonemeTable::new();
    let display_briefs = crate::briefs::load_briefs();

    let mut log: Vec<String> = Vec::new();
    let mut key_state = KeyState::default();
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
    // First chord-key scancode pressed during the current step. Set on
    // the first key-down when the touched-tracker was empty, cleared on
    // step transition. Used to verify ordered briefs — if the target's
    // `first_down` is Some(X), the user's first press must match X.
    let mut tutor_first_down: Option<u8> = None;
    let mut fingers_during_word = false;
    let mut last_was_botch = false; // true if last word output was IPA fallback

    // Initial draw
    terminal
        .draw(|f| {
            draw(
                f,
                &practice,
                false,
                &key_state,
                tutor_first_down,
                &display_phonemes,
                &display_briefs,
                interp.in_number_mode(),
            )
        })
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
            if let Some(bit) = scan::right_bit(rhe_event.scan) {
                chord_right_acc |= 1u8 << bit;
            } else if let Some(bit) = scan::left_bit(rhe_event.scan) {
                chord_left_acc |= 1u8 << bit;
            }
        }

        // Feed real output pipeline, and simultaneously watch for
        // ModTap — number-mode steps advance on that event, not on a
        // finger chord (the state machine suppresses the chord fire
        // for clean mod-taps, so chord matching would never see it).
        let mut step_advanced_by_modtap = false;
        for sm_event in sm.feed(rhe_event) {
            if matches!(sm_event, crate::state_machine::Event::ModTap) {
                let is_mod_tap_step = practice.current_step().map_or(false, |s| s.mod_tap_only);
                if is_mod_tap_step {
                    practice.advance_step();
                    step_advanced_by_modtap = true;
                    log.push("  → MATCH (mod-tap)".to_string());
                }
            }
            if let Some(action) = interp.process(&sm_event) {
                match action {
                    crate::interpreter::Action::Emit(text) => output.emit(&text),
                    crate::interpreter::Action::Backspace(n) => output.backspace(n),
                    crate::interpreter::Action::Suffix(text) => {
                        output.backspace(1); // remove trailing space
                        output.emit(&text); // emit suffix + space
                    }
                }
            }
        }

        let all_off = key_state.right_bits() == 0 && key_state.left_bits() == 0 && !key_state.word;

        // Log every key change
        let current_word_name = practice
            .current_word()
            .map(|w| w.word.as_str())
            .unwrap_or("?");
        let mode_str = match practice.mode {
            WordMode::Brief => "brief",
            WordMode::Suffix => "suf",
            WordMode::Phoneme => "phon",
            WordMode::Number => "num",
        };
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

            // Word key down → enter the word-held drill mode. For
            // number-shaped words that's Number (mod-tap entry),
            // otherwise the phoneme path.
            if rhe_event.scan == scan::WORD && rhe_event.direction == KeyDirection::Down {
                let is_number_word = practice.current_word().map_or(false, |w| {
                    w.number_steps.is_some() && w.phoneme_steps.is_empty()
                });
                practice.mode = if is_number_word {
                    WordMode::Number
                } else {
                    WordMode::Phoneme
                };
                practice.step_idx = 0;
                fingers_during_word = false;
            }
            // Word key up at step 0 (no progress) — solo word tap, or
            // the user entered a word-held mode and released without
            // making progress. Applies to both phoneme and number
            // drills.
            else if rhe_event.scan == scan::WORD
                && rhe_event.direction == KeyDirection::Up
                && matches!(practice.mode, WordMode::Phoneme | WordMode::Number)
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
            if key_state.word && is_key_down && rhe_event.scan != scan::WORD {
                fingers_during_word = true;
            }

            // ModTap already advanced the tutor past this event — skip
            // chord matching so the stale `touched_*` bits from the
            // thumb-down don't trip abandon detection. The step-
            // transition reseed at the bottom wipes those trackers.
            if step_advanced_by_modtap {
                // fall thru to reseed + draw
            } else if let Some(target) = practice.current_target() {
                let target = *target;
                let step = practice.current_step().unwrap();

                // Bounce-tolerance during commit steps: fingers mid-release
                // can momentarily re-press a key that was already part of
                // the just-matched chord. That's physical bounce, not a
                // fresh chord attempt. Only treat it as an error if the
                // pressed key wasn't in the prior step's target.
                let prev_target: Option<Target> = if practice.step_idx > 0 {
                    practice
                        .current_steps()
                        .and_then(|steps| steps.get(practice.step_idx - 1))
                        .map(|s| s.target)
                } else {
                    None
                };
                let bounce_of_prev = |scan: u8| -> bool {
                    let Some(prev) = prev_target else {
                        return false;
                    };
                    if let Some(bit) = crate::scan::left_bit(scan) {
                        prev.left & (1 << bit) != 0
                    } else if let Some(bit) = crate::scan::right_bit(scan) {
                        prev.right & (1 << bit) != 0
                    } else if scan == crate::scan::WORD {
                        prev.word
                    } else {
                        false
                    }
                };

                if step.mod_tap_only {
                    // Mod-tap steps advance on Event::ModTap (handled
                    // above, inside the sm.feed loop) — not on a
                    // chord. Thumb/word presses are part of the
                    // expected gesture, but any other finger press
                    // means the user reached for a digit instead of
                    // the mod key — reset the drill.
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && rhe_event.scan != scan::R_THUMB
                    {
                        log.push("  → RESET (finger during mod-tap)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
                        errored = true;
                    }
                } else if step.space_only {
                    // Phoneme commit step: word release advances, any
                    // finger is error — unless it's a bounce of a key
                    // from the phoneme chord that just matched.
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && !bounce_of_prev(rhe_event.scan)
                    {
                        log.push("  → RESET (finger during commit)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
                        errored = true;
                    } else if !key_state.word {
                        log.push("  → MATCH (commit)".to_string());
                        last_was_botch = false;
                        practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && !target.word {
                    // Brief commit step (all-off target): any new key
                    // outside the matched chord = error; bounce of a
                    // matched-chord key = ignored; all-off = advance.
                    if is_key_down && !bounce_of_prev(rhe_event.scan) {
                        log.push("  → RESET (finger during commit)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
                        errored = true;
                    } else if all_off {
                        log.push("  → MATCH (commit)".to_string());
                        last_was_botch = false;
                        practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && target.word {
                    // Release-to-neutral step between chord steps:
                    // all chord keys must come up before the next
                    // chord lights. Fresh finger presses reset (the
                    // user is supposed to be releasing, not typing);
                    // carryover releases (key-ups) drain until the
                    // hands are empty, then advance.
                    if is_key_down && rhe_event.scan != scan::WORD {
                        log.push("  → RESET (finger during release)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
                        errored = true;
                    } else if key_state.right_bits() == 0
                        && key_state.left_bits() == 0
                    {
                        log.push("  → MATCH (release)".to_string());
                        practice.advance_step();
                    }
                } else {
                    // Update "touched this step" trackers on each key-down.
                    // These differ from chord_*_acc because acc can be reseeded
                    // from carryover at step-transition time — we only want
                    // hand_touched to reflect *fresh* key-downs in this step,
                    // otherwise abandon fires on a pure release of carryover.
                    if is_key_down {
                        // Record the very first key pressed this step so we
                        // can enforce ordered-brief targets.
                        if tutor_first_down.is_none()
                            && (scan::right_bit(rhe_event.scan).is_some()
                                || scan::left_bit(rhe_event.scan).is_some())
                        {
                            tutor_first_down = Some(rhe_event.scan);
                        }
                        if let Some(bit) = scan::right_bit(rhe_event.scan) {
                            touched_right |= 1u8 << bit;
                        } else if let Some(bit) = scan::left_bit(rhe_event.scan) {
                            touched_left |= 1u8 << bit;
                        }
                    }

                    // hand_touched: user pressed a target-hand key in *this* step.
                    let hand_touched = (target.right != 0 && touched_right != 0)
                        || (target.left != 0 && touched_left != 0);

                    // "Target hands empty" — only the hands the target cares about.
                    let target_hands_empty = (target.right == 0 || key_state.right_bits() == 0)
                        && (target.left == 0 || key_state.left_bits() == 0);

                    // Match only checks hands named by the target. A carryover
                    // bit on a non-target hand is ignored (the user is between
                    // phonemes, still releasing their previous chord).
                    let acc_matches = (target.right == 0 || chord_right_acc == target.right)
                        && (target.left == 0 || chord_left_acc == target.left)
                        && key_state.word == target.word;

                    // Extra-key detection:
                    //   Target hand — any bit outside the target set is extra.
                    //   Non-target hand — HELD bits are carryover (still
                    //   releasing the previous chord), but FRESH key-downs
                    //   this step (tracked in touched_*) are wrong-hand
                    //   errors. `touched_*` is wiped on every step
                    //   transition, so it only ever contains fresh presses.
                    let has_extra_acc = (target.right != 0
                        && (chord_right_acc & !target.right) != 0)
                        || (target.left != 0 && (chord_left_acc & !target.left) != 0)
                        || (target.right == 0 && touched_right != 0)
                        || (target.left == 0 && touched_left != 0)
                        || (key_state.word && !target.word);

                    let space_dropped = rhe_event.scan == scan::WORD
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
                        last_was_botch = true;
                        if !all_off {
                            errored = true;
                        }
                    } else if acc_matches && hand_touched {
                        // Chord complete — union of pressed keys equals target.
                        // Ordered briefs also require the user's first-down
                        // key to be one of the target's accepted leads.
                        // Empty accepted_leads = no ordering constraint.
                        let first_down_ok = target.accepted_leads.is_empty()
                            || tutor_first_down
                                .map(|fd| target.accepted_leads.test(fd))
                                .unwrap_or(false);
                        if !first_down_ok {
                            log.push(format!(
                                "  → RESET (wrong lead: got {:?}, want one of {:?})",
                                tutor_first_down, target.accepted_leads
                            ));
                            practice.reset_word();
                            last_was_botch = true;
                            if !all_off {
                                errored = true;
                            }
                        } else {
                            // Advances to the following commit step where the user
                            // must release everything to finalise the word.
                            log.push("  → MATCH".to_string());
                            practice.advance_step();
                        }
                    } else if hand_touched && target_hands_empty && !is_key_down {
                        log.push("  → RESET (chord abandoned)".to_string());
                        practice.reset_word();
                        last_was_botch = true;
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
            // `first_down` is scoped to the current step too — whatever
            // the user led with on the prior step is irrelevant to the
            // next one.
            tutor_first_down = None;
        }

        // Hand-zero accumulator reset runs AFTER the step handler so abandon
        // detection still sees the partial-attempt bits on the final key-up.
        if key_state.right_bits() == 0 {
            chord_right_acc = 0;
        }
        if key_state.left_bits() == 0 {
            chord_left_acc = 0;
        }
        if key_state.right_bits() == 0 && key_state.left_bits() == 0 {
            tutor_first_down = None;
        }

        // Double-buffer swap: when the practice wraps past its last
        // sentence AND the background fetch has a fresh article ready,
        // rebuild the practice from it. If nothing's ready yet we
        // repeat the current batch — the next wrap will try again.
        if practice.wrapped {
            if let Some(stream) = &stream {
                if let Some(new_lines) = stream.try_next() {
                    practice = build_practice(&lookup, &brief_table, new_lines, false);
                }
            }
            practice.wrapped = false;
        }

        // Draw after every event
        terminal
            .draw(|f| {
                draw(
                    f,
                    &practice,
                    errored,
                    &key_state,
                    tutor_first_down,
                    &display_phonemes,
                    &display_briefs,
                    interp.in_number_mode(),
                )
            })
            .ok();
    }

    terminal::disable_raw_mode().ok();
    io::stdout().execute(LeaveAlternateScreen).ok();

    let log_path = "tutor_debug.log";
    std::fs::write(log_path, log.join("\n")).ok();
    println!("Debug log written to {}", log_path);
}

// ─── Build practice steps ───

/// Curated drill lines used by `rhe test`. Reproducible, offline,
/// short enough to cycle thru while iterating on chord designs.
const TEST_SENTENCES: &[&str] = &[
    "count 0 1 2 3 4 5 6 7 8 9 and then stop",
    "the answer is 42 and pi is about 3.14 today",
    "add 1+2 and 7+8 to get 3 and 15 as a result",
    "try 9-4 and 6-1 or 100-50 just for practice",
    "compute 2*3 and 4*5 to get 6 and 20 quickly",
    "divide 10/2 and 20/4 for fun with numbers",
    "use parens like (1+2)*3 and 2*(3+4) here",
    "set x=5 and y=10 then x+y=15 is correct",
    "enter the list 1,2,3 and 7,8,9 carefully",
    "compute 2^3 and 5^2 for small power values",
    "type 50% and 75% for progress bar numbers",
    "you and the to too two tests for four and fore here",
    "i would not know if you could read this but i will try",
    "we went to the store to buy a new book but bye for now",
    "here hear me out i need to see the sea clearly",
    "write what is right then we can hear here again",
    "there is something over there their book is here",
    "four is the number for sure and not just fore",
    "in the inn we would like to find some food",
    "he will do what is due when it is time",
    "the butt of the joke is but a small thing",
    "you will knot the rope i know you will",
    "i can be busy like a bee all day long",
    "our hour of practice is almost done now",
    "where will you wear that fine new jacket",
    "i knew about the new car before you did",
    "this week i feel weak but i will push thru",
    "you would find wood by the stream nearby",
    "the whole team found the hole in the wall",
    "last night the knight won the fight easily",
    "he felt through the door and threw it open",
    "which witch is which i cannot tell which",
    "i had to wait for the weight to settle down",
    "the son watches the sun rise each morning",
    "we will meet for a piece of meat tonight",
];

/// Opening-of-Alice-in-Wonderland fallback used when the network is
/// unavailable and no Wikipedia article could be fetched.
const ALICE_FALLBACK: &[&str] = &[
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

/// Map a digit/symbol glyph to the target bits that would produce it in
/// number mode. Returns (right_bits, left_bits, needs_mod) — the tutor
/// shifts right by bit 5 for inner-index and ORs bit 4 for the mod/thumb.
/// `None` for any glyph that isn't a number-mode output.
fn number_char_target(c: char) -> Option<(u8, u8, bool)> {
    // Position layout mirrors `number_data::position`: R-pinky=0,
    // R-ring=1, R-mid=2, R-idx=3, R-inner=4, L-inner=5, L-idx=6,
    // L-mid=7, L-ring=8, L-pinky=9. Symbols share position with the
    // digit at the same slot but require mod.
    let (pos, is_symbol) = match c {
        '0' => (0, false),
        '-' => (0, true),
        '1' => (1, false),
        '/' => (1, true),
        '2' => (2, false),
        '*' => (2, true),
        '3' => (3, false),
        '+' => (3, true),
        '4' => (4, false),
        ')' => (4, true),
        '5' => (5, false),
        '(' => (5, true),
        '6' => (6, false),
        '=' => (6, true),
        '7' => (7, false),
        '%' => (7, true),
        '8' => (8, false),
        '^' => (8, true),
        '9' => (9, false),
        ',' => (9, true),
        _ => return None,
    };
    // right bits 0..=3 = index..pinky (home), bit 5 = inner-index.
    // left bits  0..=3 = index..pinky (home), bit 4 = inner-index.
    let (right, left) = match pos {
        0 => (1u8 << 3, 0u8), // R-pinky
        1 => (1 << 2, 0),     // R-ring
        2 => (1 << 1, 0),     // R-mid
        3 => (1 << 0, 0),     // R-idx
        4 => (1 << 5, 0),     // R-inner
        5 => (0, 1u8 << 4),   // L-inner
        6 => (0, 1 << 0),     // L-idx
        7 => (0, 1 << 1),     // L-mid
        8 => (0, 1 << 2),     // L-ring
        9 => (0, 1 << 3),     // L-pinky
        _ => unreachable!(),
    };
    Some((right, left, is_symbol))
}

/// Build the per-step drill sequence for a number/symbol "word".
/// Structure: one mod-tap entry step (to switch into number mode),
/// then one step per character (digit chord, decimal = mod-tap, or
/// symbol = chord + mod), finally a word-release commit that emits
/// the trailing space and exits number mode. Returns `None` if the
/// word contains a character with no number-mode mapping, or has no
/// digit at all (so we don't accidentally drill words like "=").
/// Build number-mode steps for spelled-out digit words ("one" through "nine").
/// Generates: mod-tap entry → finger+mod chord (finger first = spelled word) → commit.
fn build_digit_word_steps(word: &str) -> Option<Vec<Step>> {
    let lower = word.to_lowercase();
    let scan_code = match lower.as_str() {
        "zero"  => scan::R_PINKY,
        "one"   => scan::R_RING,
        "two"   => scan::R_MID,
        "three" => scan::R_IDX,
        "four"  => scan::R_IDX_INNER,
        "five"  => scan::L_IDX_INNER,
        "six"   => scan::L_IDX,
        "seven" => scan::L_MID,
        "eight" => scan::L_RING,
        "nine"  => scan::L_PINKY,
        _ => return None,
    };

    // Figure out right/left bits for the target
    let (right, left) = if let Some(bit) = scan::right_bit(scan_code) {
        (1u8 << bit | (1 << 4), 0u8) // finger + mod (thumb)
    } else if let Some(bit) = scan::left_bit(scan_code) {
        // Left-hand digit: mod is right thumb, finger is left hand
        // Target shows left finger + right mod
        (1u8 << 4, 1u8 << bit)
    } else {
        return None;
    };

    // Finger-first ordering: the finger scancode is the accepted lead
    let mut leads = KeyMask::EMPTY;
    leads.set(scan_code);

    let mut steps = Vec::new();

    // Step 1: mod-tap entry (enter number mode)
    steps.push(Step {
        target: Target {
            right: 1 << 4,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        mod_tap_only: true,
        number_glyph: None,
        ..Step::default()
    });

    // Step 2: finger+mod chord (finger pressed first = spelled word)
    steps.push(Step {
        target: Target {
            right,
            left,
            word: true,
            accepted_leads: leads,
        },
        number_glyph: Some(word.chars().next().unwrap_or('?')),
        ..Step::default()
    });

    // Step 3: release
    steps.push(Step {
        target: Target {
            right: 0,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    });

    // Step 4: commit (word release)
    steps.push(Step {
        target: Target::default(),
        space_only: true,
        ..Step::default()
    });

    Some(steps)
}

fn build_number_steps(word: &str) -> Option<Vec<Step>> {
    if !word.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // Validate all chars up front — reject the whole word on any
    // unsupported glyph rather than silently dropping characters.
    for c in word.chars() {
        if c != '.' && number_char_target(c).is_none() {
            return None;
        }
    }

    let mut steps: Vec<Step> = Vec::new();

    // Mod-tap target: word held + mod thumb bit set. Bit 4 of
    // target.right is the mod slot, so setting it lights up the mod
    // cell through the normal target-highlight path — mod-tap steps
    // look identical to any finger step. Match is still gated on
    // Event::ModTap (the `mod_tap_only` flag routes chord-matching
    // through a branch that doesn't auto-advance on thumb-down).
    let mod_tap_target = Target {
        right: 1 << 4,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };
    let release_step = || Step {
        target: Target {
            right: 0,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    };

    steps.push(Step {
        target: mod_tap_target,
        mod_tap_only: true,
        number_glyph: None,
        ..Step::default()
    });

    // `prev_needs_release` tracks whether the previous step left the
    // user with chord keys held (digit/symbol chord press). Mod-tap
    // steps advance on thumb-up, so they leave the hand neutral and
    // don't need a release beat after them.
    let mut prev_needs_release = false;
    for c in word.chars() {
        if c == '.' {
            if prev_needs_release {
                steps.push(release_step());
            }
            steps.push(Step {
                target: mod_tap_target,
                mod_tap_only: true,
                number_glyph: Some('.'),
                ..Step::default()
            });
            prev_needs_release = false;
            continue;
        }
        if prev_needs_release {
            steps.push(release_step());
        }
        let (right, left, needs_mod) = number_char_target(c).unwrap();
        let adjusted_right = right | if needs_mod { 1 << 4 } else { 0 };
        steps.push(Step {
            target: Target {
                right: adjusted_right,
                left,
                word: true,
                accepted_leads: KeyMask::EMPTY,
            },
            number_glyph: Some(c),
            ..Step::default()
        });
        prev_needs_release = true;
    }

    // Commit: word release emits the trailing space and drops back to
    // normal mode. `space_only` reuses the phoneme-mode commit path.
    steps.push(Step {
        target: Target::default(),
        space_only: true,
        ..Step::default()
    });

    Some(steps)
}

fn build_practice(
    lookup: &WordLookup,
    brief_table: &BriefTable,
    lines: Vec<String>,
    deterministic_start: bool,
) -> Practice {
    let common_text = lines;

    let mut sentences: Vec<Vec<PracticeWord>> = Vec::new();
    // Record where each source line starts in `sentences` so random-start
    // lands on a line boundary rather than mid-wiki-sentence.
    let mut line_starts: Vec<usize> = Vec::new();

    for line in &common_text {
        line_starts.push(sentences.len());
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut skipped = Vec::new();

        for group in words.chunks(8) {
            let mut sentence: Vec<PracticeWord> = Vec::new();

            for &word_str in group {
                // Number-shaped words (digits, optionally with symbols
                // from the number-mode set) bypass phoneme/brief
                // lookup entirely. They drill number-mode only.
                if let Some(number_steps) = build_number_steps(word_str) {
                    sentence.push(PracticeWord {
                        word: word_str.to_string(),
                        phoneme_steps: Vec::new(),
                        brief_steps: None,
                        suffix_steps: None,
                        suffix_label: None,
                        number_steps: Some(number_steps),
                    });
                    continue;
                }

                // Spelled digit words → number-mode steps (mod-tap → finger+mod)
                if let Some(number_steps) = build_digit_word_steps(word_str) {
                    sentence.push(PracticeWord {
                        word: word_str.to_string(),
                        phoneme_steps: Vec::new(),
                        brief_steps: None,
                        suffix_steps: None,
                        suffix_label: None,
                        number_steps: Some(number_steps),
                    });
                    continue;
                }

                let clean: String = word_str
                    .chars()
                    .filter(|c| c.is_alphabetic() || *c == '\'')
                    .flat_map(|c| c.to_lowercase())
                    .collect();

                let Some(phonemes) = lookup.lookup(&clean) else {
                    skipped.push(clean);
                    continue;
                };

                // Build phoneme steps (word held). Insert a release
                // step between every pair of chord steps so the user
                // gets an explicit "hands neutral" beat before the
                // next chord lights — matches the physical rhythm of
                // roll-press-release-roll, and avoids carryover-
                // masking the next target.
                let release_step = || Step {
                    target: Target {
                        right: 0,
                        left: 0,
                        word: true,
                        accepted_leads: KeyMask::EMPTY,
                    },
                    ..Step::default()
                };

                let mut phoneme_steps: Vec<Step> = Vec::new();
                for (i, &phoneme) in phonemes.iter().enumerate() {
                    if i > 0 {
                        phoneme_steps.push(release_step());
                    }
                    let key = phoneme.chord_key();
                    let right = key.right_bits() | if key.has_mod() { 1 << 4 } else { 0 };
                    let left = key.left_bits();
                    phoneme_steps.push(Step {
                        target: Target {
                            right,
                            left,
                            word: true,
                            accepted_leads: KeyMask::EMPTY,
                        },
                        phoneme: Some(phoneme),
                        ..Step::default()
                    });
                }
                phoneme_steps.push(Step {
                    target: Target {
                        word: false,
                        ..Target::default()
                    },
                    phoneme: None,
                    space_only: true,
                    mod_tap_only: false,
                    number_glyph: None,
                });

                // Build brief steps (no word key) if brief exists. A
                // word may have multiple ordered entries on the same
                // chord (e.g. "four" accepts either R-idx or R-mid as
                // lead); collect every acceptable lead scancode.
                let brief_steps = {
                    let mut chord_for_word: Option<(u8, u8)> = None;
                    let mut leads = KeyMask::EMPTY;
                    for (key, first_down, brief_word) in brief_table.iter() {
                        if brief_word.trim() != clean {
                            continue;
                        }
                        let right = key.right_bits() | if key.has_mod() { 1u8 << 4 } else { 0 };
                        let left = key.left_bits();
                        match chord_for_word {
                            None => chord_for_word = Some((right, left)),
                            Some(existing) if existing != (right, left) => continue,
                            _ => {}
                        }
                        if let Some(fd) = first_down {
                            leads.set(fd);
                        }
                    }
                    chord_for_word.map(|(right, left)| {
                        vec![
                            Step {
                                target: Target {
                                    right,
                                    left,
                                    word: false,
                                    accepted_leads: leads,
                                },
                                phoneme: None,
                                space_only: false,
                                mod_tap_only: false,
                                number_glyph: None,
                            },
                            Step {
                                target: Target::default(), // all off
                                phoneme: None,
                                space_only: false,
                                mod_tap_only: false,
                                number_glyph: None,
                            },
                        ]
                    })
                };

                // Build suffix steps: roll(base) + suffix chord + all-off
                let (suffix_steps, suffix_label) = {
                    use crate::suffixes_data::SUFFIXES;
                    let phoneme_count =
                        phoneme_steps.iter().filter(|s| s.phoneme.is_some()).count();
                    let mut found = (None, None);

                    // Only worth it if phoneme path is 3+ movements
                    if phoneme_count > 2 {
                        // Try suffixes longest first
                        let mut sorted_suffixes: Vec<(u8, &str)> = SUFFIXES.to_vec();
                        sorted_suffixes.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                        for &(suffix_bits, suffix_str) in &sorted_suffixes {
                            if !clean.ends_with(suffix_str) {
                                continue;
                            }
                            if clean.len() <= suffix_str.len() {
                                continue;
                            }

                            // Try base and base+"e"
                            let bases = vec![
                                clean[..clean.len() - suffix_str.len()].to_string(),
                                format!("{}e", &clean[..clean.len() - suffix_str.len()]),
                            ];

                            for base in &bases {
                                // Find roll for base. Collect every
                                // acceptable lead (multiple ordered
                                // entries may share a chord for the
                                // same word).
                                let mut base_chord: Option<(u8, u8)> = None;
                                let mut base_leads = KeyMask::EMPTY;
                                for (key, first_down, brief_word) in brief_table.iter() {
                                    if brief_word.trim() != base {
                                        continue;
                                    }
                                    let r =
                                        key.right_bits() | if key.has_mod() { 1u8 << 4 } else { 0 };
                                    let l = key.left_bits();
                                    match base_chord {
                                        None => base_chord = Some((r, l)),
                                        Some(existing) if existing != (r, l) => continue,
                                        _ => {}
                                    }
                                    if let Some(fd) = first_down {
                                        base_leads.set(fd);
                                    }
                                }
                                if let Some((r, l)) = base_chord {
                                    let steps = vec![
                                        Step {
                                            target: Target {
                                                right: r,
                                                left: l,
                                                word: false,
                                                accepted_leads: base_leads,
                                            },
                                            phoneme: None,
                                            space_only: false,
                                            mod_tap_only: false,
                                            number_glyph: None,
                                        },
                                        Step {
                                            target: Target::default(),
                                            phoneme: None,
                                            space_only: false,
                                            mod_tap_only: false,
                                            number_glyph: None,
                                        },
                                        Step {
                                            target: Target {
                                                right: 0,
                                                left: suffix_bits,
                                                word: false,
                                                accepted_leads: KeyMask::EMPTY,
                                            },
                                            phoneme: None,
                                            space_only: false,
                                            mod_tap_only: false,
                                            number_glyph: None,
                                        },
                                        Step {
                                            target: Target::default(),
                                            phoneme: None,
                                            space_only: false,
                                            mod_tap_only: false,
                                            number_glyph: None,
                                        },
                                    ];
                                    found = (Some(steps), Some(format!("~{}", suffix_str)));
                                    break;
                                }
                            }
                            if found.0.is_some() {
                                break;
                            }
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
                    number_steps: None,
                });
            }

            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }

        // skipped words with no CMU entry are silently dropped
    }

    // Pick a random starting sentence from the set of line-start indices,
    // so successive launches (a) don't always begin at the first line and
    // (b) always start at the first chunk of some wiki sentence, not in
    // the middle of one. next_word() wraps around to cover the rest.
    let valid_starts: Vec<usize> = line_starts
        .into_iter()
        .filter(|&idx| idx < sentences.len())
        .collect();
    // Test mode starts deterministically at sentence 0 so drill order
    // is reproducible run-to-run. Normal mode randomizes so the user
    // doesn't start at the same page every session.
    let sentence_idx = if valid_starts.is_empty() {
        0
    } else if deterministic_start {
        valid_starts[0]
    } else {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        valid_starts[(seed as usize) % valid_starts.len()]
    };

    let first = sentences.get(sentence_idx).and_then(|s| s.first());
    let initial_mode = match first {
        Some(w) if w.number_steps.is_some() && w.phoneme_steps.is_empty() => WordMode::Number,
        Some(w) if w.brief_steps.is_some() => WordMode::Brief,
        Some(w) if w.suffix_steps.is_some() => WordMode::Suffix,
        _ => WordMode::Phoneme,
    };

    Practice {
        sentences,
        sentence_idx,
        word_idx: 0,
        step_idx: 0,
        mode: initial_mode,
        wrapped: false,
    }
}

// ─── Key state update ───

fn update_key_state(state: &mut KeyState, event: &RheKeyEvent) {
    let pressed = event.direction == KeyDirection::Down;
    match event.scan {
        scan::L_PINKY => state.left[0] = pressed,
        scan::L_RING => state.left[1] = pressed,
        scan::L_MID => state.left[2] = pressed,
        scan::L_IDX => state.left[3] = pressed,
        scan::L_IDX_INNER => state.left[4] = pressed,
        scan::R_IDX => state.right[0] = pressed,
        scan::R_MID => state.right[1] = pressed,
        scan::R_RING => state.right[2] = pressed,
        scan::R_PINKY => state.right[3] = pressed,
        scan::R_THUMB => state.right[4] = pressed,
        scan::R_IDX_INNER => state.right[5] = pressed,
        scan::WORD => state.word = pressed,
        _ => {}
    }
}

// ─── Drawing ───

fn draw(
    frame: &mut Frame,
    practice: &Practice,
    errored: bool,
    key_state: &KeyState,
    // The scancode the user actually pressed first in this chord
    // attempt (same value tracked by the tutor's main loop). Used by
    // the adaptive cell labels to look up ordered briefs correctly
    // when the user is mid-chord.
    tutor_first_down: Option<u8>,
    phonemes: &PhonemeTable,
    briefs: &BriefTable,
    in_number_mode: bool,
) {
    let area = frame.area();

    // Force solid black background on all terminals
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0))),
        area,
    );

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
        Paragraph::new(" rhe tutor  [Esc to quit]")
            .style(Style::default().fg(Color::Rgb(0, 255, 255))),
        chunks[0],
    );

    // Sentence
    if let Some(sentence) = practice.sentences.get(practice.sentence_idx) {
        let mut spans = vec![Span::raw(" ")];
        for (i, w) in sentence.iter().enumerate() {
            let style = if i == practice.word_idx {
                Style::default()
                    .fg(Color::Rgb(255, 255, 255))
                    .bold()
                    .underlined()
            } else if i < practice.word_idx {
                Style::default().fg(Color::Rgb(80, 80, 80))
            } else {
                Style::default().fg(Color::Rgb(160, 160, 160))
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
        // Number-mode steps carry a glyph. Digits/symbols/decimal use
        // their literal character; the mod-tap entry step (no glyph)
        // shows `#` as a stand-in so there's always *something* in
        // the hint line. Other steps fall back to the phoneme IPA.
        let step = practice.current_step();
        let phoneme_label = if let Some(glyph) = step.and_then(|s| s.number_glyph) {
            format!(" {}", glyph)
        } else if step.map_or(false, |s| s.mod_tap_only) {
            " #".to_string()
        } else {
            step.and_then(|s| s.phoneme)
                .map(|p| format!(" {}", p.to_ipa()))
                .unwrap_or_default()
        };

        let mut word_spans = vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                &pw.word,
                Style::default().fg(Color::Rgb(255, 255, 255)).bold(),
            ),
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
                Style::default().fg(Color::Rgb(160, 160, 160)),
            )]),
        ];

        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );

        // Keyboard: all dark when errored, otherwise show target.
        let (tr, tl, tw, tleads) = if errored {
            (0u8, 0u8, false, KeyMask::EMPTY)
        } else if let Some(step) = practice.current_step() {
            (
                step.target.right,
                step.target.left,
                step.target.word,
                step.target.accepted_leads,
            )
        } else {
            (0, 0, false, KeyMask::EMPTY)
        };
        draw_keyboard(
            frame,
            chunks[3],
            tr,
            tl,
            tw,
            tleads,
            key_state.right_bits(),
            key_state.left_bits(),
            key_state.word,
            tutor_first_down,
            Some((phonemes, briefs)),
            in_number_mode,
        );
    }
}

fn draw_keyboard(
    frame: &mut Frame,
    area: Rect,
    target_right: u8, // 6 bits (bit 5 = R-inner-index)
    target_left: u8,  // 5 bits (bit 4 = L-inner-index)
    target_word: bool,
    // For ordered briefs, the set of scancodes that are acceptable
    // first-down leads. Cells for these keys render at full brightness;
    // other target cells dim to the dot colour as a "press one of the
    // bright ones first" cue. Empty mask = no ordering constraint.
    target_accepted_leads: KeyMask,
    held_right: u8, // 6 bits (bit 5 = R-inner-index)
    held_left: u8,  // 5 bits (bit 4 = L-inner-index)
    held_word: bool,
    // Live first-down key the user has pressed for the current chord
    // attempt. Passed to the brief lookup for adaptive labels so
    // claimed (ordered) chords resolve to the correct word mid-roll.
    user_first_down: Option<u8>,
    // Adaptive labels: when provided, every cell shows the phoneme/brief
    // that would fire if that cell's key were added to the currently-held
    // chord. None = hardcoded fallback labels (used by bench mode).
    label_tables: Option<(&PhonemeTable, &BriefTable)>,
    // In number sub-session: labels switch to digit (or symbol when the
    // mod bit is in the candidate chord).
    in_number_mode: bool,
) {
    // Labels depend on live state:
    //   word held   → phoneme mode (look up PhonemeTable)
    //   word free   → brief/suffix mode (look up BriefTable)
    // Each cell shows what `held ∪ {this_cell}` produces — so an unheld
    // cell advertises what it would add to the current chord, and a held
    // cell shows the current chord's output. Bench mode passes `None`
    // and falls back to the static placeholder strings.

    // 10 cells left→right: L pinky/ring/middle/idx-outer/idx-inner,
    // R idx-inner/idx-outer/middle/ring/pinky. Inner-index cells are
    // placeholders for future digit-mode support.
    // Gradient from cyan-ish to yellow-ish for visual position hint.
    let key_colors = [
        Color::Rgb(0x60, 0xA8, 0xF0), //  0  L pinky
        Color::Rgb(0x70, 0xA8, 0xE0), //  1  L ring
        Color::Rgb(0x80, 0xA8, 0xD0), //  2  L middle
        Color::Rgb(0x90, 0xA8, 0xC0), //  3  L idx-outer
        Color::Rgb(0xA0, 0xA8, 0xB0), //  4  L idx-inner (future)
        Color::Rgb(0xB0, 0xA8, 0xA0), //  5  R idx-inner (future)
        Color::Rgb(0xC0, 0xA8, 0x90), //  6  R idx-outer
        Color::Rgb(0xD0, 0xA8, 0x80), //  7  R middle
        Color::Rgb(0xE0, 0xA8, 0x70), //  8  R ring
        Color::Rgb(0xF0, 0xA8, 0x60), //  9  R pinky
    ];
    let dot_colors = [
        Color::Rgb(0x30, 0x54, 0x78),
        Color::Rgb(0x38, 0x54, 0x70),
        Color::Rgb(0x40, 0x54, 0x68),
        Color::Rgb(0x48, 0x54, 0x60),
        Color::Rgb(0x50, 0x54, 0x58),
        Color::Rgb(0x58, 0x54, 0x50),
        Color::Rgb(0x60, 0x54, 0x48),
        Color::Rgb(0x68, 0x54, 0x40),
        Color::Rgb(0x70, 0x54, 0x38),
        Color::Rgb(0x78, 0x54, 0x30),
    ];

    // Build held KeyMask for candidate-chord lookups. Word key isn't in
    // the chord keymap (it's a mode selector, not a chord bit), so it's
    // handled separately via `held_word`.
    let held_mask = {
        let mut m = KeyMask::EMPTY;
        const L_SCANS: [u8; 4] = [scan::L_IDX, scan::L_MID, scan::L_RING, scan::L_PINKY];
        const R_SCANS: [u8; 4] = [scan::R_IDX, scan::R_MID, scan::R_RING, scan::R_PINKY];
        for (bit, s) in L_SCANS.iter().enumerate() {
            if held_left & (1 << bit) != 0 {
                m.set(*s);
            }
        }
        for (bit, s) in R_SCANS.iter().enumerate() {
            if held_right & (1 << bit) != 0 {
                m.set(*s);
            }
        }
        if held_right & (1 << 4) != 0 {
            m.set(scan::R_THUMB);
        }
        m
    };

    // Cell width in columns — labels longer than this get tail-trimmed
    // so the keyboard layout stays aligned.
    const CELL_LABEL_MAX: usize = 9;
    let trim = |s: String| -> String {
        if s.chars().count() > CELL_LABEL_MAX {
            s.chars().take(CELL_LABEL_MAX).collect()
        } else {
            s
        }
    };

    let lookup_label = |cell_scan: u8| -> String {
        let Some((phonemes, briefs)) = label_tables else {
            return String::new();
        };
        // In number mode, each finger position maps to a single digit
        // (or symbol, when mod is part of the candidate chord). Use
        // only the cell's own scancode for the lookup — number mode
        // doesn't chord, so "held ∪ {cell}" labels would just be noise.
        if in_number_mode {
            let mod_held = held_mask.test(scan::R_THUMB);
            let mut candidate = KeyMask::EMPTY;
            candidate.set(cell_scan);
            if mod_held {
                candidate.set(scan::R_THUMB);
            }
            let chord = ChordKey::from_mask(candidate);
            let c = if mod_held {
                crate::number_data::chord_to_symbol(chord)
            } else {
                crate::number_data::chord_to_digit(chord)
            };
            return c.map(|ch| ch.to_string()).unwrap_or_default();
        }
        // Phoneme mode fires each hand independently — left hand's label
        // must only see left-hand held bits, and vice versa, otherwise a
        // right-hand chord in progress would blank out the left-hand
        // hints (and distort any remaining left-hand labels). Brief mode
        // is a single combined chord, so all held bits contribute.
        let base = if held_word {
            if scan::LEFT_MASK.test(cell_scan) {
                held_mask & scan::LEFT_MASK
            } else if scan::RIGHT_MASK.test(cell_scan) {
                held_mask & scan::RIGHT_MASK
            } else {
                KeyMask::EMPTY
            }
        } else {
            held_mask
        };
        let mut candidate = base;
        candidate.set(cell_scan);
        let chord = ChordKey::from_mask(candidate);
        // For ordered-brief lookup: if the user already started a chord
        // (user_first_down is Some), that's the lead. If nothing is
        // held, pressing this cell would make IT the lead — so the
        // hypothetical lookup uses the cell's own scancode. That way a
        // resting ordered chord still labels its cells with the word
        // each finger-first would fire.
        let lookup_first = if base.is_empty() {
            Some(cell_scan)
        } else {
            user_first_down
        };
        let raw = if held_word {
            phonemes
                .lookup(chord)
                .map(|p| p.to_ipa().to_string())
                .unwrap_or_default()
        } else if let Some(entry) = briefs.lookup(chord, lookup_first) {
            if let Some(suffix) = entry.strip_prefix('\x01') {
                format!("-{}", suffix.trim_end())
            } else {
                entry.trim_end().to_string()
            }
        } else {
            String::new()
        };
        trim(raw)
    };

    // Cell index → scancode for the 10 cells in display order
    // (left pinky thru right pinky).
    const CELL_SCANS: [u8; 10] = [
        scan::L_PINKY,
        scan::L_RING,
        scan::L_MID,
        scan::L_IDX,
        scan::L_IDX_INNER,
        scan::R_IDX_INNER,
        scan::R_IDX,
        scan::R_MID,
        scan::R_RING,
        scan::R_PINKY,
    ];
    let cell_label = |global_idx: usize| -> String {
        // Fallback when running without tables (bench mode): use static
        // placeholders so the keyboard widget still looks right.
        if label_tables.is_none() {
            return match global_idx {
                0..=3 => ["a", "s", "d", "f"][global_idx].to_string(),
                6..=9 => ["j", "k", "l", ";"][global_idx - 6].to_string(),
                _ => String::new(),
            };
        }
        lookup_label(CELL_SCANS[global_idx])
    };
    // Widths match the old display: 4 left-box cells, L inner, R inner,
    // 4 right-box cells — indexed via left_labels[0..=4] / right_labels[0..=4].
    let left_labels: [String; 5] = [
        cell_label(0),
        cell_label(1),
        cell_label(2),
        cell_label(3),
        cell_label(4),
    ];
    let right_labels: [String; 5] = [
        cell_label(5),
        cell_label(6),
        cell_label(7),
        cell_label(8),
        cell_label(9),
    ];

    // Cell → target bit mapping. Cells 0-3 are L-pinky/ring/mid/idx
    // (bit 3..=0); cell 4 is L-inner-index (bit 4). Right-side cells 0-4
    // skip the thumb slot: cell 0 = R-inner-index (bit 5), cells 1-4 =
    // R-idx/mid/ring/pinky (bits 0..=3).
    let left_bit = |i: usize| -> Option<u8> {
        match i {
            0 => Some(3),
            1 => Some(2),
            2 => Some(1),
            3 => Some(0),
            4 => Some(4),
            _ => None,
        }
    };
    let right_bit = |i: usize| -> Option<u8> {
        match i {
            0 => Some(5),
            1 => Some(0),
            2 => Some(1),
            3 => Some(2),
            4 => Some(3),
            _ => None,
        }
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
    //
    // Three visual tiers:
    //   primary target (first_down of an ordered brief, or sole target
    //     of an unordered one) → black-on-key_color, bold
    //   secondary target (other target keys when ordering applies) →
    //     black-on-dot_color, bold — dimmer so the primary pops
    //   non-target → dark gray text, no background
    let make_cell_style = |i: usize, target: bool, primary: bool| -> Style {
        if target {
            let bg = if primary {
                key_colors[i]
            } else {
                dot_colors[i]
            };
            Style::default()
                .fg(Color::Rgb(0, 0, 0))
                .bg(bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(0x40, 0x40, 0x40))
        }
    };
    // `primary` check: bright vs dim for target cells. Every cell is
    // primary (a) when the target has no ordering constraint, or
    // (b) once the user has pressed their first key — at that point
    // the lead is locked in, there's exactly one ordered-brief word
    // still in play, and every remaining target finger is simply
    // "the next key to press". While no key is pressed yet, only
    // cells in `accepted_leads` are bright (they're the acceptable
    // first-downs).
    let is_primary = |cell_idx: usize| -> bool {
        if target_accepted_leads.is_empty() {
            return true;
        }
        if user_first_down.is_some() {
            return true;
        }
        target_accepted_leads.test(CELL_SCANS[cell_idx])
    };

    let mut row: Vec<Span> = vec![Span::raw("  ")];
    for i in 0..4 {
        let target = left_bit(i)
            .map(|b| target_left & (1 << b) != 0)
            .unwrap_or(false);
        row.push(Span::styled("│", border_style));
        row.push(Span::styled(
            format!("{:^1$}", left_labels[i], CELL),
            make_cell_style(i, target, is_primary(i)),
        ));
    }
    row.push(Span::styled("│", border_style));
    // Left inner-index (no border) — target/held comes from bit 4.
    row.push(Span::raw("  "));
    let inner_l_target = target_left & (1 << 4) != 0;
    row.push(Span::styled(
        format!("{:^1$}", left_labels[4], CELL),
        make_cell_style(4, inner_l_target, is_primary(4)),
    ));
    row.push(Span::raw("  "));
    // Right inner-index (no border) — target/held comes from bit 5.
    let inner_r_target = target_right & (1 << 5) != 0;
    row.push(Span::styled(
        format!("{:^1$}", right_labels[0], CELL),
        make_cell_style(5, inner_r_target, is_primary(5)),
    ));
    row.push(Span::raw("  "));
    // Right bordered box
    row.push(Span::styled("│", border_style));
    for i in 1..5 {
        let target = right_bit(i)
            .map(|b| target_right & (1 << b) != 0)
            .unwrap_or(false);
        row.push(Span::styled(
            format!("{:^1$}", right_labels[i], CELL),
            make_cell_style(5 + i, target, is_primary(5 + i)),
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
        let dot = if held {
            format!("{:^1$}", "●", CELL)
        } else {
            " ".repeat(CELL)
        };
        Span::styled(dot, Style::default().fg(dot_colors[cell_idx]))
    };
    let mut row2: Vec<Span> = vec![Span::raw("   ")];
    for i in 0..4 {
        let held = left_bit(i)
            .map(|b| held_left & (1 << b) != 0)
            .unwrap_or(false);
        row2.push(dot_span(i, held));
        row2.push(Span::raw(" "));
    }
    // Inner-index dots — live held state mirrors the outer fingers.
    row2.push(Span::raw(" "));
    row2.push(dot_span(4, held_left & (1 << 4) != 0));
    row2.push(Span::raw("  "));
    row2.push(dot_span(5, held_right & (1 << 5) != 0));
    row2.push(Span::raw("  "));
    for i in 1..5 {
        let held = right_bit(i)
            .map(|b| held_right & (1 << b) != 0)
            .unwrap_or(false);
        row2.push(dot_span(5 + i, held));
        row2.push(Span::raw(" "));
    }
    lines.push(Line::from(row2));

    // Thumbs target — word = purple, mod = green, black bold label overlaid.
    // Mirrors the finger-cell brightness scheme: primary target gets the
    // full background colour, secondary (target but not first_down) gets
    // the dim "dot" colour, non-target stays gray.
    let word_active = Style::default()
        .fg(Color::Rgb(0, 0, 0))
        .bg(Color::Rgb(0x80, 0x00, 0xFF))
        .add_modifier(Modifier::BOLD);
    let word_secondary = Style::default()
        .fg(Color::Rgb(0, 0, 0))
        .bg(Color::Rgb(0x40, 0x00, 0x80))
        .add_modifier(Modifier::BOLD);
    let mod_active = Style::default()
        .fg(Color::Rgb(0, 0, 0))
        .bg(Color::Rgb(0, 255, 0))
        .add_modifier(Modifier::BOLD);
    let mod_secondary = Style::default()
        .fg(Color::Rgb(0, 0, 0))
        .bg(Color::Rgb(0, 0x7F, 0))
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Color::Rgb(0x40, 0x40, 0x40));
    // `word` in rhe's chord model isn't a scancode target_first_down can
    // point to, so it only has active/inactive — no primary/secondary.
    let word_t = if target_word { word_active } else { dim_style };
    let thumb_held = held_right & (1 << 4) != 0;
    // A "mod-tap only" target has just the thumb bit set — that's the
    // number-mode entry / decimal step, which advances on key-UP, so
    // the cell should go dark once the thumb is held ("got it, release
    // to fire"). A chord target that includes mod alongside finger
    // bits is different: the chord matches on full-chord key-down, so
    // mod must stay bright until the user finishes pressing the other
    // keys. Without this split, finger-first-then-mod chords told the
    // user "release mod" mid-gesture — which would break the chord.
    let is_mod_tap_only_target = target_right == (1 << 4) && target_left == 0;
    let mod_t = if is_mod_tap_only_target {
        if thumb_held { dim_style } else { mod_active }
    } else if target_right & (1 << 4) != 0 {
        if target_accepted_leads.is_empty()
            || user_first_down.is_some()
            || target_accepted_leads.test(scan::R_THUMB)
        {
            mod_active
        } else {
            mod_secondary
        }
    } else {
        dim_style
    };
    let _ = word_secondary; // reserved for future word-first targets

    // New layout total ≈ 108 cols. Word under left idx area, mod under right idx area.
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(38)),
        Span::styled(" word ", word_t),
        Span::raw(" ".repeat(22)),
        Span::styled("  ^  ", mod_t),
    ]));

    // Thumbs held at half brightness, matching the dot style.
    let word_dot = Style::default().fg(Color::Rgb(0x40, 0x00, 0x80));
    let mod_dot = Style::default().fg(Color::Rgb(0, 0x7F, 0));
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(38)),
        Span::styled(if held_word { "  ●   " } else { "      " }, word_dot),
        Span::raw(" ".repeat(22)),
        Span::styled(
            if held_right & (1 << 4) != 0 {
                "  ●  "
            } else {
                "     "
            },
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
    #[cfg(target_os = "linux")]
    let input = GrabInput::start_grab(
        grab_enabled,
        crate::input::evdev_backend::QuitTrigger::EscOrCapsPlusEsc,
        None,
    )
    .expect("failed to start key capture");
    #[cfg(target_os = "macos")]
    let input = GrabInput::start_grab(grab_enabled, true).expect("failed to start key capture");

    let mut key_state = KeyState::default();

    use std::collections::HashMap;
    use std::time::{Instant, SystemTime};

    let mut right_chords: Vec<u8> = (1..32u8).collect(); // 1-31 (5 bits)
    let mut left_chords: Vec<u8> = (1..16u8).collect(); // 1-15 (4 bits)

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
        .draw(|f| {
            draw_bench(
                f,
                &test_seq,
                current,
                total,
                &all_results,
                &key_state,
                &phase,
                round,
            )
        })
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
                .draw(|f| {
                    draw_bench(
                        f,
                        &test_seq,
                        current,
                        total,
                        &all_results,
                        &key_state,
                        &phase,
                        round,
                    )
                })
                .ok();
            continue;
        }

        let (hand, target_bits) = test_seq[current];

        // Accumulate bits on key-down for target hand
        if phase == BenchPhase::Timing && rhe_event.direction == KeyDirection::Down {
            if hand == 'R' && scan::right_bit(rhe_event.scan).is_some() {
                accum |= key_state.right_bits();
            } else if hand == 'L' && scan::left_bit(rhe_event.scan).is_some() {
                accum |= key_state.left_bits();
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
                if rhe_event.direction == KeyDirection::Down && rhe_event.scan != scan::WORD {
                    if hand == 'R' && scan::right_bit(rhe_event.scan).is_some() {
                        accum |= key_state.right_bits();
                    } else if hand == 'L' && scan::left_bit(rhe_event.scan).is_some() {
                        accum |= key_state.left_bits();
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
            .draw(|f| {
                draw_bench(
                    f,
                    &test_seq,
                    current,
                    total,
                    &all_results,
                    &key_state,
                    &phase,
                    round,
                )
            })
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

        println!(
            "\nChord benchmark — {} rounds, {} total measurements",
            round - 1,
            all_results.len()
        );
        println!("{:<6} {:<8} {:<8} {:<6}", "Hand", "Chord", "Avg ms", "N");
        println!("{}", "─".repeat(32));

        println!("\n--- Right hand (fastest → slowest) ---");
        let mut right_avg: Vec<_> = averages.iter().filter(|(h, _, _, _)| *h == 'R').collect();
        right_avg.sort_by_key(|&&(_, _, avg, _)| avg);
        for &&(_, bits, avg, count) in &right_avg {
            println!(
                "  {:05b}  {:<8}  {}ms  (n={})",
                bits,
                chord_label_right(bits),
                avg,
                count
            );
        }

        println!("\n--- Left hand (fastest → slowest) ---");
        let mut left_avg: Vec<_> = averages.iter().filter(|(h, _, _, _)| *h == 'L').collect();
        left_avg.sort_by_key(|&&(_, _, avg, _)| avg);
        for &&(_, bits, avg, count) in &left_avg {
            println!(
                "  {:04b}  {:<8}  {}ms  (n={})",
                bits,
                chord_label_left(bits),
                avg,
                count
            );
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
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0))),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // title
            Constraint::Length(3),  // progress (same slot as sentence)
            Constraint::Length(4),  // target detail (same slot as word detail)
            Constraint::Length(11), // keyboard (same as tutor)
            Constraint::Min(0),     // recent results
        ])
        .split(area);

    // Title
    frame.render_widget(
        Paragraph::new(" rhe bench  [Esc to quit]")
            .style(Style::default().fg(Color::Rgb(0, 255, 255))),
        chunks[0],
    );

    // Progress bar area (where sentence normally goes)
    let measured = results.len();
    let progress_text = if current >= total {
        format!(
            " Round {} complete! {} total measurements. Release all for next round, Esc to exit.",
            round - 1,
            measured
        )
    } else {
        format!(
            " Round {} — chord {}/{}  ({} total)",
            round,
            current + 1,
            total,
            measured
        )
    };
    frame.render_widget(
        Paragraph::new(progress_text)
            .style(Style::default().fg(Color::Rgb(160, 160, 160)))
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
                    Style::default().fg(Color::Rgb(255, 255, 255)).bold(),
                ),
            ]),
            Line::from(vec![Span::styled(
                format!(" {}", phase_str),
                Style::default().fg(Color::Rgb(160, 160, 160)),
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
                    Style::default().fg(Color::Rgb(255, 255, 255)).bold(),
                )),
                Line::from(Span::styled(
                    " Results printed on exit",
                    Style::default().fg(Color::Rgb(160, 160, 160)),
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
            KeyMask::EMPTY,
            key_state.right_bits(),
            key_state.left_bits(),
            key_state.word,
            None,
            None,
            false,
        );
    } else {
        draw_keyboard(
            frame,
            chunks[3],
            0,
            0,
            false,
            KeyMask::EMPTY,
            0,
            0,
            false,
            None,
            None,
            false,
        );
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
