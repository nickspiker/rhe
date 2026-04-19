use std::collections::HashMap;
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::chord_map::ChordKey;
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent as RheKeyEvent, PhysicalKey, Thumb};
use crate::input::rdev_backend::RdevInput;
use crate::state_machine::{Event as SmEvent, StateMachine};
use crate::table_gen;
use crate::word_lookup::{WordChords, WordLookup};

/// Mode name using rhe terminology.
fn mode_name(mode: u8, ctrl: bool, single_hand_right: bool, single_hand_left: bool) -> &'static str {
    if single_hand_right {
        if ctrl { "⌘right" } else { "right" }
    } else if single_hand_left {
        if ctrl { "⌘left" } else { "left" }
    } else {
        match (mode, ctrl) {
            (0, false) => "zil",
            (1, false) => "ter",
            (2, false) => "stel",
            (3, false) => "lun",
            (0, true) => "zila",
            (1, true) => "tera",
            (2, true) => "stela",
            (3, true) => "luna",
            _ => "?",
        }
    }
}

/// A single chord step (one press-release of fingers).
struct ChordStep {
    ipa: String,
    chord_key: u16,
    right_fingers: u8,
    left_fingers: u8,
    mode: u8,
    ctrl: bool,
}


/// A word to practice — either a single brief or multi-syllable with space.
struct PracticeWord {
    word: String,
    full_ipa: String,
    /// true = hold space, chord syllables, release space
    needs_space: bool,
    /// The chord(s) to press, in order.
    steps: Vec<ChordStep>,
}

#[derive(Default, Clone)]
struct KeyState {
    left: [bool; 4],
    right: [bool; 4],
    ctrl: bool,
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

    fn any_left(&self) -> bool {
        self.left.iter().any(|&k| k)
    }

    fn any_right(&self) -> bool {
        self.right.iter().any(|&k| k)
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

    fn current_step(&self) -> Option<&ChordStep> {
        let word = self.current_word()?;
        word.steps.get(self.step_idx)
    }

    fn advance_step(&mut self) {
        let word = &self.sentences[self.sentence_idx][self.word_idx];
        self.step_idx += 1;
        if self.step_idx >= word.steps.len() {
            self.next_word();
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
}

pub fn run_tutor() {
    let cmudict = std::fs::read_to_string("data/cmudict.dict").unwrap();
    let freq = std::fs::read_to_string("data/en_freq.txt").unwrap();
    let syllable_table = table_gen::generate(&cmudict, &freq);
    let brief_map = crate::briefs::generate_briefs(&cmudict, &freq, &syllable_table);

    let lookup = WordLookup::new(&brief_map, &syllable_table, &cmudict);

    let practice = build_practice(&lookup);

    let grab_enabled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let input = RdevInput::start_grab(grab_enabled).expect("failed to start key capture");

    terminal::enable_raw_mode().unwrap();
    io::stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();

    let mut sm = StateMachine::new();
    let mut key_state = KeyState::default();
    let mut practice = practice;
    let mut correct = 0usize;
    let mut attempts = 0usize;
    let mut last_result: Option<bool> = None;
    loop {
        terminal.draw(|f| {
            draw(f, &practice, &key_state, correct, attempts, last_result);
        }).ok();

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.code == KeyCode::Esc && key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }

        while let Ok(rhe_event) = input.rx.try_recv() {
            update_key_state(&mut key_state, &rhe_event);

            for sm_event in sm.feed(rhe_event) {
                if let SmEvent::Chord(chord) = sm_event {
                    attempts += 1;
                    let key = ChordKey::from_chord(&chord);

                    if let Some(step) = practice.current_step() {
                        if key.0 == step.chord_key {
                            correct += 1;
                            last_result = Some(true);
                            practice.advance_step();
                        } else {
                            last_result = Some(false);
                        }
                    }
                }
            }
        }
    }

    terminal::disable_raw_mode().ok();
    io::stdout().execute(LeaveAlternateScreen).ok();

    let pct = if attempts > 0 { correct as f64 / attempts as f64 * 100.0 } else { 0.0 };
    println!("Session: {}/{} ({:.0}%)", correct, attempts, pct);
}


fn build_practice(lookup: &WordLookup) -> Practice {
    // Build text dynamically from words we actually have chords for.
    // Use the lookup to filter — only include words that resolve to Brief or MultiSyllable.
    let common_text = [
        "i think we should go now but he said no",
        "what do you want me to do with all of this",
        "she was not like that at all and i know it",
        "can you tell me what time it is right now",
        "we have to get out of here as soon as we can",
        "do you know where he went last night",
        "just tell me the truth and i will help you",
        "i did not want to go but she made me",
        "they all want to know what we think about it",
        "he got up and went out the door",
        "i was just about to call you when you came in",
        "we should not have let them go like that",
        "she was the one who told him to do it",
        "you have to be here for this one",
        "do not tell me what to do",
        "i know what you want but i said no",
        "he can come with us if he wants to",
        "just get it and go",
        "what if we go up there and look",
        "she did not like what he said to her",
        "they want us to come in now and sit down",
        "i have no idea what that is",
        "you know i like you right",
        "we can do this if we want to",
        "he was here just a bit ago",
        "she got him to go with her to the store",
        "they should be here by now",
        "do you want me to come with you or not",
        "all i know is that it was not me",
        "we have to go now or we will be late",
        "i can not do this on my own you know",
        "what do you think about all of this stuff",
        "if you want to go then just go",
        "she told him not to do that but he did",
        "i was about to go but then he came in",
        "you and me we can do this",
        "tell me what you know about him and her",
        "i think that is the best thing to do",
        "we need to find out where they went",
        "he did not say a word to me about it",
    ];

    let mut sentences: Vec<Vec<PracticeWord>> = Vec::new();

    for line in &common_text {
        let word_chords = lookup.parse_text(line);

        for group in word_chords.chunks(8) {
            let mut sentence: Vec<PracticeWord> = Vec::new();

            for wc in group {
                match wc {
                    WordChords::Brief { word, chord_key, ipa } => {
                        sentence.push(PracticeWord {
                            word: word.clone(),
                            full_ipa: ipa.clone(),
                            needs_space: false,
                            steps: vec![ChordStep {
                                ipa: ipa.clone(),
                                chord_key: *chord_key,
                                right_fingers: (chord_key & 0xF) as u8,
                                left_fingers: ((chord_key >> 4) & 0xF) as u8,
                                mode: ((chord_key >> 8) & 0x3) as u8,
                                ctrl: (chord_key >> 10) & 1 == 1,
                            }],
                        });
                    }
                    WordChords::MultiSyllable { word, syllables } => {
                        let full_ipa = syllables.iter()
                            .map(|s| s.ipa.as_str())
                            .collect::<Vec<_>>()
                            .join("·");
                        let steps = syllables.iter().map(|s| ChordStep {
                            ipa: s.ipa.clone(),
                            chord_key: s.chord_key,
                            right_fingers: s.right_fingers,
                            left_fingers: s.left_fingers,
                            mode: s.mode,
                            ctrl: s.ctrl,
                        }).collect();
                        sentence.push(PracticeWord {
                            word: word.clone(),
                            full_ipa,
                            needs_space: true,
                            steps,
                        });
                    }
                    WordChords::Unknown(_) => {}
                }
            }

            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }
    }

    Practice {
        sentences,
        sentence_idx: 0,
        word_idx: 0,
        step_idx: 0,
    }
}

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
        PhysicalKey::Thumb(Thumb::Ctrl) => state.ctrl = pressed,
        PhysicalKey::Thumb(Thumb::Space) => state.space = pressed,
    }
}

