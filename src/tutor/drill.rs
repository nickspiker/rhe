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
    /// Advance on `Event::Mod` instead of a chord state match. Available for steps whose advancement depends on the StateMachine's mod-alone detection (number-mode entry, decimal point) rather than a target key state. Currently unused — pristine-zero is now state-matched as `+mod+word` then `-mod-word`. Kept as scaffolding for future gestures that need event-driven advancement.
    pub advance_on_mod: bool,
    /// Hint text for the tutor's word-detail line — the one character this number-mode step emits ('3', '.', '+', etc.). None for non-number steps.
    pub number_glyph: Option<String>,
}

pub struct PracticeWord {
    pub word: String,
    pub phoneme_steps: Vec<Step>, // word held + phoneme sequence + commit
    pub brief_steps: Option<Vec<Step>>, // single chord without word + all-off
    pub suffix_steps: Option<Vec<Step>>, // roll(base) + suffix chord + all-off
    pub suffix_label: Option<String>, // e.g. "~ing" for display
    pub number_steps: Option<Vec<Step>>, // spelled-digit path: entry + finger+mod + commit
    pub number_fallback_steps: Option<Vec<Step>>, // digit-then-form path: entry + digit + commit + form chord
    pub symbol_steps: Option<Vec<Step>>, // symbol-mode path: +word+mod -word +word -mod +chord(s) + commit
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordMode {
    Brief,
    Phoneme,
    Suffix,
    Number,
    /// Digit-then-form fallback: user committed a digit (e.g. "2") in number mode without adding the spelled-form mod, then needs to apply a form chord to convert "2" → "two". Reached automatically from `Number` mode when the matcher detects the user releasing the digit finger without joining mod.
    NumberFallback,
    /// Symbol-mode session: entered via the `+word +mod -word +chord` gesture. The triggering chord fires the symbol via `crate::layout::symbols::chord_to_glyph`; mode reverts to Normal immediately after — one symbol per gesture.
    Symbol,
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
            WordMode::NumberFallback => word
                .number_fallback_steps
                .as_deref()
                .or(word.number_steps.as_deref()),
            WordMode::Symbol => word.symbol_steps.as_deref(),
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
            // Brief wins when present — even on digit words like "two"
            // that also have number_steps. The user can fall through
            // to Number / NumberFallback by pressing word, which the
            // tick() handler swaps modes on.
            if w.brief_steps.is_some() {
                WordMode::Brief
            } else if w.number_steps.is_some() && w.phoneme_steps.is_empty() {
                WordMode::Number
            } else if w.symbol_steps.is_some() && w.phoneme_steps.is_empty() {
                WordMode::Symbol
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
#[cfg(feature = "lang-en")]
pub const TEST_SENTENCES: &[&str] = &[
    // ─── Recent features ─────────────────────────────────────────────
    // Symbol mode right-hand consonants: each Greek letter routes thru crate::layout::symbols::chord_to_glyph (κ on K-chord, π on P-chord, etc.) — phoneme positions you already know.
    "type π and τ and σ for symbols",
    "use κ for kappa or λ for lambda often",
    // Symbol mode left-hand vowels: α ε ι ο υ on Ah/Eh/Ih/Ow/Iy chord shapes — sound mnemonic ("ah"=α, "eh"=ε, "ih"=ι, "oh"=ο, "ee"=υ).
    "we use α and ε for math notation",
    "and ι and ο and υ for vowels",
    // Symbol mode left-hand randos: @ # $ ~ ; etc. on the multi-finger left chords. Frequency-stable units: ° £ €.
    "send mail to nick @ home dot com",
    "the temperature is 72 ° today",
    "price is $ 5 or £ 4 or € 5",
    "use # for hash and ~ for home",
    // Pristine zero gesture: +word+mod then -word-mod, in any order.
    "the count is zero and one and two and three and four and five",
    // one/won ordered bundle on R-RING+R-thumb.
    "I won the match for us too",
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

/// Curated drill lines for the te reo Māori build. Common greetings, particles, and a sampling of vocabulary that exercises every consonant + every vowel + the digraphs (`ng`, `wh`) and macron-marked long vowels. Native-speaker review pending; first cut.
#[cfg(feature = "lang-mri")]
pub const TEST_SENTENCES: &[&str] = &[
    "kia ora",
    "tēnā koe",
    "kei te pēhea koe",
    "ko wai tō ingoa",
    "ko hēmi tōku ingoa",
    "haere mai ki tēnei wāhi",
    "kei te pai ahau",
    "he aha tēnei",
    "he reka te kai",
    "kei te kāinga te whānau",
    "ka tunua e ia ngā paraoa",
    "kāore au e mōhio",
    "hoatu ki a ia te pukapuka",
    "ka mau te wehi",
    "rangatahi me ngā pakeke",
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

/// Pristine empty-number-mode-exit gesture, drilled as two state targets: both keys held, then both keys released. The engine accepts all four press/release orders (word↓ first or mod↓ first × word↑ first or mod↑ first), and the strict state matcher does too — extras only error on a held key that isn't in target, so partial-press states (only word held, only thumb held) just wait for the other key without erroring.
pub fn build_pristine_zero_steps() -> Vec<Step> {
    let word_and_thumb = Target {
        right: 1 << 4, // R-thumb bit
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };
    let all_off = Target::default();
    vec![
        // Step 0: +mod +word — both keys held, in any order.
        Step {
            target: word_and_thumb,
            number_glyph: Some("zero".to_string()),
            ..Step::default()
        },
        // Step 1: -mod -word — both keys released, in any order.
        Step {
            target: all_off,
            ..Step::default()
        },
    ]
}

/// Build number-mode steps for spelled digit words ("zero" through "nine"). Generates: number-mode entry (word + mod alone) → finger+mod chord → commit. Special case: "zero" uses the pristine gesture (`build_pristine_zero_steps`); every other digit gets the standard 5-step spelled-digit path. The digit-word branch in `build_practice` then layers a brief (if one exists in the brief table — currently for "one", "two", "four") and a digit-then-form fallback on top, giving the user up to three drill paths per word.
pub fn build_digit_word_steps(word: &str) -> Option<Vec<Step>> {
    let lower = word.to_lowercase();
    if lower == "zero" {
        return Some(build_pristine_zero_steps());
    }
    let scan_code = match lower.as_str() {
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
/// 1. `+word+mod` — number-mode entry (word held + mod pressed alone)
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

    // Step 0: +word+mod (number-mode entry).
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

/// Build the per-step drill sequence for a number/symbol "word". Structure: number-mode entry + one step per character + commit.
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
            // Decimal: press-and-release mod again
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

/// Build the per-step drill sequence for a symbol-mode "word" (one or more glyphs mapped via `crate::layout::symbols::chord_to_glyph`). Returns `None` if any character isn't a known symbol.
///
/// Symbol mode is one-shot: each char gets its own 4-step gesture. For multi-char words the gestures are concatenated end-to-end.
///
/// Per-char step sequence (mirrors `+word +mod -word +chord ... all-off`):
/// 0. `+word +mod` — word + R-thumb both held, in either order. The strict-state matcher's pressed_in_target gate accepts the partial state where only one is held without erroring, so press order is up to the user.
/// 1. `-word` — thumb only; SM defers `[Mod, SpaceUp]` pending disambiguation.
/// 2. `+chord` — thumb still held, chord finger(s) press; SM drops the deferred bundle and fires `SymbolMode`. Word stays released — the symbol session lives in word-up brief territory and fires the chord on combined-hands all-zero.
/// 3. all-off — release word, thumb, and chord together; chord fires; interpreter emits the symbol with a trailing space and reverts to Normal.
#[cfg(feature = "lang-en")]
pub fn build_symbol_steps(word: &str) -> Option<Vec<Step>> {
    if word.is_empty() {
        return None;
    }
    // Resolve every char to its chord first — bail before allocating anything if even one char isn't a known symbol.
    let chords: Vec<(u8, u8, bool, char)> = word
        .chars()
        .map(|c| {
            let (right, left, modkey) = crate::layout::symbols::glyph_to_chord(c)?;
            Some((right, left, modkey, c))
        })
        .collect::<Option<Vec<_>>>()?;

    let word_and_thumb = Target {
        right: 1 << 4,
        left: 0,
        word: true,
        accepted_leads: KeyMask::EMPTY,
    };
    let thumb_only = Target {
        right: 1 << 4,
        left: 0,
        word: false,
        accepted_leads: KeyMask::EMPTY,
    };

    let mut steps: Vec<Step> = Vec::new();
    for (right, left, modkey, c) in chords {
        // Mod-bearing chord shapes are unreachable in symbol mode today (see `crate::layout::symbols::chord_to_glyph`). The `glyph_to_chord` table currently lists none, so this is defensive.
        let chord_right = right | if modkey { 1 << 4 } else { 0 };
        // Step 0: +word +mod (word + thumb both held; either order)
        steps.push(Step {
            target: word_and_thumb,
            ..Step::default()
        });
        // Step 1: -word (thumb only) — defers Mod + SpaceUp
        steps.push(Step {
            target: thumb_only,
            ..Step::default()
        });
        // Step 2: +chord (thumb + chord, word released) — fires SymbolMode
        steps.push(Step {
            target: Target {
                right: chord_right | (1 << 4),
                left,
                word: false,
                accepted_leads: KeyMask::EMPTY,
            },
            number_glyph: Some(c.to_string()),
            ..Step::default()
        });
        // Step 3: all-off — chord fires, symbol emitted, mode reverts
        steps.push(Step {
            target: Target::default(),
            ..Step::default()
        });
    }

    Some(steps)
}

#[cfg(not(feature = "lang-en"))]
pub fn build_symbol_steps(_word: &str) -> Option<Vec<Step>> {
    None
}

/// Look up the brief chord for `word` (lowercase, alphabetic) in the brief table and build the standard 2-step drill (chord, all-off) — same shape as the inline brief lookup in `build_practice`'s phoneme branch. Returns `None` if the word has no brief.
fn brief_steps_for_word(word: &str, brief_table: &BriefTable) -> Option<Vec<Step>> {
    let mut chord_for_word: Option<(u8, u8)> = None;
    let mut leads = KeyMask::EMPTY;
    for (key, first_down, brief_word) in brief_table.iter() {
        if brief_word.trim() != word {
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
                if let Some(symbol_steps) = build_symbol_steps(word_str) {
                    sentence.push(PracticeWord {
                        word: word_str.to_string(),
                        phoneme_steps: Vec::new(),
                        brief_steps: None,
                        suffix_steps: None,
                        suffix_label: None,
                        number_steps: None,
                        number_fallback_steps: None,
                        symbol_steps: Some(symbol_steps),
                    });
                    continue;
                }

                if let Some(number_steps) = build_number_steps(word_str) {
                    sentence.push(PracticeWord {
                        word: word_str.to_string(),
                        phoneme_steps: Vec::new(),
                        brief_steps: None,
                        suffix_steps: None,
                        suffix_label: None,
                        number_steps: Some(number_steps),
                        number_fallback_steps: None,
                        symbol_steps: None,
                    });
                    continue;
                }

                if let Some(number_steps) = build_digit_word_steps(word_str) {
                    // Spelled digit words like "two" / "three" / etc. get
                    // up to three drill paths the user can choose between:
                    // (1) brief — if a brief like the to/too/two ordered
                    //     bundle exists for this word; default mode.
                    // (2) number_steps (built above) — spelled-digit
                    //     gesture: enter number mode, finger-then-mod.
                    // (3) number_fallback_steps — digit-then-form: same
                    //     entry but commit just the digit, then apply the
                    //     SpelledCardinal form chord. Reached
                    //     automatically when the user diverges from
                    //     path 2 by releasing the digit finger without
                    //     joining mod.
                    let lower = word_str.to_lowercase();
                    let brief_steps = brief_steps_for_word(&lower, brief_table);
                    // "zero" is special: number_steps is the pristine
                    // 2-step gesture, which never reaches a +finger+mod
                    // state, so the divergence detector that powers
                    // Path 3 can't fire. Skip building the fallback —
                    // it would be dead weight.
                    let number_fallback_steps = if lower == "zero" {
                        None
                    } else {
                        build_spelled_form_steps(word_str)
                    };
                    sentence.push(PracticeWord {
                        word: word_str.to_string(),
                        phoneme_steps: Vec::new(),
                        brief_steps,
                        suffix_steps: None,
                        suffix_label: None,
                        number_steps: Some(number_steps),
                        number_fallback_steps,
                        symbol_steps: None,
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
                        number_fallback_steps: None,
                        symbol_steps: None,
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
                    advance_on_mod: false,
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
                    number_fallback_steps: None,
                    symbol_steps: None,
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
        Some(w) if w.brief_steps.is_some() => WordMode::Brief,
        Some(w) if w.number_steps.is_some() && w.phoneme_steps.is_empty() => WordMode::Number,
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
/// Owns a private `StateMachine` so it can spot `Mod` events and advance number-mode entry steps. This is independent of the engine thread's interpreter — the drill matches what the user *should* be chording, while the engine still types the *actual* text into the focused app.
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
    /// Set when a goof needs the drill to roll back to step 0, but the
    /// actual `practice.reset_word()` call is held until errored
    /// clears so the user keeps seeing the failed target while they
    /// wind down their bad press. Without this, the drill snaps back
    /// to step 0 the moment they goof — distracting and especially
    /// noticeable in number mode where step 0 is the +mod+word
    /// re-entry chord.
    pending_reset: bool,
    /// Set when the goof happened after the user had already made
    /// progress (step_idx > 0). In that case the rollback throws
    /// away typed chords that the engine already committed to the
    /// focused app, and we require a full all_off (including word)
    /// before clearing errored so the engine has a chance to commit
    /// and reset cleanly. Goofs at step 0 don't set this — they're
    /// trivial first-chord misses where the user can stay in the
    /// word-held context and retry the same chord.
    requires_word_release: bool,
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
            pending_reset: false,
            requires_word_release: false,
            sm: crate::state_machine::StateMachine::new(),
        }
    }

    /// True when the user's input has committed to symbol-mode entry — the SM is post-`+word +mod -word` with thumb still live, and the next chord-finger press will fire `Event::SymbolMode`. The tutor reads this to flip cell hints to Greek/rando labels at the trigger moment.
    pub fn pending_symbol_entry(&self) -> bool {
        self.sm.pending_symbol_entry()
    }

    /// A wrong key for the current step. Mark the drill as botched and
    /// either reset immediately (if the user already has hands clear,
    /// no errored period to wait through) or defer the actual rollback
    /// to step 0 until errored clears — so the visual target stays on
    /// the failed step throughout the user's wind-down. If the goof
    /// happened after some progress was made (step_idx > 0), also
    /// flag that the user needs to release word too before retrying:
    /// the rollback throws away typed chords that the engine already
    /// committed to the focused app, so word-up gives the engine a
    /// commit-and-reset boundary.
    fn goof(&mut self, all_off: bool, prev_step_idx: usize) {
        self.last_was_botch = true;
        if all_off {
            self.practice.reset_word();
        } else {
            self.errored = true;
            self.pending_reset = true;
            if prev_step_idx > 0 {
                self.requires_word_release = true;
            }
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

        let mut step_advanced_by_mod = false;
        for sm_event in self.sm.feed(rhe_event) {
            if matches!(sm_event, crate::state_machine::Event::Mod { .. }) {
                let is_mod_advance_step = self
                    .practice
                    .current_step()
                    .map_or(false, |s| s.advance_on_mod);
                if is_mod_advance_step {
                    self.practice.advance_step();
                    step_advanced_by_mod = true;
                } else if self.practice.mode == WordMode::Phoneme && self.key_state.word {
                    // Phoneme-mode mod-alone (thumb tap while word
                    // held): the engine deletes the last buffered
                    // phoneme. Mirror in the drill.
                    //
                    // step_idx > 0:
                    // - Errored: the goofed step's wrong phoneme is
                    //   what got popped. Clear errored without
                    //   changing step_idx so the user retries the
                    //   same target. Bypasses the requires_word_release
                    //   gate that normally blocks errored-clear while
                    //   word is still held.
                    // - Not errored: the most recently confirmed
                    //   phoneme is what got popped. Roll step_idx
                    //   back to the start of that phoneme. Phoneme
                    //   step layout alternates chord/release, so an
                    //   even step_idx (just released, ready for next
                    //   chord) rolls back two; an odd step_idx (just
                    //   typed a chord) rolls back one.
                    //
                    // step_idx == 0: nothing to undo, but still clear
                    // accumulators so the residual thumb bit doesn't
                    // trip the post-release goof on the next tick.
                    if self.practice.step_idx > 0 {
                        if self.errored {
                            self.errored = false;
                            self.pending_reset = false;
                            self.requires_word_release = false;
                            self.last_was_botch = false;
                        } else {
                            let target_step = if self.practice.step_idx % 2 == 0 {
                                self.practice.step_idx.saturating_sub(2)
                            } else {
                                self.practice.step_idx - 1
                            };
                            self.practice.step_idx = target_step;
                        }
                    }
                    self.touched_right = 0;
                    self.touched_left = 0;
                    self.tutor_first_down = None;
                    self.chord_right_acc = 0;
                    self.chord_left_acc = 0;
                    step_advanced_by_mod = true;
                }
            }
        }

        let all_off = self.key_state.right_bits() == 0
            && self.key_state.left_bits() == 0
            && !self.key_state.word;

        if self.errored {
            // Clear when all fingers are off. Word stays optional for
            // trivial step-0 misses (no progress made yet — user can
            // retry the same chord without losing the word-held
            // context); but if the goof discarded progress
            // (`requires_word_release`), require word-up too so the
            // engine's interpreter commits and resets cleanly before
            // the user retries the word from the top.
            let hands_off =
                self.key_state.right_bits() == 0 && self.key_state.left_bits() == 0;
            let cleared = if self.requires_word_release {
                hands_off && !self.key_state.word
            } else {
                hands_off
            };
            if cleared {
                self.errored = false;
                self.touched_right = 0;
                self.touched_left = 0;
                self.tutor_first_down = None;
                self.requires_word_release = false;
                if self.pending_reset {
                    self.practice.reset_word();
                    self.pending_reset = false;
                }
            }
        } else {
            let is_key_down = rhe_event.direction == KeyDirection::Down;

            if rhe_event.scan == scan::WORD && rhe_event.direction == KeyDirection::Down {
                let is_number_word = self.practice.current_word().map_or(false, |w| {
                    w.number_steps.is_some() && w.phoneme_steps.is_empty()
                });
                let is_symbol_word = self.practice.current_word().map_or(false, |w| {
                    w.symbol_steps.is_some() && w.phoneme_steps.is_empty()
                });
                // Multi-symbol words concatenate one 5-step gesture
                // per char, so a `+word` mid-PracticeWord (step_idx > 0)
                // is the start of the next char's gesture, not a
                // restart of the current one.
                let in_symbol_gesture = is_symbol_word
                    && self.practice.mode == WordMode::Symbol
                    && self.practice.step_idx > 0;
                if !in_symbol_gesture {
                    self.practice.mode = if is_symbol_word {
                        WordMode::Symbol
                    } else if is_number_word {
                        WordMode::Number
                    } else {
                        WordMode::Phoneme
                    };
                    self.practice.step_idx = 0;
                    self.fingers_during_word = false;
                }
            } else if rhe_event.scan == scan::WORD
                && rhe_event.direction == KeyDirection::Up
                && matches!(
                    self.practice.mode,
                    WordMode::Phoneme | WordMode::Number | WordMode::Symbol
                )
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
                //
                // Symbol mode is excluded: its gesture has a
                // legitimate -word at step 2 (target word=f), so the
                // generic check would false-positive there. The
                // Symbol matcher's pressed-in-target rule + greedy
                // advance catches mis-releases without it.
                self.goof(all_off, prev_step_idx);
            }

            if self.key_state.word && is_key_down && rhe_event.scan != scan::WORD {
                self.fingers_during_word = true;
            }

            if step_advanced_by_mod {
                // already advanced above; fall through to step-transition reseed
            } else if self
                .practice
                .current_step()
                .map_or(false, |s| s.advance_on_mod)
            {
                // advance_on_mod step: tolerate R-thumb press/release without
                // overshoot errors. Advance fires above via the Mod sm
                // event when R-thumb releases. Any other finger going down
                // is still a botch — the gesture is "thumb alone, no other
                // input." Hoisted above the Number-mode arm so any future
                // event-driven step doesn't trip the strict state matcher
                // on R-thumb down.
                if is_key_down
                    && rhe_event.scan != scan::WORD
                    && rhe_event.scan != scan::R_THUMB
                {
                    self.goof(all_off, prev_step_idx);
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
                            self.goof(all_off, prev_step_idx);
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
                            // Path 2 → Path 3 fallback. The user is in
                            // the spelled-digit gesture (number mode,
                            // target = +finger+mod) and just released
                            // the digit finger without joining mod.
                            // That commits "2" instead of "two" in the
                            // engine; recover by jumping to the
                            // digit-then-form (Path 3) drill where the
                            // user can complete with a SpelledCardinal
                            // form chord. Only fires when the word has
                            // a number_fallback_steps lane and the
                            // current step actually expects mod (target
                            // includes the R-thumb bit) — that's the
                            // exact moment Path 2 and Path 3 diverge.
                            let target_has_mod_and_finger = (target.right & (1u8 << 4)) != 0
                                && ((target.right & !(1u8 << 4)) != 0 || target.left != 0);
                            let post_release_state_word_only = state_right == 0
                                && state_left == 0
                                && state_word;
                            let has_fallback = self
                                .practice
                                .current_word()
                                .map_or(false, |w| w.number_fallback_steps.is_some());
                            if target_has_mod_and_finger
                                && post_release_state_word_only
                                && has_fallback
                            {
                                // Jump to Path 3 step 4 (release-word).
                                // Path 3 layout for a single-digit word:
                                // 0:+word+mod, 1:-mod, 2:+digit, 3:-digit,
                                // 4:-word (commits digit), 5:+form, 6:-form.
                                // The release we just processed satisfied
                                // step 3, so we land on step 4.
                                self.practice.mode = WordMode::NumberFallback;
                                self.practice.step_idx = 4;
                            } else {
                                self.goof(all_off, prev_step_idx);
                            }
                        }
                    }
                }
            } else if self.practice.mode == WordMode::Symbol {
                // Symbol mode: pure state matching with greedy
                // advance. The gesture cycles through targets that
                // alternate between word-held and word-released
                // states (e.g. step 2 has target word=f, step 3
                // word=t), so a single tick can satisfy multiple
                // step transitions when the state lines up. Greedy
                // advance walks the step index forward as long as
                // each new target is already met.
                //
                // Errors: if the user pressed a key that isn't in
                // the current target after greedy advance, that's a
                // wrong-key goof. If they released a target-bearing
                // key without the target being fully met, that's a
                // dropped-chord goof. Releasing word, thumb, or
                // chord keys at the legitimate transition points
                // doesn't trigger either branch because greedy
                // advance has already moved past them.
                let mut advanced_this_tick = false;
                loop {
                    let target = match self.practice.current_target() {
                        Some(t) => *t,
                        None => break,
                    };
                    if self.key_state.right_bits() == target.right
                        && self.key_state.left_bits() == target.left
                        && self.key_state.word == target.word
                    {
                        self.practice.advance_step();
                        advanced_this_tick = true;
                    } else {
                        break;
                    }
                }
                if !advanced_this_tick {
                    if let Some(target) = self.practice.current_target() {
                        let target = *target;
                        if is_key_down {
                            let pressed_in_target = if let Some(bit) =
                                scan::right_bit(rhe_event.scan)
                            {
                                (target.right & (1u8 << bit)) != 0
                            } else if let Some(bit) = scan::left_bit(rhe_event.scan) {
                                (target.left & (1u8 << bit)) != 0
                            } else if rhe_event.scan == scan::WORD {
                                target.word
                            } else {
                                false
                            };
                            if !pressed_in_target {
                                self.goof(all_off, prev_step_idx);
                            }
                        } else {
                            let regress_right = scan::right_bit(rhe_event.scan)
                                .map(|bit| (target.right & (1u8 << bit)) != 0)
                                .unwrap_or(false);
                            let regress_left = scan::left_bit(rhe_event.scan)
                                .map(|bit| (target.left & (1u8 << bit)) != 0)
                                .unwrap_or(false);
                            let target_met = self.key_state.right_bits() == target.right
                                && self.key_state.left_bits() == target.left
                                && self.key_state.word == target.word;
                            if (regress_right || regress_left) && !target_met {
                                self.goof(all_off, prev_step_idx);
                            }
                        }
                    }
                }
            } else if let Some(target) = self.practice.current_target() {
                let target = *target;
                let step = self.practice.current_step().unwrap();
                let advance_on_mod = step.advance_on_mod;
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

                if advance_on_mod {
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && rhe_event.scan != scan::R_THUMB
                    {
                        self.goof(all_off, prev_step_idx);
                    }
                } else if space_only {
                    if is_key_down
                        && rhe_event.scan != scan::WORD
                        && !bounce_of_prev(rhe_event.scan)
                    {
                        self.goof(all_off, prev_step_idx);
                    } else if !self.key_state.word {
                        self.last_was_botch = false;
                        self.practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && !target.word {
                    if is_key_down && !bounce_of_prev(rhe_event.scan) {
                        self.goof(all_off, prev_step_idx);
                    } else if all_off {
                        self.last_was_botch = false;
                        self.practice.advance_step();
                    }
                } else if target.right == 0 && target.left == 0 && target.word {
                    if is_key_down && rhe_event.scan != scan::WORD {
                        self.goof(all_off, prev_step_idx);
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

                    // Thumb-only press in progress: candidate mod-tap
                    // gesture (engine reads as `Event::Mod`, which the
                    // sm_event loop above treats as phoneme-undo).
                    // Suppress the goof so the user reaches thumb-up;
                    // if they DO press a non-thumb key before releasing,
                    // chord_*_acc grows past thumb-only and the gate
                    // re-engages naturally.
                    let thumb_only_acc =
                        self.chord_right_acc == (1u8 << 4) && self.chord_left_acc == 0;

                    let space_dropped = rhe_event.scan == scan::WORD
                        && rhe_event.direction == KeyDirection::Up
                        && target.word
                        && self.practice.step_idx > 0;

                    if space_dropped {
                        self.goof(all_off, prev_step_idx);
                    } else if is_key_down && has_extra_acc && !thumb_only_acc {
                        self.goof(all_off, prev_step_idx);
                    } else if acc_matches && hand_touched {
                        let first_down_ok = target.accepted_leads.is_empty()
                            || self
                                .tutor_first_down
                                .map(|fd| target.accepted_leads.test(fd))
                                .unwrap_or(false);
                        if !first_down_ok {
                            self.goof(all_off, prev_step_idx);
                        } else {
                            self.practice.advance_step();
                        }
                    } else if hand_touched && target_hands_empty && !is_key_down {
                        self.goof(all_off, prev_step_idx);
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
    in_symbol_mode: bool,
) -> String {
    if in_symbol_mode {
        // Symbol mode shows what each cell would emit on its own — single-finger lookup against the symbol-mode table, ignoring held_mask. The session is one-shot so there's no accumulating chord context to combine with; the user reads each cell's label as "this finger alone = this glyph". For multi-finger symbols (κ on R-IDX+R-MID, π on R-IDX+R-MID+R-PINKY, etc.) the cell highlighting still tells them which fingers to combine.
        let mut candidate = KeyMask::EMPTY;
        candidate.set(cell_scan);
        let chord = ChordKey::from_mask(candidate);
        return crate::layout::symbols::chord_to_glyph(chord)
            .map(|s| s.to_string())
            .unwrap_or_default();
    }
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

// Drill tests use English-specific Form variants, English digit-word multi-path drill, and English phoneme paths. Skip under lang-mri — Māori has no number forms / suffix system / homophone bundles in v0.1.2 so the tests don't apply.
#[cfg(all(test, feature = "lang-en"))]
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

    #[test]
    fn pristine_zero_steps_shape() {
        let steps = build_pristine_zero_steps();
        // 2 state targets: both held, then both released.
        assert_eq!(steps.len(), 2);
        // Step 0: word + R-thumb both held.
        assert!(steps[0].target.word);
        assert_eq!(steps[0].target.right, 1 << 4);
        // Step 1: all off.
        assert!(!steps[1].target.word);
        assert_eq!(steps[1].target.right, 0);
    }

    fn run_pristine_zero(events: &[(u8, KeyDirection)]) -> TutorState {
        let practice = Practice {
            sentences: vec![vec![PracticeWord {
                word: "zero".to_string(),
                phoneme_steps: Vec::new(),
                brief_steps: None,
                suffix_steps: None,
                suffix_label: None,
                number_steps: Some(build_pristine_zero_steps()),
                number_fallback_steps: None,
                symbol_steps: None,
            }]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Number,
            wrapped: false,
        };
        let mut state = TutorState::new(practice);
        for &(scan, direction) in events {
            state.tick(RheKeyEvent { scan, direction });
        }
        state
    }

    /// All four engine-accepted orders should drill cleanly through both
    /// steps without botching: press order × release order.
    #[test]
    fn pristine_zero_drill_word_first_thumb_first_release() {
        // word↓, thumb↓, thumb↑, word↑
        let s = run_pristine_zero(&[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::WORD, KeyDirection::Up),
        ]);
        assert!(!s.errored);
    }

    #[test]
    fn pristine_zero_drill_word_first_word_first_release() {
        // word↓, thumb↓, word↑, thumb↑
        let s = run_pristine_zero(&[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ]);
        assert!(!s.errored);
    }

    #[test]
    fn pristine_zero_drill_thumb_first_thumb_first_release() {
        // thumb↓, word↓, thumb↑, word↑
        let s = run_pristine_zero(&[
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::WORD, KeyDirection::Up),
        ]);
        assert!(!s.errored);
    }

    #[test]
    fn pristine_zero_drill_thumb_first_word_first_release() {
        // thumb↓, word↓, word↑, thumb↑
        let s = run_pristine_zero(&[
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ]);
        assert!(!s.errored);
    }

    // ─── Symbol-mode drill ───

    #[test]
    fn symbol_steps_shape_for_single_letter() {
        // π is on the all-4-right chord (effort rank 4) under the arXiv-frequency layout — it's the 5th-most-common Greek consonant in math/physics writing.
        let steps = build_symbol_steps("π").expect("π should map");
        // 0:+word+mod 1:-word 2:+chord(thumb+chord, word=f) 3:all-off
        assert_eq!(steps.len(), 4);
        // Step 0: word + thumb (in either press order).
        assert_eq!(steps[0].target.right, 1 << 4);
        assert!(steps[0].target.word);
        // Step 1: thumb only (-word).
        assert_eq!(steps[1].target.right, 1 << 4);
        assert!(!steps[1].target.word);
        // Step 2: thumb + π-chord (all 4 right fingers), word released.
        assert_eq!(steps[2].target.right, 0b1111 | (1 << 4));
        assert!(!steps[2].target.word);
        assert_eq!(steps[2].number_glyph.as_deref(), Some("π"));
        // Step 3: full all-off.
        assert_eq!(steps[3].target.right, 0);
        assert_eq!(steps[3].target.left, 0);
        assert!(!steps[3].target.word);
    }

    #[test]
    fn symbol_steps_skip_unknown_chars() {
        // "hello" has no symbol mapping for any char.
        assert!(build_symbol_steps("hello").is_none());
        // Mixed (Greek + ASCII) also returns None — symbol-mode is
        // all-or-nothing per word.
        assert!(build_symbol_steps("πa").is_none());
    }

    fn pi_practice() -> Practice {
        let symbol_steps = build_symbol_steps("π").expect("π builds");
        let word = PracticeWord {
            word: "π".to_string(),
            phoneme_steps: Vec::new(),
            brief_steps: None,
            suffix_steps: None,
            suffix_label: None,
            number_steps: None,
            number_fallback_steps: None,
            symbol_steps: Some(symbol_steps),
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Symbol,
            wrapped: false,
        }
    }

    /// Canonical symbol gesture for π drills cleanly: enter symbol mode via `+word +mod -word +chord`, with thumb still held when the chord finger(s) press, then release everything.
    #[test]
    fn pi_symbol_drills_clean() {
        let mut state = TutorState::new(pi_practice());
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down), // step 0 satisfied (word+thumb)
            (scan::WORD, KeyDirection::Up),      // → step 1 (thumb only)
            // π chord (all 4 right fingers) with thumb still held, word released → step 2 satisfied.
            (scan::R_IDX, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Down),
            (scan::R_RING, KeyDirection::Down),
            (scan::R_PINKY, KeyDirection::Down),
            // Release everything (chord + thumb) → step 3 (all-off).
            (scan::R_IDX, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Up),
            (scan::R_RING, KeyDirection::Up),
            (scan::R_PINKY, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "symbol π should drill without botching");
    }

    /// Press order independence for step 0 (word + thumb): thumb first then word should drill just as clean as word first then thumb. The strict-state matcher's pressed_in_target gate covers the partial-state ticks without erroring.
    #[test]
    fn pi_symbol_thumb_first_drills_clean() {
        let mut state = TutorState::new(pi_practice());
        for &(scan, direction) in &[
            (scan::R_THUMB, KeyDirection::Down), // partial state — gate accepts
            (scan::WORD, KeyDirection::Down),    // → step 0 satisfied
            (scan::WORD, KeyDirection::Up),      // → step 1 (thumb only)
            (scan::R_IDX, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Down),
            (scan::R_RING, KeyDirection::Down),
            (scan::R_PINKY, KeyDirection::Down), // → step 2 satisfied
            (scan::R_IDX, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Up),
            (scan::R_RING, KeyDirection::Up),
            (scan::R_PINKY, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),   // → step 3 (all-off)
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "thumb-first symbol entry should drill clean");
    }

    /// Pressing a wrong finger inside symbol mode (during the chord step) should goof — the user diverged from the target chord. L_PINKY is on a left-hand chord shape (not part of π's right-hand all-4 target).
    #[test]
    fn symbol_wrong_chord_botches() {
        let mut state = TutorState::new(pi_practice());
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::L_PINKY, KeyDirection::Down),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(state.errored, "wrong finger in symbol chord should goof");
    }

    fn single_symbol_practice(c: char) -> Practice {
        let symbol_steps = build_symbol_steps(&c.to_string())
            .unwrap_or_else(|| panic!("no symbol mapping for {}", c));
        let word = PracticeWord {
            word: c.to_string(),
            phoneme_steps: Vec::new(),
            brief_steps: None,
            suffix_steps: None,
            suffix_label: None,
            number_steps: None,
            number_fallback_steps: None,
            symbol_steps: Some(symbol_steps),
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Symbol,
            wrapped: false,
        }
    }

    /// α (alpha) on the L_MID chord (M = effort rank 3 = 4th-easiest single finger).
    #[test]
    fn alpha_symbol_drills_clean() {
        let mut state = TutorState::new(single_symbol_practice('α'));
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::L_MID, KeyDirection::Down),
            (scan::L_MID, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "α should drill cleanly");
    }

    /// @ on L_PINKY (P = effort rank 2 = 3rd-easiest single finger). High-frequency ASCII rando, ranked above α since `@` shows up everywhere in modern text.
    #[test]
    fn at_sign_symbol_drills_clean() {
        let mut state = TutorState::new(single_symbol_practice('@'));
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::L_PINKY, KeyDirection::Down),
            (scan::L_PINKY, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "@ should drill cleanly");
    }

    /// ° on L_IDX (I = effort rank 0 = the easiest chord). Frequency scan rank 2 across runs — clearest "yes, this glyph deserves the easiest slot" signal in the data.
    #[test]
    fn degree_symbol_drills_clean() {
        let mut state = TutorState::new(single_symbol_practice('°'));
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::WORD, KeyDirection::Up),
            (scan::L_IDX, KeyDirection::Down),
            (scan::L_IDX, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "° should drill cleanly");
    }

    // (Round-trip glyph↔chord coverage lives in `layout::en::symbols::tests::forward_reverse_roundtrip` — the test there iterates the actual table arrays so it doesn't drift when entries are added or moved.)

    // ─── Three-path drill for digit words (e.g. "two") ───
    //
    // build_practice produces brief_steps + number_steps +
    // number_fallback_steps for digit words that have a brief. Here
    // we hand-construct the three step lists so the test doesn't
    // need a real brief table. The actual paths under test:
    //   Path 1 (Brief)           — to/too/two ordered chord
    //   Path 2 (Number)          — entry, finger-then-mod, all-off
    //   Path 3 (NumberFallback)  — entry, digit, commit, form chord

    fn two_practice() -> Practice {
        // Brief chord for "two": R-IDX + R-MID + R-thumb (right=0b10011),
        // R-MID first-down disambiguates as "two" within to/too/two.
        let mut leads = KeyMask::EMPTY;
        leads.set(scan::R_MID);
        let brief_steps = vec![
            Step {
                target: Target {
                    right: 0b10011,
                    left: 0,
                    word: false,
                    accepted_leads: leads,
                },
                ..Step::default()
            },
            Step {
                target: Target::default(),
                ..Step::default()
            },
        ];
        let number_steps = build_digit_word_steps("two").expect("two builds");
        let number_fallback_steps =
            build_spelled_form_steps("two").expect("two has spelled form");
        let word = PracticeWord {
            word: "two".to_string(),
            phoneme_steps: Vec::new(),
            brief_steps: Some(brief_steps),
            suffix_steps: None,
            suffix_label: None,
            number_steps: Some(number_steps),
            number_fallback_steps: Some(number_fallback_steps),
            symbol_steps: None,
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Brief,
            wrapped: false,
        }
    }

    fn run_two(events: &[(u8, KeyDirection)]) -> TutorState {
        let mut state = TutorState::new(two_practice());
        for &(scan, direction) in events {
            state.tick(RheKeyEvent { scan, direction });
        }
        state
    }

    /// Default mode for digit word "two" is Brief — the ordered brief
    /// is the priority path even when number_steps is also populated.
    #[test]
    fn two_default_mode_is_brief() {
        let p = two_practice();
        assert_eq!(p.mode, WordMode::Brief);
    }

    /// Path 1: brief chord with R-MID landing first.
    #[test]
    fn two_path_brief_drills_clean() {
        let s = run_two(&[
            (scan::R_MID, KeyDirection::Down),
            (scan::R_IDX, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Up),
            (scan::R_IDX, KeyDirection::Up),
            (scan::R_THUMB, KeyDirection::Up),
        ]);
        assert!(!s.errored, "brief path should drill without botching");
    }

    /// Path 2: spelled-digit gesture. Pressing word switches Brief →
    /// Number; the rest is the standard 5-target spelled-digit path.
    #[test]
    fn two_path_spelled_digit_drills_clean() {
        let s = run_two(&[
            (scan::WORD, KeyDirection::Down),    // → Number mode, step 0
            (scan::R_THUMB, KeyDirection::Down), // step 0 target = +word+mod ✓
            (scan::R_THUMB, KeyDirection::Up),   // step 1 target = -mod ✓
            (scan::R_MID, KeyDirection::Down),   // step 2 target = +finger ✓
            (scan::R_THUMB, KeyDirection::Down), // step 3 target = +finger+mod ✓
            (scan::R_THUMB, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Up),
            (scan::WORD, KeyDirection::Up), // step 4 target = all-off ✓
        ]);
        assert!(!s.errored, "Path 2 should drill without botching");
    }

    fn one_practice() -> Practice {
        // Brief chord for "one": R-RING + R-thumb (right=0b10100),
        // R-thumb first-down disambiguates as "one" within one/won.
        let mut leads = KeyMask::EMPTY;
        leads.set(scan::R_THUMB);
        let brief_steps = vec![
            Step {
                target: Target {
                    right: 0b10100,
                    left: 0,
                    word: false,
                    accepted_leads: leads,
                },
                ..Step::default()
            },
            Step {
                target: Target::default(),
                ..Step::default()
            },
        ];
        let number_steps = build_digit_word_steps("one").expect("one builds");
        let number_fallback_steps =
            build_spelled_form_steps("one").expect("one has spelled form");
        let word = PracticeWord {
            word: "one".to_string(),
            phoneme_steps: Vec::new(),
            brief_steps: Some(brief_steps),
            suffix_steps: None,
            suffix_label: None,
            number_steps: Some(number_steps),
            number_fallback_steps: Some(number_fallback_steps),
            symbol_steps: None,
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Brief,
            wrapped: false,
        }
    }

    /// "one" gets the same three-path treatment as "two" — brief
    /// default, spelled-digit fallback on word-press, digit-then-form
    /// fallback on finger-release-without-mod.
    #[test]
    fn one_default_mode_is_brief() {
        assert_eq!(one_practice().mode, WordMode::Brief);
    }

    /// Brief drill for "one": R-thumb landing first within R-RING+R-thumb.
    #[test]
    fn one_path_brief_drills_clean() {
        let mut state = TutorState::new(one_practice());
        for &(scan, direction) in &[
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_RING, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::R_RING, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored, "one brief path should drill clean");
    }

    fn five_practice() -> Practice {
        // "five" has no brief — default mode is Number.
        let number_steps = build_digit_word_steps("five").expect("five builds");
        let number_fallback_steps =
            build_spelled_form_steps("five").expect("five has spelled form");
        let word = PracticeWord {
            word: "five".to_string(),
            phoneme_steps: Vec::new(),
            brief_steps: None,
            suffix_steps: None,
            suffix_label: None,
            number_steps: Some(number_steps),
            number_fallback_steps: Some(number_fallback_steps),
            symbol_steps: None,
        };
        let initial_mode = match Some(&word) {
            Some(w) if w.brief_steps.is_some() => WordMode::Brief,
            Some(w) if w.number_steps.is_some() && w.phoneme_steps.is_empty() => WordMode::Number,
            _ => WordMode::Phoneme,
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: initial_mode,
            wrapped: false,
        }
    }

    /// "five" has no brief, so default mode is Number.
    #[test]
    fn five_default_mode_is_number() {
        assert_eq!(five_practice().mode, WordMode::Number);
    }

    /// Path 3 fallback works for "five" (digit on left hand) just as
    /// it does for "two" (digit on right hand).
    #[test]
    fn five_path_digit_then_form_falls_back_cleanly() {
        let mut state = TutorState::new(five_practice());
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::L_IDX_INNER, KeyDirection::Down),
            (scan::L_IDX_INNER, KeyDirection::Up), // divergence
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert!(!state.errored);
        assert_eq!(state.practice.mode, WordMode::NumberFallback);
        assert_eq!(state.practice.step_idx, 4);
    }

    /// Path 3: user starts Path 2 but releases the digit finger
    /// without joining mod. Drill switches to NumberFallback at the
    /// release-word step; user finishes with the SpelledCardinal
    /// form chord (L-IDX) to convert "2" → "two".
    #[test]
    fn two_path_digit_then_form_falls_back_cleanly() {
        let s = run_two(&[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Down), // Number step 2 → step 3
            (scan::R_MID, KeyDirection::Up),   // divergence: switch to NumberFallback at step 4
        ]);
        assert!(!s.errored, "fallback divergence should not botch");
        assert_eq!(s.practice.mode, WordMode::NumberFallback);
        assert_eq!(s.practice.step_idx, 4, "should land on -word step");

        // Continue: release word (commits "2"), then form chord, then release.
        let s = run_two(&[
            (scan::WORD, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Down),
            (scan::R_THUMB, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Up),
            (scan::WORD, KeyDirection::Up),  // step 4 ✓
            (scan::L_IDX, KeyDirection::Down), // step 5 (+form chord) ✓
            (scan::L_IDX, KeyDirection::Up),   // step 6 (-form) ✓
        ]);
        assert!(!s.errored, "full Path 3 should drill clean");
    }

    // ─── Phoneme-mode mod-undo (thumb tap deletes one phoneme) ───
    //
    // Build a tiny 2-phoneme practice from raw chord targets so the
    // test doesn't need a CMU dictionary. K = right 0b0011 (R_IDX +
    // R_MID); T = right 0b0101 (R_IDX + R_RING) — picked just to be
    // distinct, exact bits don't matter, only that they alternate
    // correctly through the 5-step phoneme layout (chord, release,
    // chord, release, commit).

    fn kt_practice() -> Practice {
        let chord_step = |right: u8| Step {
            target: Target {
                right,
                left: 0,
                word: true,
                accepted_leads: KeyMask::EMPTY,
            },
            ..Step::default()
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
        let phoneme_steps = vec![
            chord_step(0b0011), // step 0: phoneme[0] chord
            release_step(),     // step 1: release
            chord_step(0b0101), // step 2: phoneme[1] chord
            release_step(),     // step 3: release
            Step {
                target: Target::default(), // step 4: commit (all-off)
                space_only: true,
                ..Step::default()
            },
        ];
        let word = PracticeWord {
            word: "kt".to_string(),
            phoneme_steps,
            brief_steps: None,
            suffix_steps: None,
            suffix_label: None,
            number_steps: None,
            number_fallback_steps: None,
            symbol_steps: None,
        };
        Practice {
            sentences: vec![vec![word]],
            sentence_idx: 0,
            word_idx: 0,
            step_idx: 0,
            mode: WordMode::Phoneme,
            wrapped: false,
        }
    }

    /// After a goof at step 2 (phoneme[1] target), pressing mod alone
    /// while word is still held should clear errored and leave step_idx
    /// at the same step so the user can retry.
    #[test]
    fn phoneme_mode_mod_undo_clears_errored() {
        let mut state = TutorState::new(kt_practice());
        // Type phoneme[0] correctly.
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),    // step 0 stays (target K)
            (scan::R_IDX, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Down),   // K complete → step 1
            (scan::R_IDX, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Up),     // release → step 2
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert_eq!(state.practice.step_idx, 2, "should be at phoneme[1] target");
        assert!(!state.errored);

        // Goof phoneme[1]: press wrong chord (R_PINKY isn't part of T).
        state.tick(RheKeyEvent {
            scan: scan::R_PINKY,
            direction: KeyDirection::Down,
        });
        assert!(state.errored, "wrong chord should goof");
        let goofed_step = state.practice.step_idx;

        // Release the wrong chord (errored stays — requires_word_release blocks while word held).
        state.tick(RheKeyEvent {
            scan: scan::R_PINKY,
            direction: KeyDirection::Up,
        });
        assert!(state.errored, "still errored while word held");

        // Mod-tap (thumb down + up) while word held should ungoof.
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Down,
        });
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Up,
        });
        assert!(!state.errored, "mod tap should clear errored");
        assert_eq!(
            state.practice.step_idx, goofed_step,
            "step_idx stays at goofed step for retry"
        );
    }

    /// Mod-tap mid-word without an error rolls step_idx back by one
    /// phoneme so the user can retype the previous syllable.
    #[test]
    fn phoneme_mode_mod_undo_rolls_back_one_phoneme() {
        let mut state = TutorState::new(kt_practice());
        // Type phoneme[0] (K) and release: lands on step 2.
        for &(scan, direction) in &[
            (scan::WORD, KeyDirection::Down),
            (scan::R_IDX, KeyDirection::Down),
            (scan::R_MID, KeyDirection::Down),
            (scan::R_IDX, KeyDirection::Up),
            (scan::R_MID, KeyDirection::Up),
        ] {
            state.tick(RheKeyEvent { scan, direction });
        }
        assert_eq!(state.practice.step_idx, 2);
        assert!(!state.errored);

        // Mod-tap should roll back to step 0 (start of phoneme[0]).
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Down,
        });
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Up,
        });
        assert!(!state.errored);
        assert_eq!(state.practice.step_idx, 0, "one phoneme undone");
    }

    /// Mod-tap at step 0 (no phonemes typed) is a no-op — clears any
    /// transient acc state but doesn't underflow step_idx or goof.
    #[test]
    fn phoneme_mode_mod_undo_at_step_zero_no_op() {
        let mut state = TutorState::new(kt_practice());
        state.tick(RheKeyEvent {
            scan: scan::WORD,
            direction: KeyDirection::Down,
        });
        // Mod-tap before typing any phoneme.
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Down,
        });
        state.tick(RheKeyEvent {
            scan: scan::R_THUMB,
            direction: KeyDirection::Up,
        });
        assert!(!state.errored, "step-0 mod tap should not goof");
        assert_eq!(state.practice.step_idx, 0);
    }
}
