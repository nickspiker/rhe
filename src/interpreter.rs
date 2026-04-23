//! Converts state machine events into output actions (emit text, backspace, suffix).

use crate::chord_map::{BriefTable, Phoneme, PhonemeTable};
use crate::state_machine::Event;
use crate::table_gen::PhonemeDictionary;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// What to emit when a phoneme buffer doesn't resolve to a dictionary word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackMode {
    /// Approximate English grapheme spelling (e.g. "muhlee"). Always ASCII,
    /// always representable in any keyboard layout. Default.
    Autospell,
    /// Raw IPA characters (e.g. "mɛliː"). Accurate phonetically but
    /// requires unicode input or a compatible keymap on the output side.
    Ipa,
}

impl FallbackMode {
    /// Resolve from the `RHE_FALLBACK` env var. Defaults to `Autospell`.
    /// Values: "ipa" or "phonetic" → Ipa; anything else → Autospell.
    pub fn from_env() -> Self {
        match std::env::var("RHE_FALLBACK").as_deref() {
            Ok("ipa") | Ok("phonetic") | Ok("IPA") => Self::Ipa,
            _ => Self::Autospell,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Autospell => 0,
            Self::Ipa => 1,
        }
    }

    pub fn from_u8(raw: u8) -> Self {
        match raw {
            1 => Self::Ipa,
            _ => Self::Autospell,
        }
    }

    /// Shared handle seeded from `RHE_FALLBACK`. The tray and interpreter
    /// both hold a clone of this `Arc` so the menu can flip the mode at
    /// runtime without restarting the engine.
    pub fn new_shared_from_env() -> Arc<AtomicU8> {
        Arc::new(AtomicU8::new(Self::from_env().as_u8()))
    }
}

/// Output actions to send to the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Emit a string (word + trailing space, or instant brief output).
    Emit(String),
    /// Delete N characters (undo last emitted word).
    Backspace(usize),
    /// Suffix: backspace 1 (trailing space), then emit suffix + space.
    Suffix(String),
}

/// Which sub-session of word-held the user is currently in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Default: Chord events look up phonemes (word held) or briefs
    /// (word not held). Word release commits the phoneme buffer.
    Normal,
    /// Entered via a mod-tap during a word-held session. Chord events
    /// emit digits or symbols (if mod is also in the chord). Another
    /// mod-tap emits a decimal point. Word release emits a trailing
    /// space and returns to Normal.
    Number,
}

/// Converts state machine events into output actions.
///
/// Chord with space_held=true: look up phoneme, buffer it.
/// Chord with space_held=false: look up brief, emit immediately.
/// SpaceUp: look up buffered phonemes in dictionary, emit word.
pub struct Interpreter {
    phonemes: PhonemeTable,
    briefs: BriefTable,
    dictionary: PhonemeDictionary,
    buffer: Vec<Phoneme>,
    emit_history: Vec<usize>, // stack of emitted char counts for multi-backspace
    fallback: Arc<AtomicU8>,
    mode: Mode,
}

impl Interpreter {
    /// Seed the fallback from the `RHE_FALLBACK` env var. The returned
    /// interpreter owns its own atomic — no runtime switching from outside.
    pub fn new(phonemes: PhonemeTable, briefs: BriefTable, dictionary: PhonemeDictionary) -> Self {
        Self::with_fallback(phonemes, briefs, dictionary, FallbackMode::new_shared_from_env())
    }

    pub fn with_fallback(
        phonemes: PhonemeTable,
        briefs: BriefTable,
        dictionary: PhonemeDictionary,
        fallback: Arc<AtomicU8>,
    ) -> Self {
        Self {
            phonemes,
            briefs,
            dictionary,
            buffer: Vec::new(),
            emit_history: Vec::new(),
            fallback,
            mode: Mode::Normal,
        }
    }

    /// Currently inside a word-held number sub-session? The tutor uses
    /// this to swap keyboard labels (digits/symbols vs phonemes/briefs)
    /// so the drill shows what the next press actually emits.
    pub fn in_number_mode(&self) -> bool {
        self.mode == Mode::Number
    }

