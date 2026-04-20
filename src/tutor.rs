use std::io;

use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::chord_map::{BriefTable, Phoneme, PhonemeTable};
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent as RheKeyEvent, PhysicalKey, Thumb};
use crate::input::iohid_backend::{IoHidInput, HidEvent};
use crate::interpreter::Interpreter;
use crate::output::macos::MacOSOutput;
use crate::output::TextOutput;
use crate::state_machine::StateMachine;
use crate::table_gen::PhonemeDictionary;
use crate::word_lookup::WordLookup;

// ─── Target: a 10-bit snapshot of what keys should be pressed ───

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Target {
    right: u8,
    left: u8,
    modkey: bool,
    space: bool,
}

impl Target {
    /// Does the live key state have any key pressed that's NOT in this target?
    fn has_extra(&self, state: &KeyState) -> bool {
        let extra_right = state.right_bits() & !self.right;
        let extra_left = state.left_bits() & !self.left;
        let extra_mod = state.modkey && !self.modkey;
        let extra_space = state.space && !self.space;
        extra_right != 0 || extra_left != 0 || extra_mod || extra_space
    }

    /// Does the live key state exactly match this target?
    fn matches(&self, state: &KeyState) -> bool {
        state.right_bits() == self.right
            && state.left_bits() == self.left
            && state.modkey == self.modkey
            && state.space == self.space
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
    steps: Vec<Step>,
}

#[derive(Default, Clone)]
struct KeyState {
    left: [bool; 4],
    right: [bool; 4],
    modkey: bool,
    space: bool,
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
    }
}

struct Practice {
    sentences: Vec<Vec<PracticeWord>>,
    sentence_idx: usize,
    word_idx: usize,
    step_idx: usize,
}

impl Practice {
    fn current_word(&self) -> Option<&PracticeWord> {
        self.sentences.get(self.sentence_idx)?.get(self.word_idx)
    }

    fn current_step(&self) -> Option<&Step> {
        self.current_word()?.steps.get(self.step_idx)
    }

    fn current_target(&self) -> Option<&Target> {
        Some(&self.current_step()?.target)
    }

    fn advance_step(&mut self) {
        self.step_idx += 1;
        if let Some(word) = self.current_word() {
            if self.step_idx >= word.steps.len() {
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
    }

    fn reset_word(&mut self) {
        self.step_idx = 0;
    }
}

// ─── Main loop ───

pub fn run_tutor() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict").unwrap();
    let lookup = WordLookup::new(&cmudict);
    let practice = build_practice(&lookup);

    terminal::enable_raw_mode().unwrap();
    io::stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

    let grab_enabled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let input = IoHidInput::start_grab(grab_enabled).expect("failed to start key capture");

    // Real output pipeline: same events → state machine → interpreter → text injection
    let mut sm = StateMachine::new();
    let freq_data = std::fs::read_to_string("data/word_freq.txt").unwrap_or_default();
    let dict = PhonemeDictionary::build(&cmudict, &freq_data);
    let mut interp = Interpreter::new(PhonemeTable::new(), BriefTable::new(), dict);
    let output = MacOSOutput::new();

    let mut log: Vec<String> = Vec::new();
    let mut key_state = KeyState::default();
    let mut practice = practice;
    let mut errored = false; // true = all dark, wait for all keys off

    // Initial draw
    terminal.draw(|f| draw(f, &practice, false)).ok();

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
                }
            }
        }

        let all_off = key_state.right_bits() == 0 && key_state.left_bits() == 0
            && !key_state.modkey && !key_state.space;

        // Log every key change
        log.push(format!("state R:{:04b} L:{:04b} sp={} mod={} | step={} err={}",
            key_state.right_bits(), key_state.left_bits(), key_state.space, key_state.modkey,
            practice.step_idx, errored));

        if errored {
            // Wait for all keys off, then clear
            if all_off {
                log.push("  → ERROR CLEAR".to_string());
                errored = false;
            }
        } else if let Some(target) = practice.current_target() {
            let target = *target;
            let is_key_down = rhe_event.direction == KeyDirection::Down;
            let step = practice.current_step().unwrap();

            if step.space_only {
                if !key_state.space {
                    log.push("  → MATCH (commit)".to_string());
                    practice.advance_step();
                }
            } else {
                let space_dropped = rhe_event.key == PhysicalKey::Thumb(Thumb::Space)
                    && rhe_event.direction == KeyDirection::Up
                    && target.space
                    && practice.step_idx > 0;

                if space_dropped {
                    log.push("  → RESET (space released mid-word)".to_string());
                    practice.reset_word();
                    if !all_off { errored = true; }
                } else if is_key_down && target.has_extra(&key_state) {
                    log.push("  → RESET (extra key down)".to_string());
                    practice.reset_word();
                    if !all_off { errored = true; }
                } else if target.matches(&key_state) {
                    log.push("  → MATCH".to_string());
                    practice.advance_step();
                }
            }
        }

        // Draw after every event
        terminal.draw(|f| draw(f, &practice, errored)).ok();
    }

    terminal::disable_raw_mode().ok();
    io::stdout().execute(LeaveAlternateScreen).ok();

    let log_path = "tutor_debug.log";
    std::fs::write(log_path, log.join("\n")).ok();
    println!("Debug log written to {}", log_path);
}

