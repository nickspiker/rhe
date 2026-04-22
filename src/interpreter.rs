//! Converts state machine events into output actions (emit text, backspace, suffix).

use crate::chord_map::{BriefTable, ChordKey, Phoneme, PhonemeTable};
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
        }
    }

    pub fn process(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::Chord(chord) => {
                let key = ChordKey::from_chord(chord);
                if chord.space_held {
                    if let Some(phoneme) = self.phonemes.lookup(key) {
                        self.buffer.push(phoneme);
                    }
                    None
                } else {
                    self.briefs.lookup(key).map(|s| {
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
            Event::SpaceUp => {
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
    use crate::chord_state::{Chord, FingerChord};

    fn make_chord(right: u8, left: u8, modkey: bool, space: bool) -> Chord {
        Chord {
            right: FingerChord(right),
            left: FingerChord(left),
            modkey,
            space_held: space,
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
        interp.process(&Event::Chord(make_chord(0b0011, 0, false, true)));
        // æ = left index+middle (0011), space held
        interp.process(&Event::Chord(make_chord(0, 0b0011, false, true)));
        // t = right index (0001), space held
        interp.process(&Event::Chord(make_chord(0b0001, 0, false, true)));

        let action = interp.process(&Event::SpaceUp).unwrap();
        assert_eq!(action, Action::Emit("cat ".to_string()));
    }

    #[test]
    fn brief_mode() {
        let mut interp = setup();

        // Both-hands chord without space = brief
        let action = interp.process(&Event::Chord(make_chord(0b0001, 0b0001, false, false)));
        assert_eq!(action, Some(Action::Emit("the ".to_string())));
    }

    #[test]
    fn empty_space_tap() {
        let mut interp = setup();
        assert!(interp.process(&Event::SpaceUp).is_none());
    }
}
