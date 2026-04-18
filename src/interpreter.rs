use crate::chord_map::{ChordKey, SyllableTable};
use crate::state_machine::Event;

/// Output actions to send to the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Emit a string (word + trailing space, or instant brief output).
    Emit(String),
    /// Delete the current word buffer (undo).
    Undo,
}

/// Converts state machine events into output actions.
///
/// Word mode (between WordStart and WordEnd): buffer syllables.
/// Brief mode (no active word): emit immediately.
pub struct Interpreter {
    syllables: SyllableTable,
    briefs: SyllableTable,
    word_buffer: String,
    in_word: bool,
}

impl Interpreter {
    pub fn new(syllables: SyllableTable, briefs: SyllableTable) -> Self {
        Self {
            syllables,
            briefs,
            word_buffer: String::new(),
            in_word: false,
        }
    }

    /// Process a state machine event. Returns an action if one should be taken.
    pub fn process(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::WordStart => {
                self.in_word = true;
                self.word_buffer.clear();
                None
            }
            Event::WordEnd => {
                self.in_word = false;
                if self.word_buffer.is_empty() {
                    None
                } else {
                    let mut word = std::mem::take(&mut self.word_buffer);
                    word.push(' ');
                    Some(Action::Emit(word))
                }
            }
            Event::Chord(chord) => {
                let key = ChordKey::from_chord(chord);
                if self.in_word {
                    if let Some(syllable) = self.syllables.lookup(key) {
                        self.word_buffer.push_str(syllable);
                    }
                    None
                } else {
                    self.briefs.lookup(key).map(|s| Action::Emit(s.to_string()))
                }
            }
            Event::Undo => {
                self.word_buffer.clear();
                self.in_word = false;
                Some(Action::Undo)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_map::ChordKey;
    use crate::chord_state::{Chord, FingerChord, Mode, ThumbState};

    fn make_chord(mode: Mode, right: u8, left: u8, ctrl: bool) -> Chord {
        Chord {
            mode,
            right: FingerChord(right),
            left: FingerChord(left),
            thumbs: if ctrl { ThumbState::CTRL } else { ThumbState::NONE },
        }
    }

    fn setup_tables() -> (SyllableTable, SyllableTable) {
        let mut syllables = SyllableTable::new();
        let mut briefs = SyllableTable::new();

        let cat = make_chord(Mode::Mode1, 0b0101, 0b0011, false);
        syllables.insert(ChordKey::from_chord(&cat), "cat".to_string());

        let su = make_chord(Mode::Mode1, 0b0001, 0b0000, false);
        let per = make_chord(Mode::Mode2, 0b0010, 0b0100, false);
        syllables.insert(ChordKey::from_chord(&su), "su".to_string());
        syllables.insert(ChordKey::from_chord(&per), "per".to_string());

        let the = make_chord(Mode::Mode1, 0b1010, 0b0000, false);
        briefs.insert(ChordKey::from_chord(&the), "the ".to_string());

        (syllables, briefs)
    }

    #[test]
    fn single_syllable_word() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let chord = make_chord(Mode::Mode1, 0b0101, 0b0011, false);

        assert!(interp.process(&Event::WordStart).is_none());
        assert!(interp.process(&Event::Chord(chord)).is_none());
        let action = interp.process(&Event::WordEnd).unwrap();
        assert_eq!(action, Action::Emit("cat ".to_string()));
    }

    #[test]
    fn multi_syllable_word() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let su = make_chord(Mode::Mode1, 0b0001, 0b0000, false);
        let per = make_chord(Mode::Mode2, 0b0010, 0b0100, false);

        interp.process(&Event::WordStart);
        interp.process(&Event::Chord(su));
        interp.process(&Event::Chord(per));
        let action = interp.process(&Event::WordEnd).unwrap();
        assert_eq!(action, Action::Emit("super ".to_string()));
    }

    #[test]
    fn brief_no_space() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let the = make_chord(Mode::Mode1, 0b1010, 0b0000, false);
        let action = interp.process(&Event::Chord(the)).unwrap();
        assert_eq!(action, Action::Emit("the ".to_string()));
    }

    #[test]
    fn undo_clears_buffer() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let su = make_chord(Mode::Mode1, 0b0001, 0b0000, false);

        interp.process(&Event::WordStart);
        interp.process(&Event::Chord(su));
        let action = interp.process(&Event::Undo).unwrap();
        assert_eq!(action, Action::Undo);

        // Word end after undo should produce nothing
        assert!(interp.process(&Event::WordEnd).is_none());
    }

    #[test]
    fn empty_word_produces_nothing() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        interp.process(&Event::WordStart);
        assert!(interp.process(&Event::WordEnd).is_none());
    }
}
