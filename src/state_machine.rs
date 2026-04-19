use crate::chord_state::{Chord, FingerChord, Mode, ThumbState};
use crate::hand::{Finger, Hand, KeyDirection, KeyEvent, PhysicalKey, Thumb};

/// Events emitted by the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A chord was completed (all fingers released).
    /// Chord.space_held indicates if this is a syllable (true) or brief (false).
    Chord(Chord),
    /// Space was released — emit buffered word if any.
    SpaceUp,
}

/// Tracks key events and emits chords + word boundary events.
///
/// Space is word-level: press to start a word, release to end it.
/// Ctrl is per-syllable: captured at chord completion time.
/// For finger chords, only two signals matter:
///   - First-down: which hand pressed a finger key first
///   - First-up: which hand released a finger key first
/// Intra-hand ordering is ignored.
#[derive(Debug)]
pub struct StateMachine {
    phase: Phase,
    left: FingerChord,
    right: FingerChord,
    ctrl_held: bool,
    space_held: bool,
    first_down: Option<Hand>,
    first_up: Option<Hand>,
    left_max: FingerChord,
    right_max: FingerChord,
    ctrl_max: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No finger keys pressed.
    Idle,
    /// At least one finger key is down, accumulating.
    Accumulating,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            left: FingerChord::NONE,
            right: FingerChord::NONE,
            ctrl_held: false,
            space_held: false,
            first_down: None,
            first_up: None,
            left_max: FingerChord::NONE,
            right_max: FingerChord::NONE,
            ctrl_max: false,
        }
    }

    /// Feed a key event. Returns any events produced.
    pub fn feed(&mut self, event: KeyEvent) -> Vec<Event> {
        match event.key {
            PhysicalKey::Thumb(thumb) => self.handle_thumb(thumb, event.direction),
            PhysicalKey::Finger(hand, finger) => self.handle_finger(hand, finger, event.direction),
        }
    }

    fn handle_thumb(&mut self, thumb: Thumb, direction: KeyDirection) -> Vec<Event> {
        match (thumb, direction) {
            (Thumb::Ctrl, KeyDirection::Down) => {
                self.ctrl_held = true;
                if self.phase == Phase::Accumulating {
                    self.ctrl_max = true;
                }
                vec![]
            }
            (Thumb::Ctrl, KeyDirection::Up) => {
                self.ctrl_held = false;
                vec![]
            }
            (Thumb::Space, KeyDirection::Down) => {
                self.space_held = true;
                vec![]
            }
            (Thumb::Space, KeyDirection::Up) => {
                self.space_held = false;
                vec![Event::SpaceUp]
            }
        }
    }

    fn handle_finger(&mut self, hand: Hand, finger: Finger, direction: KeyDirection) -> Vec<Event> {
        match direction {
            KeyDirection::Down => {
                self.finger_down(hand, finger);
                vec![]
            }
            KeyDirection::Up => self.finger_up(hand, finger),
        }
    }

    fn finger_down(&mut self, hand: Hand, finger: Finger) {
        if self.phase == Phase::Idle {
            self.phase = Phase::Accumulating;
            self.first_down = None;
            self.first_up = None;
            self.left_max = FingerChord::NONE;
            self.right_max = FingerChord::NONE;
            self.ctrl_max = self.ctrl_held;
        }

        match hand {
            Hand::Left => {
                self.left.set(finger);
                self.left_max.0 |= self.left.0;
            }
            Hand::Right => {
                self.right.set(finger);
                self.right_max.0 |= self.right.0;
            }
        }

        if self.first_down.is_none() {
            self.first_down = Some(hand);
        }
    }

    fn finger_up(&mut self, hand: Hand, finger: Finger) -> Vec<Event> {
        match hand {
            Hand::Left => self.left.clear(finger),
            Hand::Right => self.right.clear(finger),
        }

        if self.phase != Phase::Accumulating {
            return vec![];
        }

        if self.first_up.is_none() {
            self.first_up = Some(hand);
        }

        if self.left.is_empty() && self.right.is_empty() {
            let events = self.resolve();
            self.phase = Phase::Idle;
            events
        } else {
            vec![]
        }
    }

    fn resolve(&self) -> Vec<Event> {
        let Some(first_down) = self.first_down else {
            return vec![];
        };
        let Some(first_up) = self.first_up else {
            return vec![];
        };

        vec![Event::Chord(Chord {
            mode: Mode::from_order(first_down, first_up),
            right: self.right_max,
            left: self.left_max,
            thumbs: if self.ctrl_max {
                ThumbState::CTRL
            } else {
                ThumbState::NONE
            },
            space_held: self.space_held,
        })]
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

    fn thumb(thumb: Thumb, dir: KeyDirection) -> KeyEvent {
        KeyEvent {
            key: PhysicalKey::Thumb(thumb),
            direction: dir,
        }
    }

    fn feed_all(sm: &mut StateMachine, events: &[KeyEvent]) -> Vec<Event> {
        events.iter().flat_map(|e| sm.feed(*e)).collect()
    }

    #[test]
    fn mode1_right_wraps() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Left, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
            finger(Hand::Left, Finger::Index, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!("expected chord") };
        assert_eq!(chord.mode, Mode::Mode1);
        assert!(chord.right.is_pressed(Finger::Index));
        assert!(chord.left.is_pressed(Finger::Index));
    }

    #[test]
    fn mode2() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Left, Finger::Middle, KeyDirection::Down),
            finger(Hand::Left, Finger::Middle, KeyDirection::Up),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!("expected chord") };
        assert_eq!(chord.mode, Mode::Mode2);
    }

    #[test]
    fn mode3() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Ring, KeyDirection::Down),
            finger(Hand::Right, Finger::Pinky, KeyDirection::Down),
            finger(Hand::Right, Finger::Pinky, KeyDirection::Up),
            finger(Hand::Left, Finger::Ring, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!("expected chord") };
        assert_eq!(chord.mode, Mode::Mode3);
    }

    #[test]
    fn mode4() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Left, Finger::Index, KeyDirection::Up),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!("expected chord") };
        assert_eq!(chord.mode, Mode::Mode4);
    }

    #[test]
    fn multi_finger_chord() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Middle, KeyDirection::Down),
            finger(Hand::Left, Finger::Ring, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
            finger(Hand::Right, Finger::Middle, KeyDirection::Up),
            finger(Hand::Left, Finger::Ring, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!("expected chord") };
        assert!(chord.right.is_pressed(Finger::Index));
        assert!(chord.right.is_pressed(Finger::Middle));
        assert_eq!(chord.right.count(), 2);
        assert!(chord.left.is_pressed(Finger::Ring));
        assert_eq!(chord.left.count(), 1);
    }

    #[test]
    fn ctrl_captured_per_syllable() {
        let mut sm = StateMachine::new();

        // First chord: no ctrl
        let events1 = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        let Event::Chord(c1) = &events1[0] else { panic!() };
        assert!(!c1.thumbs.has_ctrl());

        // Press ctrl
        sm.feed(thumb(Thumb::Ctrl, KeyDirection::Down));

        // Second chord: ctrl held
        let events2 = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        let Event::Chord(c2) = &events2[0] else { panic!() };
        assert!(c2.thumbs.has_ctrl());
    }

    #[test]
    fn space_held_on_chord() {
        let mut sm = StateMachine::new();

        // Space down
        sm.feed(thumb(Thumb::Space, KeyDirection::Down));

        // Chord while space held
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert!(chord.space_held);

        // Space up
        let events = feed_all(&mut sm, &[
            thumb(Thumb::Space, KeyDirection::Up),
        ]);
        assert!(events.contains(&Event::SpaceUp));
    }

    #[test]
    fn no_space_on_chord() {
        let mut sm = StateMachine::new();

        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert!(!chord.space_held);
    }

    #[test]
    fn sloppy_intra_hand_release() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Middle, KeyDirection::Down),
            finger(Hand::Left, Finger::Ring, KeyDirection::Down),
            // Right middle releases first — first_up = Right
            finger(Hand::Right, Finger::Middle, KeyDirection::Up),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
            finger(Hand::Left, Finger::Ring, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert_eq!(chord.mode, Mode::Mode1);
        assert!(chord.right.is_pressed(Finger::Index));
        assert!(chord.right.is_pressed(Finger::Middle));
    }

    #[test]
    fn onset_only_no_coda() {
        let mut sm = StateMachine::new();
        let events = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Ring, KeyDirection::Down),
            finger(Hand::Right, Finger::Pinky, KeyDirection::Down),
            finger(Hand::Right, Finger::Ring, KeyDirection::Up),
            finger(Hand::Right, Finger::Pinky, KeyDirection::Up),
        ]);

        assert_eq!(events.len(), 1);
        let Event::Chord(chord) = &events[0] else { panic!() };
        assert_eq!(chord.mode, Mode::Mode1);
        assert!(chord.left.is_empty());
    }

    #[test]
    fn two_sequential_chords() {
        let mut sm = StateMachine::new();

        // First chord
        let e1 = feed_all(&mut sm, &[
            finger(Hand::Right, Finger::Index, KeyDirection::Down),
            finger(Hand::Right, Finger::Index, KeyDirection::Up),
        ]);
        assert_eq!(e1.len(), 1);

        // Second chord
        let e2 = feed_all(&mut sm, &[
            finger(Hand::Left, Finger::Middle, KeyDirection::Down),
            finger(Hand::Left, Finger::Middle, KeyDirection::Up),
        ]);
        assert_eq!(e2.len(), 1);

        // Both should be chords
        assert!(matches!(e1[0], Event::Chord(_)));
        assert!(matches!(e2[0], Event::Chord(_)));
    }
}