// ─── Build practice steps ───

fn build_practice(lookup: &WordLookup) -> Practice {
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
                let clean: String = word_str.chars()
                    .filter(|c| c.is_alphabetic() || *c == '\'')
                    .collect();

                let Some(phonemes) = lookup.lookup(&clean) else {
                    skipped.push(clean);
                    continue;
                };

                let mut steps: Vec<Step> = Vec::new();

                for (i, &phoneme) in phonemes.iter().enumerate() {
                    let key = phoneme.chord_key();
                    let r = key.right_bits();
                    let l = key.left_bits();
                    let m = key.has_mod();

                    if i == 0 {
                        // First phoneme: space + chord
                        steps.push(Step {
                            target: Target { right: r, left: l, modkey: m, space: true },
                            phoneme: Some(phoneme),
                            space_only: false,
                        });
                    } else {
                        let prev_key = phonemes[i - 1].chord_key();
                        // Down: both hands + space
                        steps.push(Step {
                            target: Target {
                                right: prev_key.right_bits() | r,
                                left: prev_key.left_bits() | l,
                                modkey: prev_key.has_mod() || m,
                                space: true,
                            },
                            phoneme: Some(phoneme),
                            space_only: false,
                        });
                        // Up: release prev hand, keep new chord + space
                        steps.push(Step {
                            target: Target { right: r, left: l, modkey: m, space: true },
                            phoneme: None,
                            space_only: false,
                        });
                    }
                }

                // Commit: space up (fingers don't matter)
                steps.push(Step {
                    target: Target { space: false, ..Target::default() },
                    phoneme: None,
                    space_only: true,
                });

                sentence.push(PracticeWord { word: clean, steps });
            }

            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }

        if !skipped.is_empty() {
            eprintln!("  SKIP unknown: {}", skipped.join(", "));
        }
    }

    Practice {
        sentences,
        sentence_idx: 0,
        word_idx: 0,
        step_idx: 0,
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
        PhysicalKey::Thumb(Thumb::Mod) => state.modkey = pressed,
        PhysicalKey::Thumb(Thumb::Space) => state.space = pressed,
    }
}

// ─── Drawing ───

fn draw(frame: &mut Frame, practice: &Practice, errored: bool) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // title
            Constraint::Length(3),  // sentence
            Constraint::Length(4),  // word detail
            Constraint::Length(7),  // keyboard
            Constraint::Min(0),    // padding
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
        let phoneme_label = practice.current_step()
            .and_then(|s| s.phoneme)
            .map(|p| format!(" {}", p.to_ipa()))
            .unwrap_or_default();

        let lines = vec![
            Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(&pw.word, Style::default().fg(Color::White).bold()),
            ]),
            Line::from(vec![
                Span::styled(phoneme_label, Style::default().fg(Color::Gray)),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );

        // Keyboard: all dark when errored, otherwise show target
        if errored {
            draw_keyboard(frame, chunks[3], 0, 0, false, false);
        } else if let Some(step) = practice.current_step() {
            draw_keyboard(frame, chunks[3],
                step.target.right, step.target.left,
                step.target.modkey, step.target.space);
        }
    }
}

fn draw_keyboard(
    frame: &mut Frame,
    area: Rect,
    right_fingers: u8,
    left_fingers: u8,
    modkey: bool,
    space: bool,
) {
    let left_labels = ["P", "R", "M", "I"];
    let right_labels = ["I", "M", "R", "P"];

    let target_style = Style::default().fg(Color::Black).bg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from("  ┌─────┬─────┬─────┬─────┐  ┌─────┬─────┬─────┬─────┐"));

    let mut row: Vec<Span> = vec![Span::raw("  ")];

    for i in 0..4 {
        let bit = 3 - i;
        let target = left_fingers & (1 << bit) != 0;
        let style = if target { target_style } else { dim_style };
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", left_labels[i]), style));
    }
    row.push(Span::raw("│  "));

    for i in 0..4 {
        let target = right_fingers & (1 << i) != 0;
        let style = if target { target_style } else { dim_style };
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", right_labels[i]), style));
    }
    row.push(Span::raw("│"));
    lines.push(Line::from(row));

    lines.push(Line::from("  └─────┴─────┴─────┴─────┘  └─────┴─────┴─────┴─────┘"));

    let mod_style = if modkey { target_style } else { dim_style };
    let space_style = if space { target_style } else { dim_style };

    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled("  ⌘  ", mod_style),
        Span::raw("  "),
        Span::styled("[space]", space_style),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}
