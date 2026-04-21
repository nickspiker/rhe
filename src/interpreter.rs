use crate::chord_map::{BriefTable, ChordKey, Phoneme, PhonemeTable};
use crate::state_machine::Event;
use crate::table_gen::PhonemeDictionary;

/// Output actions to send to the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Emit a string (word + trailing space, or instant brief output).
    Emit(String),
    /// Delete N characters (undo last emitted word).
    Backspace(usize),
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
    last_emit_len: usize,
}

impl Interpreter {
    pub fn new(phonemes: PhonemeTable, briefs: BriefTable, dictionary: PhonemeDictionary) -> Self {
        Self {
            phonemes,
            briefs,
            dictionary,
            buffer: Vec::new(),
            last_emit_len: 0,
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
                        let text = s.to_string();
                        self.last_emit_len = text.len();
                        Action::Emit(text)
                    })
                }
            }
            Event::SpaceUp => {
                if self.buffer.is_empty() {
                    None
                } else {
                    let phonemes = std::mem::take(&mut self.buffer);
                    let text = if let Some(word) = self.dictionary.lookup(&phonemes) {
                        format!("{} ", word)
                    } else {
                        let ipa: String = phonemes.iter().map(|p| p.to_ipa()).collect();
                        format!("{} ", ipa)
                    };
                    self.last_emit_len = text.len();
                    Some(Action::Emit(text))
                }
            }
            Event::Backspace => {
                let n = self.last_emit_len;
                self.last_emit_len = 0;
                if n > 0 {
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
        let the_key = ChordKey(0b0001_0001); // right index + left index
        briefs.insert(the_key, "the ".to_string());

        let dict = PhonemeDictionary::build("CAT  K AE1 T\nTHE  DH AH0\n", "the 1000\ncat 500\n");

        Interpreter::new(phonemes, briefs, dict)
    }

    #[test]
    fn phoneme_mode_cat() {
        let mut interp = setup();

        // k = right ring (0100), space held
        interp.process(&Event::Chord(make_chord(0b0100, 0, false, true)));
        // æ = left pinky (1000), space held
        interp.process(&Event::Chord(make_chord(0, 0b1000, false, true)));
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