// ─── Drawing ───────────────────────────────────────────────────

fn draw(
    frame: &mut Frame,
    practice: &Practice,
    keys: &KeyState,
    correct: usize,
    attempts: usize,
    last_result: Option<bool>,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // title
            Constraint::Length(3),  // sentence
            Constraint::Length(4),  // word detail
            Constraint::Length(11), // keyboard + preview
            Constraint::Length(2),  // result
            Constraint::Min(0),    // stats
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

    // Word detail
    if let Some(pw) = practice.current_word() {
        let step_info = if let Some(s) = practice.current_step() {
            let name = mode_name(
                s.mode, s.ctrl,
                s.right_fingers > 0 && s.left_fingers == 0,
                s.left_fingers > 0 && s.right_fingers == 0,
            );
            if pw.needs_space {
                format!("{}  syllable {}/{}", name, practice.step_idx + 1, pw.steps.len())
            } else {
                name.to_string()
            }
        } else {
            String::new()
        };

        let mut word_spans = vec![
            Span::styled(" ", Style::default()),
            Span::styled(&pw.word, Style::default().fg(Color::White).bold()),
        ];

        frame.render_widget(
            Paragraph::new(vec![Line::from(word_spans)])
                .block(Block::default().borders(Borders::ALL).title(" Target ")),
            chunks[2],
        );

        if let Some(step) = practice.current_step() {
            // Peek at next step for grey preview
            let next = if practice.step_idx + 1 < pw.steps.len() {
                Some(&pw.steps[practice.step_idx + 1])
            } else {
                // Next word's first step
                let next_word_idx = if practice.word_idx + 1 < practice.sentences[practice.sentence_idx].len() {
                    Some(practice.word_idx + 1)
                } else { None };
                next_word_idx.and_then(|wi| practice.sentences[practice.sentence_idx][wi].steps.first())
            };
            draw_keyboard(frame, chunks[3], step, pw.needs_space, next);
        }
    }

    // Result
    let result = match last_result {
        Some(true) => Span::styled(" Correct!", Style::default().fg(Color::Green)),
        Some(false) => Span::styled(" Try again", Style::default().fg(Color::Red)),
        None => Span::styled(" ", Style::default()),
    };
    frame.render_widget(Paragraph::new(Line::from(result)), chunks[4]);

    // Stats
    let pct = if attempts > 0 { correct as f64 / attempts as f64 * 100.0 } else { 0.0 };
    frame.render_widget(
        Paragraph::new(format!(
            " Sentence {}/{}  |  Accuracy: {}/{} ({:.0}%)",
            practice.sentence_idx + 1,
            practice.sentences.len(),
            correct, attempts, pct,
        )).style(Style::default().fg(Color::DarkGray)),
        chunks[5],
    );
}

