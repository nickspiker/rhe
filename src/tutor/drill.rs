//! Renderer-agnostic drill machinery for the tutor.
//!
//! Owns the data types (`Target`, `Step`, `KeyState`, `Practice`, `TutorState`), the dictionary-driven builders that turn a sentence into a chord-step sequence, and the state machine that drives a drill forward from raw key events. Knows nothing about ratatui or winit — both renderers (the legacy terminal tutor and the new GUI tutor window) call into the same `TutorState`.
//!
//! Lifted out of `tutor.rs` during Phase C so the GUI window in `tray.rs` can plug into the drill without dragging in ratatui.

use crate::hand::{KeyDirection, KeyEvent as RheKeyEvent};
use crate::key_mask::KeyMask;
use crate::layout::chords::{BriefTable, ChordKey, Phoneme, PhonemeTable};
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
    /// Set of scancodes any of which is an acceptable lead finger for this ordered brief. Empty mask = no ordering constraint.
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
    /// Commit step — matches on word release (phoneme mode) or on all-off (brief mode). Any finger press during this step triggers the "finger during commit" reset (except for bounces of keys already in the prior chord).
    pub space_only: bool,
    /// Match on `Event::ModTap` instead of a chord. Used for number- mode entry (first tap of the sequence) and for the decimal point within a number sequence.
    pub mod_tap_only: bool,
    /// Hint text for the tutor's word-detail line — the one character this number-mode step emits ('3', '.', '+', etc.). None for non-number steps.
    pub number_glyph: Option<String>,
}

