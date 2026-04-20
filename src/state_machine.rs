use crate::chord_state::{Chord, FingerChord};
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent, PhysicalKey, Thumb};

/// Events emitted by the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A single-hand chord fired (hand released). Phoneme lookup.
    Chord(Chord),
    /// Space was released — commit buffered word.
    SpaceUp,
    /// Space tapped alone (no fingers, no mod) = backspace.
    Backspace,
    /// Mod tapped while space held (no fingers) = undo last phoneme from buffer.
    UndoPhoneme,
}

/// Per-hand firing state machine.
///
/// Each hand accumulates independently. When a hand's fingers all release,
/// that hand's chord fires. Mod only applies to right hand.
/// Space up = commit word. Space down = clear buffer for new word.
#[derive(Debug)]
pub struct StateMachine {
    // Live key state
    left: FingerChord,
    right: FingerChord,
    mod_held: bool,
    space_held: bool,

    // Per-hand accumulators (bits touched since last fire)
    left_accum: FingerChord,
    right_accum: FingerChord,

    // Mod accumulated since last right-hand fire
    mod_accum: bool,

    // For detecting solo thumb taps
    fingers_during_space: bool,
    fingers_during_mod: bool,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            left: FingerChord::NONE,
            right: FingerChord::NONE,
            mod_held: false,
            space_held: false,
            left_accum: FingerChord::NONE,
            right_accum: FingerChord::NONE,
            mod_accum: false,
            fingers_during_space: false,
            fingers_during_mod: false,
        }
    }

    pub fn feed(&mut self, event: KeyEvent) -> Vec<Event> {
        match event.key {
            PhysicalKey::Thumb(thumb) => self.handle_thumb(thumb, event.direction),
            PhysicalKey::Finger(hand, finger) => self.handle_finger(hand, finger, event.direction),
        }
    }

    fn handle_thumb(&mut self, thumb: Thumb, direction: KeyDirection) -> Vec<Event> {
        match (thumb, direction) {
            (Thumb::Mod, KeyDirection::Down) => {
                self.mod_held = true;
                self.mod_accum = true;
                self.fingers_during_mod = self.has_any_fingers();
                vec![]
            }
            (Thumb::Mod, KeyDirection::Up) => {
                self.mod_held = false;
                // Mod tapped while space held + no fingers = undo last phoneme
                if !self.fingers_during_mod && !self.has_any_fingers() && self.space_held {
                    vec![Event::UndoPhoneme]
                } else {
                    vec![]
                }
            }
            (Thumb::Space, KeyDirection::Down) => {
                self.space_held = true;
                self.fingers_during_space = self.has_any_fingers();
                // New word — reset accumulators
                self.left_accum = FingerChord::NONE;
                self.right_accum = FingerChord::NONE;
                self.mod_accum = false;
                vec![]
            }
            (Thumb::Space, KeyDirection::Up) => {
                self.space_held = false;
                if !self.fingers_during_space && !self.has_any_fingers() && !self.mod_held {
                    // Solo space tap = backspace
                    vec![Event::Backspace]
                } else {
                    vec![Event::SpaceUp]
                }
            }
        }
    }

    fn handle_finger(&mut self, hand: Hand, finger: Finger, direction: KeyDirection) -> Vec<Event> {
        self.fingers_during_space = true;
        self.fingers_during_mod = true;

        match direction {
            KeyDirection::Down => {
                match hand {
                    Hand::Left => {
                        self.left.set(finger);
                        self.left_accum.0 |= self.left.0;
                    }
                    Hand::Right => {
                        self.right.set(finger);
                        self.right_accum.0 |= self.right.0;
                    }
                }
                vec![]
            }
            KeyDirection::Up => {
                match hand {
                    Hand::Left => self.left.clear(finger),
                    Hand::Right => self.right.clear(finger),
                }
                self.try_fire(hand)
            }
        }
    }

    /// If the given hand is now fully released and had accumulated bits, fire its chord.
    fn try_fire(&mut self, hand: Hand) -> Vec<Event> {
        match hand {
            Hand::Left => {
                if self.left.is_empty() && !self.left_accum.is_empty() {
                    let chord = Chord {
                        left: self.left_accum,
                        right: FingerChord::NONE,
                        modkey: false, // mod never applies to left hand
                        space_held: self.space_held,
                    };
                    self.left_accum = FingerChord::NONE;
                    vec![Event::Chord(chord)]
                } else {
                    vec![]
                }
            }
            Hand::Right => {
                if self.right.is_empty() && !self.right_accum.is_empty() {
                    let chord = Chord {
                        left: FingerChord::NONE,
                        right: self.right_accum,
                        modkey: self.mod_accum,
                        space_held: self.space_held,
                    };
                    self.right_accum = FingerChord::NONE;
                    self.mod_accum = false;
                    vec![Event::Chord(chord)]
                } else {
                    vec![]
                }
            }
        }
    }

    fn has_any_fingers(&self) -> bool {
        !self.left.is_empty() || !self.right.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finger(hand: Hand, finger: Finger, dir: KeyDirection) -> KeyEvent {
        KeyEvent { key: PhysicalKey::Finger(hand, finger), direction: dir }
    }

    fn thumb(thumb: Thumb, dir: KeyDirection) -> KeyEvent {
        KeyEvent { key: PhysicalKey::Thumb(thumb), direction: dir }
    }

    fn feed_all(sm: &mut StateMachine, events: &[KeyEvent]) -> Vec<Event> {
        events.iter().flat_map(|e| sm.feed(*e)).collect()
    }

    #[test]
    fn single_right_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert_eq!(chord.right.0, 0b0001);
        assert_eq!(chord.left.0, 0);
        assert!(!chord.modkey);
    }

    #[test]
    fn single_left_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
            finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert_eq!(chord.left.0, 0b1000);
        assert_eq!(chord.right.0, 0);
    }

    #[test]
    fn interleaved_hands_fire_separately() {
        let mut sm = StateMachine::new();
        // Left down, right down, left up (fires left), right up (fires right)
        let events = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 2);
        let Event::Chord(c1) = &events[0] else { panic!() };
        let Event::Chord(c2) = &events[1] else { panic!() };
        assert_eq!(c1.left.0, 0b1000); // left fired first
        assert_eq!(c2.right.0, 0b0001); // right fired second
    }

    #[test]
    fn mod_applies_to_right_only() {
        let mut sm = StateMachine::new();
        // Hold mod, press both hands, release left (no mod), release right (has mod)
        sm.feed(thumb(Thumb::Mod, KeyDirection::Down));
        let events = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Pinky, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Left, Finger::Pinky, KeyDirection::Up),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 2);
        let Event::Chord(left_chord) = &events[0] else { panic!() };
        let Event::Chord(right_chord) = &events[1] else { panic!() };
        assert!(!left_chord.modkey); // left never gets mod
        assert!(right_chord.modkey); // right gets mod
    }

    #[test]
    fn mod_resets_after_right_fire() {
        let mut sm = StateMachine::new();
        sm.feed(thumb(Thumb::Mod, KeyDirection::Down));
        // First right chord with mod
        feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        sm.feed(thumb(Thumb::Mod, KeyDirection::Up));
        // Second right chord without mod
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Middle, KeyDirection::Down),
            finger(Hand::Right, Finger::Middle, KeyDirection::Up),
        ]);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert!(!chord.modkey); // mod was reset after first fire
    }

    #[test]
    fn space_up_commits() {
        let mut sm = StateMachine::new();
        sm.feed(thumb(Thumb::Space, KeyDirection::Down));
        feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        let events = feed_all(&mut sm, &[thumb(Thumb::Space, KeyDirection::Up)]);
        assert!(events.contains(&Event::SpaceUp));
    }

    #[test]
    fn space_down_resets_accumulators() {
        let mut sm = StateMachine::new();
        // Press space, build a chord, release it
        sm.feed(thumb(Thumb::Space, KeyDirection::Down));
        feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        // Space up (commit), space down (new word)
        sm.feed(thumb(Thumb::Space, KeyDirection::Up));
        sm.feed(thumb(Thumb::Space, KeyDirection::Down));
        // Accumulators should be clear — finger held from before shouldn't carry over
        assert!(sm.right_accum.is_empty());
        assert!(sm.left_accum.is_empty());
        assert!(!sm.mod_accum);
    }

    #[test]
    fn solo_space_tap() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            thumb(Thumb::Space, KeyDirection::Down),
            thumb(Thumb::Space, KeyDirection::Up),
        ]);
        assert!(events.contains(&Event::Backspace));
    }

    #[test]
    fn solo_mod_tap_does_nothing() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            thumb(Thumb::Mod, KeyDirection::Down),
            thumb(Thumb::Mod, KeyDirection::Up),
        ]);
        assert!(events.is_empty());
    }

    #[test]
    fn mod_tap_with_space_undoes_phoneme() {
        let mut sm = StateMachine::new();
        sm.feed(thumb(Thumb::Space, KeyDirection::Down));
        let events = feed_all(&mut sm, &[
            thumb(Thumb::Mod, KeyDirection::Down),
            thumb(Thumb::Mod, KeyDirection::Up),
        ]);
        assert!(events.contains(&Event::UndoPhoneme));
    }

    #[test]
    fn multi_finger_accumulates() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Middle, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
            finger(Hand::Right, Finger::Middle, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert_eq!(chord.right.0, 0b0011); // both accumulated
    }
}