fn draw_keyboard(frame: &mut Frame, area: Rect, word: &ChordStep,
                  needs_space: bool, next_step: Option<&ChordStep>) {
    // Key labels: IPA consonant for each finger position
    let left_ipa = ["n", "j", "w", "t"];   // pinky, ring, middle, index (bits 3,2,1,0)
    let right_ipa = ["t", "w", "j", "n"];  // index, middle, ring, pinky (bits 0,1,2,3)

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from("  ┌─────┬─────┬─────┬─────┐  ┌─────┬─────┬─────┬─────┐"));

    // Key row
    let mut row: Vec<Span> = vec![Span::raw("  ")];

    // Which hand leads? mode 0,1 = right first; mode 2,3 = left first
    let right_leads = word.mode < 2;

    for i in 0..4 {
        let bit = 3 - i;
        let target = word.left_fingers & (1 << bit) != 0;
        let style = target_style(target, word.mode, !right_leads);
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", left_ipa[i]), style));
    }
    row.push(Span::raw("│  "));

    for i in 0..4 {
        let bit = i;
        let target = word.right_fingers & (1 << bit) != 0;
        let style = target_style(target, word.mode, right_leads);
        row.push(Span::raw("│"));
        row.push(Span::styled(format!("  {}  ", right_ipa[i]), style));
    }
    row.push(Span::raw("│"));
    lines.push(Line::from(row));

    lines.push(Line::from("  └─────┴─────┴─────┴─────┘  └─────┴─────┴─────┴─────┘"));

    // Thumbs — static, just show what's needed
    let cmd_style = if word.ctrl {
        Style::default().fg(Color::Black).bg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let space_style = if needs_space {
        Style::default().fg(Color::Black).bg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    lines.push(Line::from(vec![
        Span::raw("         "),
        Span::styled("  ⌘  ", cmd_style),
        Span::raw("  "),
        Span::styled("[space]", space_style),
    ]));

    // Next chord preview in dark grey (if available)
    if let Some(next) = next_step {
        lines.push(Line::from("  ┌─────┬─────┬─────┬─────┐  ┌─────┬─────┬─────┬─────┐"));
        let mut next_row: Vec<Span> = vec![Span::raw("  ")];
        for i in 0..4 {
            let bit = 3 - i;
            let target = next.left_fingers & (1 << bit) != 0;
            let style = if target {
                Style::default().fg(Color::Rgb(60, 60, 60))
            } else {
                Style::default().fg(Color::Rgb(30, 30, 30))
            };
            next_row.push(Span::raw("│"));
            next_row.push(Span::styled(format!("  {}  ", left_ipa[i]), style));
        }
        next_row.push(Span::raw("│  "));
        for i in 0..4 {
            let bit = i;
            let target = next.right_fingers & (1 << bit) != 0;
            let style = if target {
                Style::default().fg(Color::Rgb(60, 60, 60))
            } else {
                Style::default().fg(Color::Rgb(30, 30, 30))
            };
            next_row.push(Span::raw("│"));
            next_row.push(Span::styled(format!("  {}  ", right_ipa[i]), style));
        }
        next_row.push(Span::raw("│"));
        lines.push(Line::from(next_row));
        lines.push(Line::from("  └─────┴─────┴─────┴─────┘  └─────┴─────┴─────┴─────┘"));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Static target display. No live key feedback.
/// Bright = lead hand (press first). Dim = follow hand.
fn target_style(target: bool, mode: u8, lead: bool) -> Style {
    if !target {
        return Style::default().fg(Color::DarkGray);
    }
    let color = match mode {
        0 => if lead { Color::Red } else { Color::Rgb(100, 30, 30) },
        1 => if lead { Color::Yellow } else { Color::Rgb(100, 100, 30) },
        2 => if lead { Color::Green } else { Color::Rgb(30, 100, 30) },
        _ => if lead { Color::Blue } else { Color::Rgb(30, 30, 100) },
    };
    Style::default().fg(Color::Black).bg(color)
}
