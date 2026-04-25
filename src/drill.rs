//! Renderer-agnostic drill machinery for the tutor.
//!
//! Owns the data types (`Target`, `Step`, `KeyState`, `Practice`,
//! `TutorState`), the dictionary-driven builders that turn a sentence
//! into a chord-step sequence, and the state machine that drives a
//! drill forward from raw key events. Knows nothing about ratatui or
//! winit — both renderers (the legacy terminal tutor and the new GUI
//! tutor window) call into the same `TutorState`.
//!
//! Lifted out of `tutor.rs` during Phase C so the GUI window in
//! `tray.rs` can plug into the drill without dragging in ratatui.

use crate::chord_map::{BriefTable, ChordKey, Phoneme, PhonemeTable};
use crate::hand::{KeyDirection, KeyEvent as RheKeyEvent};
use crate::key_mask::KeyMask;
use crate::scan;
use crate::word_lookup::WordLookup;

// ─── Target: what keys should be pressed ───
// right = 5 bits (4 fingers + thumb/spacebar as bit 4)
// left  = 4 bits (4 fingers)
// word  = left ⌘ held

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Target {
    pub right: u8,
    pub left: u8,
    pub word: bool,
    /// Set of scancodes any of which is an acceptable lead finger for
    /// this ordered brief. Empty mask = no ordering constraint.
    pub accepted_leads: KeyMask,
}

impl Target {
    pub fn has_extra(&self, state: &KeyState) -> bool {
        let extra_word = state.word && !self.word;
        if self.right != 0 && self.left != 0 {
            let extra_right = state.right_bits() & !self.right;
            let extra_left = state.left_bits() & !self.left;
            extra_right != 0 || extra_left != 0 || extra_word
        } else if self.right != 0 {
            (state.right_bits() & !self.right) != 0 || extra_word
        } else if self.left != 0 {
            (state.left_bits() & !self.left) != 0 || extra_word
        } else {
            state.right_bits() != 0 || state.left_bits() != 0 || extra_word
        }
    }

    pub fn matches(&self, state: &KeyState) -> bool {
        let word_ok = state.word == self.word;
        if self.right != 0 && self.left != 0 {
            state.right_bits() == self.right && state.left_bits() == self.left && word_ok
        } else if self.right != 0 {
            state.right_bits() == self.right && word_ok
        } else if self.left != 0 {
            state.left_bits() == self.left && word_ok
        } else {
            state.right_bits() == 0 && state.left_bits() == 0 && word_ok
        }
    }
}

// ─── Steps for a word ───

#[derive(Default, Clone)]
pub struct Step {
    pub target: Target,
    pub phoneme: Option<Phoneme>,
    /// Commit step — matches on word release (phoneme mode) or on
    /// all-off (brief mode). Any finger press during this step
    /// triggers the "finger during commit" reset (except for bounces
    /// of keys already in the prior chord).
    pub space_only: bool,
    /// Match on `Event::ModTap` instead of a chord. Used for number-
    /// mode entry (first tap of the sequence) and for the decimal
    /// point within a number sequence.
    pub mod_tap_only: bool,
    /// Hint text for the tutor's word-detail line — the one character
    /// this number-mode step emits ('3', '.', '+', etc.). None for
    /// non-number steps.
    pub number_glyph: Option<String>,
}

pub struct PracticeWord {
    pub word: String,
    pub phoneme_steps: Vec<Step>,        // word held + phoneme sequence + commit
    pub brief_steps: Option<Vec<Step>>,  // single chord without word + all-off
    pub suffix_steps: Option<Vec<Step>>, // roll(base) + suffix chord + all-off
    pub suffix_label: Option<String>,    // e.g. "~ing" for display
    pub number_steps: Option<Vec<Step>>, // mod-tap entry + per-char + commit
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WordMode {
    Brief,
    Phoneme,
    Suffix,
    Number,
}

#[derive(Default, Clone)]
pub struct KeyState {
    /// [pinky, ring, middle, index, inner-index]. Only inner-index is
    /// reachable in number mode; zero in every other path.
    pub left: [bool; 5],
    /// [index, middle, ring, pinky, thumb, inner-index].
    pub right: [bool; 6],
    pub word: bool, // left ⌘
}

impl KeyState {
    pub fn left_bits(&self) -> u8 {
        (self.left[0] as u8) << 3
            | (self.left[1] as u8) << 2
            | (self.left[2] as u8) << 1
            | self.left[3] as u8
            | (self.left[4] as u8) << 4
    }

