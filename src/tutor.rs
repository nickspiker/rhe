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
#[cfg(not(target_os = "macos"))]
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordMode {
    Brief,
    Phoneme,
}

#[derive(Default, Clone)]
struct KeyState {
    left: [bool; 4],
    right: [bool; 5], // 4 fingers + thumb (spacebar)
    word: bool,       // left ⌘
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
            WordMode::Brief => word.brief_steps.as_deref().or(Some(&word.phoneme_steps)),
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

    fn default_mode(&self) -> WordMode {
        if self.current_word().map_or(false, |w| w.brief_steps.is_some()) {
            WordMode::Brief
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
    let mut hand_touched = false; // target hand had bits at some point this step

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
        let mode_str = match practice.mode { WordMode::Brief => "brief", WordMode::Phoneme => "phon" };
        log.push(format!(
            "state R:{:05b} L:{:04b} word={} | \"{}\" [{}] step={} err={}",
            key_state.right_bits(),
            key_state.left_bits(),
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
                hand_touched = false;
            }
            // Word key up at step 0 (no progress) → back to default mode
            else if rhe_event.key == PhysicalKey::Word
                && rhe_event.direction == KeyDirection::Up
                && practice.mode == WordMode::Phoneme
                && practice.step_idx == 0
            {
                practice.mode = practice.default_mode();
                practice.step_idx = 0;
                hand_touched = false;
            }

            if let Some(target) = practice.current_target() {
                let target = *target;
                let step = practice.current_step().unwrap();

                if step.space_only {
                    // Commit step: any finger down = error, word up = advance
                    if is_key_down && rhe_event.key != PhysicalKey::Word {
                        log.push("  → RESET (finger during commit)".to_string());
                        practice.reset_word();
                        hand_touched = false;
                        errored = true;
                    } else if !key_state.word {
                        log.push("  → MATCH (commit)".to_string());
                        practice.advance_step();
                        hand_touched = false;
                    }
                } else {
                    // Track if target hand was pressed (only on key-down for that hand)
                    if is_key_down {
                        let pressed_target_hand = match rhe_event.key {
                            PhysicalKey::Finger(Hand::Right, _) => target.right != 0,
                            PhysicalKey::Finger(Hand::Left, _) => target.left != 0,
                            _ => false,
                        };
                        if pressed_target_hand {
                            hand_touched = true;
                        }
                    }

                    // Detect abandoned chord: hand was touched, now back to zero
                    let target_hand_zero = if target.right != 0 {
                        key_state.right_bits() == 0
                    } else {
                        key_state.left_bits() == 0
                    };
                    let hand_abandoned = hand_touched && target_hand_zero && !is_key_down;

                    let space_dropped = rhe_event.key == PhysicalKey::Word
                        && rhe_event.direction == KeyDirection::Up
                        && target.word
                        && practice.step_idx > 0;

                    if space_dropped {
                        log.push("  → RESET (space released mid-word)".to_string());
                        practice.reset_word();
                        hand_touched = false;
                        if !all_off {
                            errored = true;
                        }
                    } else if hand_abandoned {
                        log.push("  → RESET (chord abandoned)".to_string());
                        practice.reset_word();
                        hand_touched = false;
                        if !all_off {
                            errored = true;
                        }
                    } else if is_key_down && target.has_extra(&key_state) {
                        log.push("  → RESET (extra key down)".to_string());
                        practice.reset_word();
                        hand_touched = false;
                        if !all_off {
                            errored = true;
                        }
                    } else if target.matches(&key_state) {
                        log.push("  → MATCH".to_string());
                        practice.advance_step();
                        hand_touched = false;
                    }
                }
            }
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

                sentence.push(PracticeWord {
                    word: clean,
                    phoneme_steps,
                    brief_steps,
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

    // Word detail + phoneme hint
    if let Some(pw) = practice.current_word() {
        let phoneme_label = practice
            .current_step()
            .and_then(|s| s.phoneme)
            .map(|p| format!(" {}", p.to_ipa()))
            .unwrap_or_default();

        let lines = vec![
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(&pw.word, Style::default().fg(Color::White).bold()),
            ]),
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
    let left_labels = ["P", "R", "M", "I"];
    let right_labels = ["I", "M", "R", "P"];

    let target_style = Style::default().fg(Color::Black).bg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);
    let held_style = Style::default().fg(Color::Rgb(8, 8, 8));

    let mut lines: Vec<Line> = Vec::new();

    // Target row (with borders)
    lines.push(Line::from(
        "  ┌─────┬─────┬─────┬─────┐  ┌─────┬─────┬─────┬─────┐",
    ));

    let mut row: Vec<Span> = vec![Span::raw("  ")];
    for i in 0..4 {
        let bit = 3 - i;
        let target = target_left & (1 << bit) != 0;
        let style = if target { target_style } else { dim_style };
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", left_labels[i]), style));
    }
    row.push(Span::raw("│  "));
    for i in 0..4 {
        let target = target_right & (1 << i) != 0;
        let style = if target { target_style } else { dim_style };
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", right_labels[i]), style));
    }
    row.push(Span::raw("│"));
    lines.push(Line::from(row));

    lines.push(Line::from(
        "  └─────┴─────┴─────┴─────┘  └─────┴─────┴─────┴─────┘",
    ));

    // Held row (no borders, subtle)
    let mut row2: Vec<Span> = vec![Span::raw("   ")];
    for i in 0..4 {
        let bit = 3 - i;
        let held = held_left & (1 << bit) != 0;
        row2.push(Span::styled(
            if held { "  ●   " } else { "      " },
            held_style,
        ));
    }
    row2.push(Span::raw("   "));
    for i in 0..4 {
        let held = held_right & (1 << i) != 0;
        row2.push(Span::styled(
            if held { "  ●   " } else { "      " },
            held_style,
        ));
    }
    lines.push(Line::from(row2));

    // Thumbs target (with labels)
    let word_t = if target_word { target_style } else { dim_style };
    let mod_t = if target_right & (1 << 4) != 0 {
        target_style
    } else {
        dim_style
    };
    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled("[word]", word_t),
        Span::raw("                    "),
        Span::styled("[mod]", mod_t),
    ]));

    // Thumbs held (subtle, no borders)
    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled(
            if held_word { "  ●   " } else { "      " },
            held_style,
        ),
        Span::raw("                    "),
        Span::styled(
            if held_right & (1 << 4) != 0 { "  ●  " } else { "     " },
            held_style,
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}