    pub fn process(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::Chord { key, space_held, first_down } => {
                if self.mode == Mode::Number {
                    // Number mode dispatch:
                    //   no mod → digit ("5")
                    //   mod, first_down = thumb → symbol ("+")
                    //   mod, first_down = finger → spelled word ("five")
                    // The gesture-order split gives three distinct
                    // outputs from the same ten-finger layout without
                    // forcing the user to leave number mode for prose.
                    let emitted = if !key.has_mod() {
                        crate::number_data::chord_to_digit(*key)
                            .map(|c| c.to_string())
                    } else if *first_down == Some(crate::scan::R_THUMB) {
                        crate::number_data::chord_to_symbol(*key)
                            .map(|c| c.to_string())
                    } else {
                        crate::number_data::chord_to_digit_word(*key)
                            .map(|s| s.to_string())
                    };
                    return emitted.map(|s| {
                        let n = s.chars().count();
                        self.emit_history.push(n);
                        Action::Emit(s)
                    });
                }
                if *space_held {
                    if let Some(phoneme) = self.phonemes.lookup(*key) {
                        self.buffer.push(phoneme);
                    }
                    None
                } else {
                    self.briefs.lookup(*key, *first_down).map(|s| {
                        if s.starts_with('\x01') {
                            // Suffix: backspace trailing space, then emit suffix
                            let suffix = &s[1..];
                            self.emit_history.push(suffix.chars().count());
                            Action::Suffix(suffix.to_string())
                        } else {
                            let text = s.to_string();
                            self.emit_history.push(text.chars().count());
                            Action::Emit(text)
                        }
                    })
                }
            }
            Event::ModTap => {
                // First mod-tap during a word-held session switches
                // us into Number mode; subsequent mod-taps (same
                // session) emit a decimal point. Either way there
                // should be no pending phoneme buffer — clearing it
                // is defensive.
                match self.mode {
                    Mode::Normal => {
                        self.mode = Mode::Number;
                        self.buffer.clear();
                        eprintln!("rhe: number mode ON");
                        None
                    }
                    Mode::Number => {
                        self.emit_history.push(1);
                        Some(Action::Emit(".".to_string()))
                    }
                }
            }
            Event::SpaceUp => {
                if self.mode == Mode::Number {
                    // Exit number mode with a trailing space. The
                    // emit_history push keeps Backspace symmetrical —
                    // one tap removes the space, next tap the last
                    // digit/symbol, and so on.
                    self.mode = Mode::Normal;
                    self.emit_history.push(1);
                    eprintln!("rhe: number mode OFF");
                    return Some(Action::Emit(" ".to_string()));
                }
                if self.buffer.is_empty() {
                    None
                } else {
                    let phonemes = std::mem::take(&mut self.buffer);
                    let mode = FallbackMode::from_u8(self.fallback.load(Ordering::Relaxed));
                    let text = if mode == FallbackMode::Ipa {
                        // IPA mode: always output IPA, skip dictionary
                        let ipa: String = phonemes.iter().map(|p| p.to_ipa()).collect();
                        format!("{} ", ipa)
                    } else if let Some(word) = self.dictionary.lookup(&phonemes) {
                        format!("{} ", word)
                    } else {
                        // Autospell fallback for unknown words
                        let fallback: String = match mode {
                            FallbackMode::Autospell => {
                                phonemes.iter().map(|p| p.to_grapheme()).collect()
                            }
                            FallbackMode::Ipa => unreachable!(), // handled above
                        };
                        format!("{} ", fallback)
                    };
                    self.emit_history.push(text.chars().count());
                    Some(Action::Emit(text))
                }
            }
            Event::Backspace => {
                if let Some(n) = self.emit_history.pop() {
                    Some(Action::Backspace(n))
                } else {
                    Some(Action::Backspace(1))
                }
            }
            Event::UndoPhoneme => {
                self.buffer.pop();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_map::ChordKey;

    fn chord_event(right: u8, left: u8, modkey: bool, space: bool) -> Event {
        Event::Chord {
            key: ChordKey::from_packed(right, left, modkey),
            space_held: space,
            first_down: None,
        }
    }

    fn setup() -> Interpreter {
        let phonemes = PhonemeTable::new();
        let mut briefs = BriefTable::new();

        // "the" as a brief on both-hands combo
        let the_key = ChordKey::from_packed_u16(0b0001_0001); // right index + left index
        briefs.insert(the_key, "the ".to_string());

        let dict = PhonemeDictionary::build("CAT  K AE1 T\nTHE  DH AH0\n", "the 1000\ncat 500\n");

        Interpreter::new(phonemes, briefs, dict)
    }

    #[test]
    fn phoneme_mode_cat() {
        let mut interp = setup();

        // k = right index+middle (0011), space held
        interp.process(&chord_event(0b0011, 0, false, true));
        // æ = left index+middle (0011), space held
        interp.process(&chord_event(0, 0b0011, false, true));
        // t = right index (0001), space held
        interp.process(&chord_event(0b0001, 0, false, true));

        let action = interp.process(&Event::SpaceUp).unwrap();
        assert_eq!(action, Action::Emit("cat ".to_string()));
    }

    #[test]
    fn brief_mode() {
        let mut interp = setup();

        // Both-hands chord without space = brief
        let action = interp.process(&chord_event(0b0001, 0b0001, false, false));
        assert_eq!(action, Some(Action::Emit("the ".to_string())));
    }

    #[test]
    fn empty_space_tap() {
        let mut interp = setup();
        assert!(interp.process(&Event::SpaceUp).is_none());
    }

    #[test]
    fn number_mode_entry_and_digit() {
        let mut interp = setup();
        // First ModTap: enter number mode, no emit.
        assert!(interp.process(&Event::ModTap).is_none());
        assert_eq!(interp.mode, Mode::Number);
        // Chord with R_IDX alone → digit "3" (position 3).
        use crate::key_mask::KeyMask;
        let key = ChordKey::from_mask(KeyMask::EMPTY.with(crate::scan::R_IDX));
        let event = Event::Chord { key, space_held: true, first_down: None };
        let action = interp.process(&event).unwrap();
        assert_eq!(action, Action::Emit("3".to_string()));
    }

    #[test]
    fn number_mode_decimal_on_second_mod_tap() {
        let mut interp = setup();
        interp.process(&Event::ModTap); // enter
        let action = interp.process(&Event::ModTap).unwrap();
        assert_eq!(action, Action::Emit(".".to_string()));
    }

    #[test]
    fn number_mode_symbol_with_mod() {
        let mut interp = setup();
        interp.process(&Event::ModTap); // enter
        // R_IDX + thumb, first_down = thumb → "+"
        use crate::key_mask::KeyMask;
        let key = ChordKey::from_mask(
            KeyMask::EMPTY.with(crate::scan::R_IDX).with(crate::scan::R_THUMB),
        );
        let event = Event::Chord {
            key,
            space_held: true,
            first_down: Some(crate::scan::R_THUMB),
        };
        let action = interp.process(&event).unwrap();
        assert_eq!(action, Action::Emit("+".to_string()));
    }

    #[test]
    fn number_mode_finger_first_emits_spelled_word() {
        let mut interp = setup();
        interp.process(&Event::ModTap); // enter
        // R_IDX + thumb, first_down = R_IDX → "three"
        use crate::key_mask::KeyMask;
        let key = ChordKey::from_mask(
            KeyMask::EMPTY.with(crate::scan::R_IDX).with(crate::scan::R_THUMB),
        );
        let event = Event::Chord {
            key,
            space_held: true,
            first_down: Some(crate::scan::R_IDX),
        };
        let action = interp.process(&event).unwrap();
        assert_eq!(action, Action::Emit("three".to_string()));
    }

    #[test]
    fn number_mode_spaceup_emits_space_and_exits() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        let action = interp.process(&Event::SpaceUp).unwrap();
        assert_eq!(action, Action::Emit(" ".to_string()));
        assert_eq!(interp.mode, Mode::Normal);
    }

    #[test]
    fn number_mode_backspace_pops_digit() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        use crate::key_mask::KeyMask;
        let key = ChordKey::from_mask(KeyMask::EMPTY.with(crate::scan::R_PINKY));
        let event = Event::Chord { key, space_held: true, first_down: None };
        interp.process(&event).unwrap(); // emits "0"
        let action = interp.process(&Event::Backspace).unwrap();
        assert_eq!(action, Action::Backspace(1));
    }

    #[test]
    fn number_mode_multi_finger_ignored() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        // Two fingers together don't map to a digit.
        use crate::key_mask::KeyMask;
        let key = ChordKey::from_mask(
            KeyMask::EMPTY.with(crate::scan::R_PINKY).with(crate::scan::R_RING),
        );
        let event = Event::Chord { key, space_held: true, first_down: None };
        assert!(interp.process(&event).is_none());
    }
}