    pub fn right_bits(&self) -> u8 {
        self.right[0] as u8
            | (self.right[1] as u8) << 1
            | (self.right[2] as u8) << 2
            | (self.right[3] as u8) << 3
            | (self.right[4] as u8) << 4
            | (self.right[5] as u8) << 5
    }
}

pub struct Practice {
    pub sentences: Vec<Vec<PracticeWord>>,
    pub sentence_idx: usize,
    pub word_idx: usize,
    pub step_idx: usize,
    pub mode: WordMode,
    /// Set the instant we wrap past the last sentence; the live tutor
    /// loop watches this to swap in a freshly-prefetched Wikipedia
    /// article.
    pub wrapped: bool,
}

impl Practice {
    pub fn current_word(&self) -> Option<&PracticeWord> {
        self.sentences.get(self.sentence_idx)?.get(self.word_idx)
    }

    pub fn current_steps(&self) -> Option<&[Step]> {
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

    pub fn current_step(&self) -> Option<&Step> {
        self.current_steps()?.get(self.step_idx)
    }

    pub fn current_target(&self) -> Option<&Target> {
        Some(&self.current_step()?.target)
    }

    pub fn advance_step(&mut self) {
        self.step_idx += 1;
        if let Some(steps) = self.current_steps() {
            if self.step_idx >= steps.len() {
                self.next_word();
            }
        }
    }

    pub fn next_word(&mut self) {
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

    pub fn reset_word(&mut self) {
        self.step_idx = 0;
        self.mode = self.default_mode();
    }

    pub fn prev_word(&mut self) {
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

    pub fn default_mode(&self) -> WordMode {
        if let Some(w) = self.current_word() {
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

// ─── Build practice steps ───

/// Curated drill lines used by `rhe test`. Reproducible, offline,
/// short enough to cycle thru while iterating on chord designs.
pub const TEST_SENTENCES: &[&str] = &[
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
pub const ALICE_FALLBACK: &[&str] = &[
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

/// Map a single number-mode character ('0'..='9' and the symbols on
/// the same chord positions) to its right/left finger bits and a
/// flag indicating whether the symbol requires the mod (right
/// thumb) chord variant.
pub fn number_char_target(c: char) -> Option<(u8, u8, bool)> {
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
    let (right, left) = match pos {
        0 => (1u8 << 3, 0u8),
        1 => (1 << 2, 0),
        2 => (1 << 1, 0),
        3 => (1 << 0, 0),
        4 => (1 << 5, 0),
        5 => (0, 1u8 << 4),
        6 => (0, 1 << 0),
        7 => (0, 1 << 1),
        8 => (0, 1 << 2),
        9 => (0, 1 << 3),
        _ => unreachable!(),
    };
    Some((right, left, is_symbol))
}

/// Build number-mode steps for spelled digit words ("zero" through
/// "nine"). Generates: mod-tap entry → finger+mod chord → commit.
pub fn build_digit_word_steps(word: &str) -> Option<Vec<Step>> {
    let lower = word.to_lowercase();
    let scan_code = match lower.as_str() {
        "zero" => scan::R_PINKY,
        "one" => scan::R_RING,
        "two" => scan::R_MID,
        "three" => scan::R_IDX,
        "four" => scan::R_IDX_INNER,
        "five" => scan::L_IDX_INNER,
        "six" => scan::L_IDX,
        "seven" => scan::L_MID,
        "eight" => scan::L_RING,
        "nine" => scan::L_PINKY,
        _ => return None,
    };

    let (right, left) = if let Some(bit) = scan::right_bit(scan_code) {
        (1u8 << bit | (1 << 4), 0u8)
    } else if let Some(bit) = scan::left_bit(scan_code) {
        (1u8 << 4, 1u8 << bit)
    } else {
        return None;
    };

    let mut leads = KeyMask::EMPTY;
    leads.set(scan_code);

    let mut steps = Vec::new();

    // Mod-tap entry
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

    // Finger+mod chord
    steps.push(Step {
        target: Target {
            right,
            left,
            word: true,
            accepted_leads: leads,
        },
        number_glyph: Some(word.to_lowercase()),
        ..Step::default()
    });

    // Release
    steps.push(Step {
        target: Target {
            right: 0,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    });

    // Commit
    steps.push(Step {
        target: Target::default(),
        space_only: true,
        ..Step::default()
    });

    Some(steps)
}

/// Build the per-step drill sequence for a number/symbol "word".
/// Structure: mod-tap entry + one step per character + commit.
pub fn build_number_steps(word: &str) -> Option<Vec<Step>> {
    if !word.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    for c in word.chars() {
        if c != '.' && number_char_target(c).is_none() {
            return None;
        }
    }

    let mut steps: Vec<Step> = Vec::new();
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

    let mut prev_needs_release = false;
    for c in word.chars() {
        if c == '.' {
            if prev_needs_release {
                steps.push(release_step());
            }
            steps.push(Step {
                target: mod_tap_target,
                mod_tap_only: true,
                number_glyph: Some(".".to_string()),
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
            number_glyph: Some(c.to_string()),
            ..Step::default()
        });
        prev_needs_release = true;
    }

    steps.push(Step {
        target: Target::default(),
        space_only: true,
        ..Step::default()
    });

    Some(steps)
}

/// Compile a list of drill text into a `Practice`. Splits each line
/// into 8-word chunks (sentences in the practice sense) and builds
/// phoneme/brief/suffix/number step paths per word.
pub fn build_practice(
    lookup: &WordLookup,
    brief_table: &BriefTable,
    lines: Vec<String>,
    deterministic_start: bool,
) -> Practice {
    let mut sentences: Vec<Vec<PracticeWord>> = Vec::new();
    let mut line_starts: Vec<usize> = Vec::new();

    for line in &lines {
        line_starts.push(sentences.len());
        let words: Vec<&str> = line.split_whitespace().collect();

        for group in words.chunks(8) {
            let mut sentence: Vec<PracticeWord> = Vec::new();

            for &word_str in group {
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
                    continue;
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
                                ..Step::default()
                            },
                            Step {
                                target: Target::default(),
                                ..Step::default()
                            },
                        ]
                    })
                };

                let (suffix_steps, suffix_label) = {
                    use crate::suffixes_data::SUFFIXES;
                    let phoneme_count = phoneme_steps.iter().filter(|s| s.phoneme.is_some()).count();
                    let mut found = (None, None);

                    if phoneme_count > 2 {
                        let mut sorted_suffixes: Vec<(u8, &str)> = SUFFIXES.to_vec();
                        sorted_suffixes.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                        for &(suffix_bits, suffix_str) in &sorted_suffixes {
                            if !clean.ends_with(suffix_str) {
                                continue;
                            }
                            if clean.len() <= suffix_str.len() {
                                continue;
                            }

                            let bases = vec![
                                clean[..clean.len() - suffix_str.len()].to_string(),
                                format!("{}e", &clean[..clean.len() - suffix_str.len()]),
                            ];

                            for base in &bases {
                                let mut base_chord: Option<(u8, u8)> = None;
                                let mut base_leads = KeyMask::EMPTY;
                                for (key, first_down, brief_word) in brief_table.iter() {
                                    if brief_word.trim() != base {
                                        continue;
                                    }
                                    let r = key.right_bits()
                                        | if key.has_mod() { 1u8 << 4 } else { 0 };
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
                                            ..Step::default()
                                        },
                                        Step {
                                            target: Target::default(),
                                            ..Step::default()
                                        },
                                        Step {
                                            target: Target {
                                                right: 0,
                                                left: suffix_bits,
                                                word: false,
                                                accepted_leads: KeyMask::EMPTY,
                                            },
                                            ..Step::default()
                                        },
                                        Step {
                                            target: Target::default(),
                                            ..Step::default()
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
    }

    let valid_starts: Vec<usize> = line_starts
        .into_iter()
        .filter(|&idx| idx < sentences.len())
        .collect();
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

// ─── Drill state machine ───

/// Renderer-agnostic drill driver. Wraps the loop body that used to
/// live inline in `run_tutor`: feed it raw key events with `tick()`,
/// then read `practice.current_word()` / `current_step()` / `key_state`
/// to render whatever frontend you like.
///
/// Owns a private `StateMachine` so it can spot `ModTap` events and
/// advance number-mode entry steps. This is independent of the engine
/// thread's interpreter — the drill matches what the user *should* be
/// chording, while the engine still types the *actual* text into the
/// focused app.
pub struct TutorState {
    pub practice: Practice,
    pub key_state: KeyState,
    pub errored: bool,
    pub last_was_botch: bool,
    pub fingers_during_word: bool,
    pub tutor_first_down: Option<u8>,
    chord_right_acc: u8,
    chord_left_acc: u8,
    touched_right: u8,
    touched_left: u8,
    sm: crate::state_machine::StateMachine,
}

impl TutorState {
    pub fn new(practice: Practice) -> Self {
        Self {
            practice,
            key_state: KeyState::default(),
            errored: false,
            last_was_botch: false,
            fingers_during_word: false,
            tutor_first_down: None,
            chord_right_acc: 0,
            chord_left_acc: 0,
            touched_right: 0,
            touched_left: 0,
            sm: crate::state_machine::StateMachine::new(),
        }
    }

    /// Advance the drill on a single raw key event.
    pub fn tick(&mut self, rhe_event: RheKeyEvent) {
        update_key_state(&mut self.key_state, &rhe_event);

        let prev_step_idx = self.practice.step_idx;
        let prev_mode = self.practice.mode;

        if rhe_event.direction == KeyDirection::Down {
            if let Some(bit) = scan::right_bit(rhe_event.scan) {
                self.chord_right_acc |= 1u8 << bit;
            } else if let Some(bit) = scan::left_bit(rhe_event.scan) {
                self.chord_left_acc |= 1u8 << bit;
            }
        }

        let mut step_advanced_by_modtap = false;
        for sm_event in self.sm.feed(rhe_event) {
            if matches!(sm_event, crate::state_machine::Event::ModTap) {
                let is_mod_tap_step = self
                    .practice
                    .current_step()
                    .map_or(false, |s| s.mod_tap_only);
                if is_mod_tap_step {
                    self.practice.advance_step();
                    step_advanced_by_modtap = true;
                }
            }
        }

        let all_off = self.key_state.right_bits() == 0
            && self.key_state.left_bits() == 0
            && !self.key_state.word;

        if self.errored {
            if all_off {
                self.errored = false;
                self.touched_right = 0;
                self.touched_left = 0;
                self.tutor_first_down = None;
            }
        } else {
            let is_key_down = rhe_event.direction == KeyDirection::Down;

            if rhe_event.scan == scan::WORD && rhe_event.direction == KeyDirection::Down {
                let is_number_word = self.practice.current_word().map_or(false, |w| {
                    w.number_steps.is_some() && w.phoneme_steps.is_empty()
                });
                self.practice.mode = if is_number_word {
                    WordMode::Number
                } else {
                    WordMode::Phoneme
                };
                self.practice.step_idx = 0;
                self.fingers_during_word = false;
            } else if rhe_event.scan == scan::WORD
                && rhe_event.direction == KeyDirection::Up
                && matches!(self.practice.mode, WordMode::Phoneme | WordMode::Number)
                && self.practice.step_idx == 0
            {
                if !self.fingers_during_word {
                    if self.last_was_botch {
                        self.last_was_botch = false;
                    } else {
                        self.practice.prev_word();
                    }
                }
                self.practice.mode = self.practice.default_mode();
                self.practice.step_idx = 0;
            }

            if self.key_state.word && is_key_down && rhe_event.scan != scan::WORD {
                self.fingers_during_word = true;
            }

            if step_advanced_by_modtap {
                // already advanced above; fall through to step-transition reseed
            } else if let Some(target) = self.practice.current_target() {
                let target = *target;
                let step = self.practice.current_step().unwrap();
                let mod_tap_only = step.mod_tap_only;
                let space_only = step.space_only;

                let prev_target: Option<Target> = if self.practice.step_idx > 0 {
                    self.practice
                        .current_steps()
                        .and_then(|steps| steps.get(self.practice.step_idx - 1))
                        .map(|s| s.target)
                } else {
                    None
                };
                let bounce_of_prev = |scan_code: u8| -> bool {
                    let Some(prev) = prev_target else {
                        return false;
                    };
                    if let Some(bit) = scan::left_bit(scan_code) {
                        prev.left & (1 << bit) != 0
                    } else if let Some(bit) = scan::right_bit(scan_code) {
                        prev.right & (1 << bit) != 0
                    } else if scan_code == scan::WORD {
                        prev.word
                    } else {
                        false
                    }
                };

                if mod_tap_only {
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && rhe_event.scan != scan::R_THUMB
                    {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        self.errored = true;
                    }
                } else if space_only {
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && !bounce_of_prev(rhe_event.scan)
                    {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        self.errored = true;
                    } else if !self.key_state.word {
                        self.last_was_botch = false;
                        self.practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && !target.word {
                    if is_key_down && !bounce_of_prev(rhe_event.scan) {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        self.errored = true;
                    } else if all_off {
                        self.last_was_botch = false;
                        self.practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && target.word {
                    if is_key_down && rhe_event.scan != scan::WORD {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        self.errored = true;
                    } else if self.key_state.right_bits() == 0
                        && self.key_state.left_bits() == 0
                    {
                        self.practice.advance_step();
                    }
                } else {
                    if is_key_down {
                        if self.tutor_first_down.is_none()
                            && (scan::right_bit(rhe_event.scan).is_some()
                                || scan::left_bit(rhe_event.scan).is_some())
                        {
                            self.tutor_first_down = Some(rhe_event.scan);
                        }
                        if let Some(bit) = scan::right_bit(rhe_event.scan) {
                            self.touched_right |= 1u8 << bit;
                        } else if let Some(bit) = scan::left_bit(rhe_event.scan) {
                            self.touched_left |= 1u8 << bit;
                        }
                    }

                    let hand_touched = (target.right != 0 && self.touched_right != 0)
                        || (target.left != 0 && self.touched_left != 0);

                    let target_hands_empty = (target.right == 0
                        || self.key_state.right_bits() == 0)
                        && (target.left == 0 || self.key_state.left_bits() == 0);

                    let acc_matches = (target.right == 0
                        || self.chord_right_acc == target.right)
                        && (target.left == 0 || self.chord_left_acc == target.left)
                        && self.key_state.word == target.word;

                    let has_extra_acc = (target.right != 0
                        && (self.chord_right_acc & !target.right) != 0)
                        || (target.left != 0 && (self.chord_left_acc & !target.left) != 0)
                        || (target.right == 0 && self.touched_right != 0)
                        || (target.left == 0 && self.touched_left != 0)
                        || (self.key_state.word && !target.word);

                    let space_dropped = rhe_event.scan == scan::WORD
                        && rhe_event.direction == KeyDirection::Up
                        && target.word
                        && self.practice.step_idx > 0;

                    if space_dropped {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        if !all_off {
                            self.errored = true;
                        }
                    } else if is_key_down && has_extra_acc {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        if !all_off {
                            self.errored = true;
                        }
                    } else if acc_matches && hand_touched {
                        let first_down_ok = target.accepted_leads.is_empty()
                            || self
                                .tutor_first_down
                                .map(|fd| target.accepted_leads.test(fd))
                                .unwrap_or(false);
                        if !first_down_ok {
                            self.practice.reset_word();
                            self.last_was_botch = true;
                            if !all_off {
                                self.errored = true;
                            }
                        } else {
                            self.practice.advance_step();
                        }
                    } else if hand_touched && target_hands_empty && !is_key_down {
                        self.practice.reset_word();
                        self.last_was_botch = true;
                        if !all_off {
                            self.errored = true;
                        }
                    }
                }
            }
        }

        // Step-transition reseed
        if self.practice.step_idx != prev_step_idx || self.practice.mode != prev_mode {
            if let Some(new_target) = self.practice.current_target() {
                self.chord_right_acc = self.key_state.right_bits() & new_target.right;
                self.chord_left_acc = self.key_state.left_bits() & new_target.left;
            } else {
                self.chord_right_acc = 0;
                self.chord_left_acc = 0;
            }
            self.touched_right = 0;
            self.touched_left = 0;
            self.tutor_first_down = None;
        }

        // Hand-zero accumulator reset (after step handler so abandon
        // detection still saw partial-attempt bits).
        if self.key_state.right_bits() == 0 {
            self.chord_right_acc = 0;
        }
        if self.key_state.left_bits() == 0 {
            self.chord_left_acc = 0;
        }
        if self.key_state.right_bits() == 0 && self.key_state.left_bits() == 0 {
            self.tutor_first_down = None;
        }
    }
}

// ─── Adaptive cell labels ───

/// Build a `KeyMask` from a `KeyState` for adaptive-label lookups.
/// Mirrors the bit ordering used by the rest of the drill machinery.
pub fn key_state_to_mask(state: &KeyState) -> KeyMask {
    let mut m = KeyMask::EMPTY;
    const L_SCANS: [u8; 4] = [scan::L_IDX, scan::L_MID, scan::L_RING, scan::L_PINKY];
    const R_SCANS: [u8; 4] = [scan::R_IDX, scan::R_MID, scan::R_RING, scan::R_PINKY];
    for (bit, s) in L_SCANS.iter().enumerate() {
        if state.left_bits() & (1 << bit) != 0 {
            m.set(*s);
        }
    }
    for (bit, s) in R_SCANS.iter().enumerate() {
        if state.right_bits() & (1 << bit) != 0 {
            m.set(*s);
        }
    }
    if state.right_bits() & (1 << 4) != 0 {
        m.set(scan::R_THUMB);
    }
    if state.left[4] {
        m.set(scan::L_IDX_INNER);
    }
    if state.right[5] {
        m.set(scan::R_IDX_INNER);
    }
    m
}

/// Short label for a number-form transform chord. Stub set picked
/// to fit a 9-char cell — each abbreviates the form's output:
/// `spell` ("five"), `tuple` ("quintuple"), `pre` ("penta"),
/// `ord` ("fifth"), `frac` ("half"/"third"), `mul` ("once"/"twice").
/// Refine once the labels are visible alongside real numbers.
pub fn form_label(form: crate::number_forms::Form) -> &'static str {
    use crate::number_forms::Form;
    match form {
        Form::SpelledCardinal => "spell",
        Form::Group => "tuple",
        Form::Prefix => "pre",
        Form::Ordinal => "ord",
        Form::Fraction => "frac",
        Form::Multiplier => "mul",
    }
}

/// Predict what `cell_scan` would emit if added to the currently-held
/// chord, for adaptive on-cell labels.
///
/// - `held_word` selects between phoneme mode (word held) and brief
///   mode (word released). In phoneme mode each hand fires
///   independently, so the candidate chord only includes the cell's
///   own hand bits.
/// - `user_first_down` lets ordered briefs resolve to the right word
///   when the user is mid-roll. When nothing is held the cell itself
///   becomes the hypothetical lead.
/// - `in_number_mode` swaps the lookup to digit/symbol tables.
/// - `has_number_context` shifts brief-mode L-hand cells to number-
///   form labels (the same chords' alternate meaning when a pure-
///   integer is sitting one slot back, ready to be transformed).
pub fn cell_label(
    cell_scan: u8,
    held_mask: KeyMask,
    held_word: bool,
    user_first_down: Option<u8>,
    phonemes: &PhonemeTable,
    briefs: &BriefTable,
    in_number_mode: bool,
    has_number_context: bool,
) -> String {
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
    // must only see left-hand held bits. Brief mode is a single
    // combined chord, so all held bits contribute.
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

    let lookup_first = if base.is_empty() {
        Some(cell_scan)
    } else {
        user_first_down
    };

    // Brief-mode L-hand chord with armed number context: this chord
    // would transform the just-emitted integer, not append a suffix.
    // Show the form abbreviation instead of the brief lookup.
    if !held_word && has_number_context {
        if let Some(form) = crate::interpreter::chord_to_form(chord) {
            return form_label(form).to_string();
        }
    }

    if held_word {
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
    }
}

// ─── Key state update ───

pub fn update_key_state(state: &mut KeyState, event: &RheKeyEvent) {
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
