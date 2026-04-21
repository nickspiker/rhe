use crate::chord_state::{Chord, FingerChord};
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent, PhysicalKey};

/// Events emitted by the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A single-hand chord fired (hand released). Phoneme lookup.
    Chord(Chord),
    /// Word key released — commit buffered word.
    SpaceUp,
    /// Solo word tap (no fingers during tap) = backspace.
    Backspace,
    /// Undo last phoneme (reserved for future gesture).
    UndoPhoneme,
}

/// Per-hand firing state machine.
///
/// Right hand = 5 bits (4 fingers + thumb/spacebar).
/// Left hand = 4 bits (4 fingers).
/// Word key (left ⌘) = word boundary, not part of any chord.
///
/// Each hand accumulates independently. When a hand's keys all release,
/// that hand's chord fires. Word up = commit.
#[derive(Debug)]
pub struct StateMachine {
    // Live key state
    left: FingerChord,
    right: FingerChord, // 4 finger bits
    right_thumb: bool,  // spacebar = 5th bit
    word_held: bool,

    // Per-hand accumulators
    left_accum: FingerChord,
    right_accum: u8, // 5 bits accumulated

    // For detecting solo word tap
    fingers_during_word: bool,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            left: FingerChord::NONE,
            right: FingerChord::NONE,
            right_thumb: false,
            word_held: false,
            left_accum: FingerChord::NONE,
            right_accum: 0,
            fingers_during_word: false,
        }
    }

    pub fn feed(&mut self, event: KeyEvent) -> Vec<Event> {
        match event.key {
            PhysicalKey::Word => self.handle_word(event.direction),
            PhysicalKey::Finger(hand, finger) => self.handle_finger(hand, finger, event.direction),
        }
    }

    fn handle_word(&mut self, direction: KeyDirection) -> Vec<Event> {
        match direction {
            KeyDirection::Down => {
                self.word_held = true;
                self.fingers_during_word = self.has_any_fingers();
                // New word — reset accumulators
                self.left_accum = FingerChord::NONE;
                self.right_accum = 0;
                vec![]
            }
            KeyDirection::Up => {
                self.word_held = false;
                if !self.fingers_during_word && !self.has_any_fingers() {
                    vec![Event::Backspace]
                } else {
                    vec![Event::SpaceUp]
                }
            }
        }
    }

    fn handle_finger(&mut self, hand: Hand, finger: Finger, direction: KeyDirection) -> Vec<Event> {
        self.fingers_during_word = true;

        match direction {
            KeyDirection::Down => {
                match hand {
                    Hand::Left => {
                        self.left.set(finger);
                        self.left_accum.0 |= self.left.0;
                    }
                    Hand::Right => {
                        if finger == Finger::Thumb {
                            self.right_thumb = true;
                            self.right_accum |= 1 << 4;
                        } else {
                            self.right.set(finger);
                            self.right_accum |= self.right.0;
                        }
                    }
                }
                vec![]
            }
            KeyDirection::Up => {
                match hand {
                    Hand::Left => self.left.clear(finger),
                    Hand::Right => {
                        if finger == Finger::Thumb {
                            self.right_thumb = false;
                        } else {
                            self.right.clear(finger);
                        }
                    }
                }
                self.try_fire(hand)
            }
        }
    }

    /// Fire logic depends on mode:
    /// - Word held (phoneme mode): fire per-hand when that hand goes to zero.
    /// - Word not held (brief mode): fire when ALL keys go to zero.
    fn try_fire(&mut self, hand: Hand) -> Vec<Event> {
        if self.word_held {
            // Per-hand firing: each hand fires independently
            match hand {
                Hand::Left => {
                    if self.left.is_empty() && !self.left_accum.is_empty() {
                        let chord = Chord {
                            left: self.left_accum,
                            right: FingerChord::NONE,
                            modkey: false,
                            space_held: true,
                        };
                        self.left_accum = FingerChord::NONE;
                        return vec![Event::Chord(chord)];
                    }
                }
                Hand::Right => {
                    if self.right.is_empty() && !self.right_thumb && self.right_accum != 0 {
                        let has_mod = self.right_accum & (1 << 4) != 0;
                        let fingers = self.right_accum & 0xF;
                        let chord = Chord {
                            left: FingerChord::NONE,
                            right: FingerChord(fingers),
                            modkey: has_mod,
                            space_held: true,
                        };
                        self.right_accum = 0;
                        return vec![Event::Chord(chord)];
                    }
                }
            }
            vec![]
        } else {
            // All-zero firing: both hands accumulate, fire when everything released
            if !self.left.is_empty() || !self.right.is_empty() || self.right_thumb {
                return vec![];
            }

            let has_left = !self.left_accum.is_empty();
            let has_right = self.right_accum != 0;

            if !has_left && !has_right {
                return vec![];
            }

            let has_mod = self.right_accum & (1 << 4) != 0;
            let right_fingers = self.right_accum & 0xF;

            let chord = Chord {
                left: self.left_accum,
                right: FingerChord(right_fingers),
                modkey: has_mod,
                space_held: false,
            };

            self.left_accum = FingerChord::NONE;
            self.right_accum = 0;

            vec![Event::Chord(chord)]
        }
    }

    fn has_any_fingers(&self) -> bool {
        !self.left.is_empty() || !self.right.is_empty() || self.right_thumb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finger(hand: Hand, finger: Finger, dir: KeyDirection) -> KeyEvent {
        KeyEvent {
            key: PhysicalKey::Finger(hand, finger),
            direction: dir,
        }
    }

    fn word(dir: KeyDirection) -> KeyEvent {
        KeyEvent {
            key: PhysicalKey::Word,
            direction: dir,
        }
    }

    fn feed_all(sm: &mut StateMachine, events: &[KeyEvent]) -> Vec<Event> {
        events.iter().flat_map(|e| sm.feed(*e)).collect()
    }

    #[test]
    fn single_right_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.right.0, 0b0001);
        assert!(!chord.modkey);
    }

    #[test]
    fn right_thumb_is_mod() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Right, Finger::Thumb, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
                finger(Hand::Right, Finger::Thumb, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.right.0, 0b0001);
        assert!(chord.modkey); // thumb = mod = 5th bit
    }

    #[test]
    fn single_left_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
                finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.left.0, 0b1000);
        assert!(!chord.modkey);
    }

    #[test]
    fn rolled_hands_fire_as_one_chord() {
        let mut sm = StateMachine::new();
        // Left down, right down, left up, right up — fires ONE combined chord
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.left.0, 0b1000);
        assert_eq!(chord.right.0, 0b0001);
    }

    #[test]
    fn mod_in_combined_chord() {
        let mut sm = StateMachine::new();
        // Thumb + both hands → one chord with mod
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Thumb, KeyDirection::Down),
                finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
                finger(Hand::Right, Finger::Thumb, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.left.0, 0b1000);
        assert_eq!(chord.right.0, 0b0001);
        assert!(chord.modkey);
    }

    #[test]
    fn word_up_commits() {
        let mut sm = StateMachine::new();
        sm.feed(word(KeyDirection::Down));
        feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
            ],
        );
        let events = feed_all(&mut sm, &[word(KeyDirection::Up)]);
        assert!(events.contains(&Event::SpaceUp));
    }

    #[test]
    fn solo_word_tap() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[word(KeyDirection::Down), word(KeyDirection::Up)]);
        assert!(events.contains(&Event::Backspace));
    }

    #[test]
    fn multi_finger_accumulates() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Right, Finger::Middle, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
                finger(Hand::Right, Finger::Middle, KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else {
            panic!()
        };
        assert_eq!(chord.right.0, 0b0011);
    }

    #[test]
    fn word_down_resets_accumulators() {
        let mut sm = StateMachine::new();
        sm.feed(word(KeyDirection::Down));
        feed_all(
            &mut sm,
            &[
                finger(Hand::Right, Finger::Index, KeyDirection::Down),
                finger(Hand::Right, Finger::Index, KeyDirection::Up),
            ],
        );
        sm.feed(word(KeyDirection::Up));
        sm.feed(word(KeyDirection::Down));
        assert_eq!(sm.right_accum, 0);
        assert!(sm.left_accum.is_empty());
    }
}
