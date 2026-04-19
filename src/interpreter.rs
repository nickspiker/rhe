use crate::chord_map::{ChordKey, SyllableTable};
use crate::state_machine::Event;

/// Output actions to send to the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Emit a string (word + trailing space, or instant brief output).
    Emit(String),
}

/// Converts state machine events into output actions.
///
/// Chord with space_held=true: buffer as syllable.
/// Chord with space_held=false: emit immediately as brief.
/// SpaceUp: flush buffer as word + space.
pub struct Interpreter {
    syllables: SyllableTable,
    briefs: SyllableTable,
    word_buffer: String,
}

impl Interpreter {
    pub fn new(syllables: SyllableTable, briefs: SyllableTable) -> Self {
        Self {
            syllables,
            briefs,
            word_buffer: String::new(),
        }
    }

    pub fn process(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::Chord(chord) => {
                let key = ChordKey::from_chord(chord);
                if chord.space_held {
                    // Syllable mode: buffer it
                    if let Some(syllable) = self.syllables.lookup(key) {
                        self.word_buffer.push_str(syllable);
                    }
                    None
                } else {
                    // Brief mode: emit immediately
                    self.briefs.lookup(key).map(|s| Action::Emit(s.to_string()))
                }
            }
            Event::SpaceUp => {
                if self.word_buffer.is_empty() {
                    None // TODO: could be enter/newline
                } else {
                    let mut word = std::mem::take(&mut self.word_buffer);
                    word.push(' ');
                    Some(Action::Emit(word))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord_map::ChordKey;
    use crate::chord_state::{Chord, FingerChord, Mode, ThumbState};

    fn make_chord(mode: Mode, right: u8, left: u8, ctrl: bool, space: bool) -> Chord {
        Chord {
            mode,
            right: FingerChord(right),
            left: FingerChord(left),
            thumbs: if ctrl { ThumbState::CTRL } else { ThumbState::NONE },
            space_held: space,
        }
    }

    fn setup_tables() -> (SyllableTable, SyllableTable) {
        let mut syllables = SyllableTable::new();
        let mut briefs = SyllableTable::new();

        let cat = make_chord(Mode::Mode1, 0b0101, 0b0011, false, false);
        syllables.insert(ChordKey::from_chord(&cat), "cat".to_string());

        let su = make_chord(Mode::Mode1, 0b0001, 0b0000, false, false);
        let per = make_chord(Mode::Mode2, 0b0010, 0b0100, false, false);
        syllables.insert(ChordKey::from_chord(&su), "su".to_string());
        syllables.insert(ChordKey::from_chord(&per), "per".to_string());

        let the = make_chord(Mode::Mode1, 0b1010, 0b0000, false, false);
        briefs.insert(ChordKey::from_chord(&the), "the ".to_string());

        (syllables, briefs)
    }

    #[test]
    fn single_syllable_word_with_space() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        // Chord with space held = syllable buffered
        let chord = make_chord(Mode::Mode1, 0b0101, 0b0011, false, true);
        assert!(interp.process(&Event::Chord(chord)).is_none());

        // Space up = emit
        let action = interp.process(&Event::SpaceUp).unwrap();
        assert_eq!(action, Action::Emit("cat ".to_string()));
    }

    #[test]
    fn multi_syllable_word() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let su = make_chord(Mode::Mode1, 0b0001, 0b0000, false, true);
        let per = make_chord(Mode::Mode2, 0b0010, 0b0100, false, true);

        interp.process(&Event::Chord(su));
        interp.process(&Event::Chord(per));
        let action = interp.process(&Event::SpaceUp).unwrap();
        assert_eq!(action, Action::Emit("super ".to_string()));
    }

    #[test]
    fn brief_no_space() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        let the = make_chord(Mode::Mode1, 0b1010, 0b0000, false, false);
        let action = interp.process(&Event::Chord(the)).unwrap();
        assert_eq!(action, Action::Emit("the ".to_string()));
    }

    #[test]
    fn empty_space_tap() {
        let (syllables, briefs) = setup_tables();
        let mut interp = Interpreter::new(syllables, briefs);

        // Space tap with no chords = nothing (for now)
        assert!(interp.process(&Event::SpaceUp).is_none());
    }
}