pub struct PracticeWord {
    pub word: String,
    pub phoneme_steps: Vec<Step>, // word held + phoneme sequence + commit
    pub brief_steps: Option<Vec<Step>>, // single chord without word + all-off
    pub suffix_steps: Option<Vec<Step>>, // roll(base) + suffix chord + all-off
    pub suffix_label: Option<String>, // e.g. "~ing" for display
    pub number_steps: Option<Vec<Step>>, // mod-tap entry + per-char + commit
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordMode {
    Brief,
    Phoneme,
    Suffix,
    Number,
}

#[derive(Default, Clone)]
pub struct KeyState {
    /// [pinky, ring, middle, index, inner-index]. Only inner-index is reachable in number mode; zero in every other path.
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
    /// Set the instant we wrap past the last sentence; the live tutor loop watches this to swap in a freshly-prefetched Wikipedia article.
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

/// Curated drill lines used by the tray menu's Test Text source. Reproducible, offline, short enough to cycle thru while iterating on chord designs. One line per recent feature comes first so a quick pass through Test Text exercises every new chord / gesture before the older homophone and number-mode regression sets.
pub const TEST_SENTENCES: &[&str] = &[
    // ─── Recent features ─────────────────────────────────────────────
    // Pristine zero gesture (word + mod-tap + word-up = "zero").
    "the count is zero and now we begin to type slowly",
    // one/won ordered bundle on R-RING+R-thumb.
    "i won the match and one of you must lose this round",
    // thing-family compound: thing / something / anything / nothing / everything.
    "the thing is something anything nothing everything has its own place here",
    // one-family compound: someone / anyone / everyone.
    "someone said anyone or everyone could be welcome here today right now",
    // Batch A homophones: ok/okay, seen/scene.
    "i have seen this scene and ok and okay both work fine here",
    // Batch A homophones: knows/nose.
    "she knows my nose well so we get along fine all day",
    // Batch B homophones: peace/piece.
    "for peace we share a piece of warm bread by the fire",
    // Batch B homophones: weather/whether.
    "i wonder whether the weather will hold today or turn cold tonight",
    // Batch B homophones: cell/sell, role/roll.
    "let us sell the cell and roll the new role at dawn",
    // Batch B homophones: led/lead.
    "he led the team and now i lead the next round here",
    // Batch B homophones: site/sight.
    "the site is a sight for sore eyes today and tomorrow too",

    // ─── Number-mode regression set ──────────────────────────────────
    "the answer is 42 four times ten plus two",
    "pi is about 3.14159 ish today",
    "count 0 1 2 3 4 5 6 7 8 9 and then stop",
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

/// Brown Corpus (40k sentences of American English): one sentence per line, zstd-compressed in `data/brown_corpus.txt.zst` and resolved through `crate::data` (cache → checkout → GitHub raw). Decompressed on each call; called once when the user selects the "Brown Corpus" tutor text source.
pub fn brown_corpus_lines() -> Vec<String> {
    let compressed = crate::data::load_brown_corpus_zstd();
    let decompressed = zstd::decode_all(compressed.as_slice()).unwrap_or_default();
    let text = String::from_utf8_lossy(&decompressed);
    text.lines().map(|l| l.to_string()).collect()
}

/// Map a single number-mode character ('0'..='9' and the symbols on the same chord positions) to its right/left finger bits and a flag indicating whether the symbol requires the mod (right thumb) chord variant.
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

/// Pristine empty-number-mode-exit gesture: word-hold → mod-tap → word-release. Engine emits "zero ". Both this and the older R-pinky-spelled path are accepted by the engine, but the pristine path is the recent shortcut and is what the tutor teaches for "zero".
pub fn build_pristine_zero_steps() -> Vec<Step> {
    let word_only = Target {
        right: 0,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };
    let all_off = Target::default();
    vec![
        // 1. +word
        Step {
            target: word_only,
            ..Step::default()
        },
        // 2. mod-tap (R-thumb press+release while word held)
        Step {
            target: word_only,
            mod_tap_only: true,
            number_glyph: Some("zero".to_string()),
            ..Step::default()
        },
        // 3. all-off (word release → engine emits "zero ")
        Step {
            target: all_off,
            ..Step::default()
        },
    ]
}

/// Build number-mode steps for spelled digit words ("zero" through "nine"). Generates: mod-tap entry → finger+mod chord → commit. Special cases: "zero" uses the pristine gesture (`build_pristine_zero_steps`); "one" returns `None` so practice falls through to the brief table, where the recent one/won ordered bundle (R-RING+R-thumb, R_THUMB-first → "one", R_RING-first → "won") owns the chord.
pub fn build_digit_word_steps(word: &str) -> Option<Vec<Step>> {
    let lower = word.to_lowercase();
    if lower == "zero" {
        return Some(build_pristine_zero_steps());
    }
    if lower == "one" {
        return None;
    }
    let scan_code = match lower.as_str() {
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

    // Finger-only bits (no mod) and finger+mod bits
    let (finger_right, finger_left, full_right, full_left) =
        if let Some(bit) = scan::right_bit(scan_code) {
            (1u8 << bit, 0u8, 1u8 << bit | (1 << 4), 0u8)
        } else if let Some(bit) = scan::left_bit(scan_code) {
            (0u8, 1u8 << bit, 1u8 << 4, 1u8 << bit)
        } else {
            return None;
        };

    let mut leads = KeyMask::EMPTY;
    leads.set(scan_code);

    let word_only = Target {
        right: 0,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };

    let mut steps = Vec::new();

    // Step 1: +mod +word
    steps.push(Step {
        target: Target {
            right: 1 << 4,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    });

    // Step 2: -mod (word still held)
    steps.push(Step {
        target: word_only,
        ..Step::default()
    });

    // Step 3: +number (finger before mod = spelled)
    steps.push(Step {
        target: Target {
            right: finger_right,
            left: finger_left,
            word: true,
            accepted_leads: leads,
        },
        number_glyph: Some(word.to_lowercase()),
        ..Step::default()
    });

    // Step 4: +mod (finger + mod + word)
    steps.push(Step {
        target: Target {
            right: full_right,
            left: full_left,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    });

    // Step 5: -mod -word -number (all off)
    steps.push(Step {
        target: Target::default(),
        ..Step::default()
    });

    Some(steps)
}

/// Reverse-lookup: given a word like "nineteen" / "twentieth" / "twice" / "half", return `(integer, form_left_bits)` such that applying that form to the integer produces the word. Built once, cached. Lower-priority forms inserted first so higher-priority (more specific) forms override on overlap.
fn lookup_form(word: &str) -> Option<(u64, u8)> {
    use crate::layout::number_forms::{self as f, Form};
    use std::collections::HashMap;
    use std::sync::OnceLock;

    static TABLE: OnceLock<HashMap<String, (u64, u8)>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut m: HashMap<String, (u64, u8)> = HashMap::new();
        // Cardinal first — broadest. Loop range covers everything
        // each form might produce; multi-token outputs ("five
        // hundred", "five times") will never match a single
        // whitespace-split practice word, so they sit unused but
        // harmless.
        for n in 0u64..=1000 {
            let s = n.to_string();
            if let Some(w) = f::spelled_cardinal(&s) {
                m.insert(w, (n, Form::SpelledCardinal.chord_bits()));
            }
        }
        for n in 0u64..=1000 {
            let s = n.to_string();
            if let Some(w) = f::ordinal(&s) {
                m.insert(w, (n, Form::Ordinal.chord_bits()));
            }
        }
        for n in 1u64..=99 {
            let s = n.to_string();
            if let Some(w) = f::multiplier(&s) {
                m.insert(w, (n, Form::Multiplier.chord_bits()));
            }
        }
        for n in 1u64..=10 {
            let s = n.to_string();
            if let Some(w) = f::group(&s) {
                m.insert(w, (n, Form::Group.chord_bits()));
            }
        }
        for n in 1u64..=10 {
            let s = n.to_string();
            if let Some(w) = f::prefix(&s) {
                m.insert(w, (n, Form::Prefix.chord_bits()));
            }
        }
        // Fraction limited to 2..=4 ("half", "third", "quarter") so
        // it doesn't override ordinal forms ("fifth", "sixth", ...)
        // for n>=5, where the same string is shared and the user
        // almost always means ordinal.
        for n in 2u64..=4 {
            let s = n.to_string();
            if let Some(w) = f::fraction(&s) {
                m.insert(w, (n, Form::Fraction.chord_bits()));
            }
        }
        m
    });
    table.get(word).copied()
}

/// Build drill steps for a spelled-form number word like "nineteen", "twentieth", "half", "twice", "tri", etc. Returns `None` if the word doesn't match any cardinal/ordinal/multiplier/group/fraction/ prefix form for n in 0..=1000.
///
/// Step sequence mirrors what the engine actually accepts:
/// 1. `+word+mod` — mod-tap entry
/// 2. `-mod` (word held) — confirm number-mode entry
/// 3. for each digit of the underlying integer: a. `+digit` (target finger, word held) b. `-digit` (back to word_only)
/// 4. all-off — release word; engine emits the integer + a space and arms `has_number_context`
/// 5. form chord (left-hand only, no word) — engine sees number context and replaces the integer with the spelled form
/// 6. all-off — release form fingers
pub fn build_spelled_form_steps(word: &str) -> Option<Vec<Step>> {
    let (n, form_left) = lookup_form(word)?;
    let digits: String = n.to_string();

    let word_only = Target {
        right: 0,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };
    let all_off = Target::default();

    let mut steps: Vec<Step> = Vec::new();

    // Step 0: +word+mod (mod-tap entry).
    steps.push(Step {
        target: Target {
            right: 1 << 4,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        ..Step::default()
    });

    // Step 1: -mod (word still held).
    steps.push(Step {
        target: word_only,
        ..Step::default()
    });

    // Per-digit steps: press, then release back to word_only.
    for c in digits.chars() {
        let (right, left, _) = number_char_target(c)?;
        steps.push(Step {
            target: Target {
                right,
                left,
                word: true,
                accepted_leads: KeyMask::EMPTY,
            },
            number_glyph: Some(c.to_string()),
            ..Step::default()
        });
        steps.push(Step {
            target: word_only,
            ..Step::default()
        });
    }

    // Release word: exits number mode, engine emits space + arms
    // number context for the form transform that's about to come.
    steps.push(Step {
        target: all_off,
        ..Step::default()
    });

    // Form chord — left-hand only, no word. Engine recognises this
    // via has_number_context and applies the form, replacing the
    // integer with the spelled form.
    steps.push(Step {
        target: Target {
            right: 0,
            left: form_left,
            word: false,
            accepted_leads: KeyMask::EMPTY,
        },
        number_glyph: Some(word.to_string()),
        ..Step::default()
    });

    // Release form chord.
    steps.push(Step {
        target: all_off,
        ..Step::default()
    });

    Some(steps)
}

/// Build the per-step drill sequence for a number/symbol "word". Structure: mod-tap entry + one step per character + commit.
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
    let word_only = Target {
        right: 0,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };

    // Step 1: +mod +word
    steps.push(Step {
        target: Target {
            right: 1 << 4,
            left: 0,
            word: true,
            accepted_leads: KeyMask::EMPTY,
        },
        number_glyph: None,
        ..Step::default()
    });
    // Step 2: -mod (word still held)
    steps.push(Step {
        target: word_only,
        ..Step::default()
    });

    for c in word.chars() {
        if c == '.' {
            // Decimal: re-tap mod
            steps.push(Step {
                target: Target {
                    right: 1 << 4,
                    left: 0,
                    word: true,
                    accepted_leads: KeyMask::EMPTY,
                },
                number_glyph: Some(".".to_string()),
                ..Step::default()
            });
            steps.push(Step {
                target: word_only,
                ..Step::default()
            });
            continue;
        }
        let (right, left, needs_mod) = number_char_target(c).unwrap();
        if needs_mod {
            // Operator: +mod then +finger (thumb first = symbol)
            steps.push(Step {
                target: Target {
                    right: 1 << 4,
                    left: 0,
                    word: true,
                    accepted_leads: KeyMask::EMPTY,
                },
                ..Step::default()
            });
            steps.push(Step {
                target: Target {
                    right: right | (1 << 4),
                    left,
                    word: true,
                    accepted_leads: KeyMask::EMPTY,
                },
                number_glyph: Some(c.to_string()),
                ..Step::default()
            });
        } else {
            // Digit: just +finger (no mod)
            steps.push(Step {
                target: Target {
                    right,
                    left,
                    word: true,
                    accepted_leads: KeyMask::EMPTY,
                },
                number_glyph: Some(c.to_string()),
                ..Step::default()
            });
        }
        // -finger (word still held)
        steps.push(Step {
            target: word_only,
            ..Step::default()
        });
    }

    // Replace last word_only with all-off (release word too)
    if let Some(last) = steps.last_mut() {
        last.target = Target::default();
    }

    Some(steps)
}

/// Compile a list of drill text into a `Practice`. Splits each line into 8-word chunks (sentences in the practice sense) and builds phoneme/brief/suffix/number step paths per word.
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

                if let Some(number_steps) = build_spelled_form_steps(word_str) {
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
                    use crate::layout::suffixes::SUFFIXES;
                    let phoneme_count =
                        phoneme_steps.iter().filter(|s| s.phoneme.is_some()).count();
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

/// Renderer-agnostic drill driver. Wraps the loop body that used to live inline in `run_tutor`: feed it raw key events with `tick()`, then read `practice.current_word()` / `current_step()` / `key_state` to render whatever frontend you like.
///
/// Owns a private `StateMachine` so it can spot `ModTap` events and advance number-mode entry steps. This is independent of the engine thread's interpreter — the drill matches what the user *should* be chording, while the engine still types the *actual* text into the focused app.
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
        let prev_word_idx = self.practice.word_idx;
        let prev_sentence_idx = self.practice.sentence_idx;
        let prev_errored = self.errored;

        crate::tlog!(
            "key: {} {} | state R:{:05b} L:{:05b} W:{} | word=\"{}\" mode={:?} step={}",
            scan::label(rhe_event.scan),
            match rhe_event.direction {
                KeyDirection::Down => "down",
                KeyDirection::Up => "up",
            },
            self.key_state.right_bits(),
            self.key_state.left_bits(),
            self.key_state.word as u8,
            self.practice
                .current_word()
                .map(|w| w.word.as_str())
                .unwrap_or(""),
            self.practice.mode,
            self.practice.step_idx,
        );

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
            } else if rhe_event.scan == scan::WORD
                && rhe_event.direction == KeyDirection::Up
                && self.practice.mode == WordMode::Number
                && self.practice.step_idx > 0
                && self.practice.current_target().map_or(false, |t| t.word)
            {
                // Number-mode botch: user released word while a
                // mid-sequence step still required word held. Without
                // this, the drill would silently stagnate and a later
                // word re-press would reset to step 0 with no error
                // ever shown — the user could "pass" on the retry.
                // Phoneme mode catches this further down via
                // `space_dropped`; number mode needs its own check
                // because its matcher is state-based, not acc-based.
                self.practice.reset_word();
                self.last_was_botch = true;
                self.errored = true;
            }

            if self.key_state.word && is_key_down && rhe_event.scan != scan::WORD {
                self.fingers_during_word = true;
            }

            if step_advanced_by_modtap {
                // already advanced above; fall through to step-transition reseed
            } else if self
                .practice
                .current_step()
                .map_or(false, |s| s.mod_tap_only)
            {
                // mod_tap_only step: tolerate R-thumb press/release without
                // overshoot errors. Advance fires above via the ModTap sm
                // event when R-thumb releases. Any other finger going down
                // is still a botch — the gesture is "thumb tap, no other
                // input." Hoisted above the Number-mode arm so spelled-zero
                // (lives in number_steps) doesn't trip the strict state
                // matcher on R-thumb down.
                if is_key_down
                    && rhe_event.scan != scan::WORD
                    && rhe_event.scan != scan::R_THUMB
                {
                    self.practice.reset_word();
                    self.last_was_botch = true;
                    self.errored = true;
                }
            } else if self.practice.mode == WordMode::Number {
                // Number mode: pure state matching. Advance when
                // current key state exactly equals the target.
                // Reset on any extra key that overshoots the target.
                if let Some(target) = self.practice.current_target() {
                    let target = *target;
                    let state_right = self.key_state.right_bits();
                    let state_left = self.key_state.left_bits();
                    let state_word = self.key_state.word;
                    if state_right == target.right
                        && state_left == target.left
                        && state_word == target.word
                    {
                        self.practice.advance_step();
                    } else if is_key_down {
                        let extra_right = state_right & !target.right;
                        let extra_left = state_left & !target.left;
                        let extra_word = state_word && !target.word;
                        if extra_right != 0 || extra_left != 0 || extra_word {
                            self.practice.reset_word();
                            self.last_was_botch = true;
                            self.errored = true;
                        }
                    } else {
                        // Key release: if the released key was a target
                        // bit and the chord still isn't complete, the
                        // user dropped it without finishing — botch.
                        // Catches the "press, release, re-press, +mod"
                        // workaround: at the post-finger step
                        // (target = finger+mod), releasing the finger
                        // before mod has joined is a regression.
                        let regress_right = scan::right_bit(rhe_event.scan)
                            .map(|bit| (target.right & (1u8 << bit)) != 0)
                            .unwrap_or(false);
                        let regress_left = scan::left_bit(rhe_event.scan)
                            .map(|bit| (target.left & (1u8 << bit)) != 0)
                            .unwrap_or(false);
                        let chord_incomplete = state_right != target.right
                            || state_left != target.left
                            || state_word != target.word;
                        if (regress_right || regress_left) && chord_incomplete {
                            self.practice.reset_word();
                            self.last_was_botch = true;
                            self.errored = true;
                        }
                    }
                }
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
                    } else if self.key_state.right_bits() == 0 && self.key_state.left_bits() == 0 {
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

                    let acc_matches = (target.right == 0 || self.chord_right_acc == target.right)
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

        if !prev_errored && self.errored {
            crate::tlog!(
                "  → BOTCH: reset to step 0 of \"{}\"",
                self.practice
                    .current_word()
                    .map(|w| w.word.as_str())
                    .unwrap_or("?")
            );
        }
        if self.practice.word_idx != prev_word_idx
            || self.practice.sentence_idx != prev_sentence_idx
        {
            crate::tlog!(
                "  → next word: \"{}\" mode={:?}",
                self.practice
                    .current_word()
                    .map(|w| w.word.as_str())
                    .unwrap_or("?"),
                self.practice.mode,
            );
        } else if self.practice.step_idx != prev_step_idx {
            crate::tlog!(
                "  → step {} → {} (mode={:?})",
                prev_step_idx,
                self.practice.step_idx,
                self.practice.mode,
            );
        }
    }
}

// ─── Adaptive cell labels ───

/// Build a `KeyMask` from a `KeyState` for adaptive-label lookups. Mirrors the bit ordering used by the rest of the drill machinery.
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

/// Short label for a number-form transform chord. Stub set picked to fit a 9-char cell — each abbreviates the form's output: `spell` ("five"), `tuple` ("quintuple"), `pre` ("penta"), `ord` ("fifth"), `frac` ("half"/"third"), `mul` ("once"/"twice"). Refine once the labels are visible alongside real numbers.
pub fn form_label(form: crate::layout::number_forms::Form) -> &'static str {
    use crate::layout::number_forms::Form;
    match form {
        Form::SpelledCardinal => "spell",
        Form::Group => "tuple",
        Form::Prefix => "pre",
        Form::Ordinal => "ord",
        Form::Fraction => "frac",
        Form::Multiplier => "mul",
    }
}

/// Predict what `cell_scan` would emit if added to the currently-held chord, for adaptive on-cell labels.
///
/// - `held_word` selects between phoneme mode (word held) and brief mode (word released). In phoneme mode each hand fires independently, so the candidate chord only includes the cell's own hand bits.
/// - `user_first_down` lets ordered briefs resolve to the right word when the user is mid-roll. When nothing is held the cell itself becomes the hypothetical lead.
/// - `in_number_mode` swaps the lookup to digit/symbol tables.
/// - `has_number_context` shifts brief-mode L-hand cells to number- form labels (the same chords' alternate meaning when a pure- integer is sitting one slot back, ready to be transformed).
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
            crate::layout::numbers::chord_to_symbol(chord)
        } else {
            crate::layout::numbers::chord_to_digit(chord)
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
        if let Some(form) = crate::layout::number_forms::Form::from_chord(chord) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::number_forms::Form;

    #[test]
    fn spelled_form_cardinals() {
        let bits = Form::SpelledCardinal.chord_bits();
        assert_eq!(lookup_form("ten"), Some((10, bits)));
        assert_eq!(lookup_form("nineteen"), Some((19, bits)));
        assert_eq!(lookup_form("twenty"), Some((20, bits)));
        assert_eq!(lookup_form("twenty-one"), Some((21, bits)));
        assert_eq!(lookup_form("ninety-nine"), Some((99, bits)));
    }

    #[test]
    fn spelled_form_ordinals() {
        let bits = Form::Ordinal.chord_bits();
        assert_eq!(lookup_form("first"), Some((1, bits)));
        assert_eq!(lookup_form("nineteenth"), Some((19, bits)));
        assert_eq!(lookup_form("twentieth"), Some((20, bits)));
        assert_eq!(lookup_form("thousandth"), Some((1000, bits)));
    }

    #[test]
    fn spelled_form_specifics_override_default() {
        // "twice" / "thrice" → multiplier, not cardinal.
        let mult = Form::Multiplier.chord_bits();
        assert_eq!(lookup_form("twice"), Some((2, mult)));
        assert_eq!(lookup_form("thrice"), Some((3, mult)));
        // "half" / "third" / "quarter" → fraction.
        let frac = Form::Fraction.chord_bits();
        assert_eq!(lookup_form("half"), Some((2, frac)));
        assert_eq!(lookup_form("third"), Some((3, frac)));
        assert_eq!(lookup_form("quarter"), Some((4, frac)));
        // Group / prefix words.
        let group = Form::Group.chord_bits();
        assert_eq!(lookup_form("pair"), Some((2, group)));
        assert_eq!(lookup_form("triple"), Some((3, group)));
        let prefix = Form::Prefix.chord_bits();
        assert_eq!(lookup_form("mono"), Some((1, prefix)));
        assert_eq!(lookup_form("tri"), Some((3, prefix)));
    }

    #[test]
    fn spelled_form_steps_for_nineteen() {
        let steps = build_spelled_form_steps("nineteen").expect("nineteen should map");
        // entry(1) + -mod(1) + per-digit press+release × 2 digits(4)
        //   + release-word(1) + form(1) + final all-off(1) = 9
        assert_eq!(steps.len(), 9);
        // Final step is full all-off.
        let last = steps.last().unwrap();
        assert_eq!(last.target.right, 0);
        assert_eq!(last.target.left, 0);
        assert!(!last.target.word);
        // The form-chord step (second-to-last) carries the spelled
        // word as its glyph hint and the L_IDX chord for SpelledCardinal.
        let form_step = &steps[steps.len() - 2];
        assert_eq!(form_step.target.right, 0);
        assert_eq!(form_step.target.left, Form::SpelledCardinal.chord_bits());
        assert!(!form_step.target.word);
        assert_eq!(form_step.number_glyph.as_deref(), Some("nineteen"));
    }

    #[test]
    fn spelled_form_steps_skip_phoneme_words() {
        // Words with no spelled-form mapping return None so the
        // practice builder falls through to phoneme steps.
        assert!(build_spelled_form_steps("hello").is_none());
        assert!(build_spelled_form_steps("answer").is_none());
    }
}
