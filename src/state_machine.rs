//! Chord detection: accumulates keys, fires on all-zero.
//! Dual mode: per-hand (word held) vs all-zero (rolls).

use crate::chord_map::ChordKey;
use crate::hand::{KeyDirection, KeyEvent};
use crate::key_mask::KeyMask;
use crate::scan;

/// Events emitted by the state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A chord fired. `space_held` is true when the word key was held
    /// while the chord was being accumulated (→ phoneme lookup);
    /// false for a free-standing chord (→ brief lookup).
    /// `first_down` is the scancode that started this accumulation
    /// cycle — used by the brief lookup to resolve ordered entries
    /// (homophone splits etc.). `None` only when the accumulation
    /// somehow fires with an empty record, which shouldn't happen.
    Chord {
        key: ChordKey,
        space_held: bool,
        first_down: Option<u8>,
    },
    /// Word key released — commit buffered word.
    SpaceUp,
    /// Solo word tap (no fingers during tap) = backspace.
    Backspace,
    /// Undo last phoneme (reserved for future gesture).
    UndoPhoneme,
}

// Hand partitions of the chord keyspace live in `scan::LEFT_MASK` and
// `scan::RIGHT_MASK`. Right-thumb counts as right hand — it's the mod
// bit and it participates in the right-hand "has this hand released
// all its keys" check.
use scan::{LEFT_MASK, RIGHT_MASK};

/// State machine for rhe's chord pipeline.
///
/// `live` = keys currently held (chord keys only; word is tracked by
/// `word_held` since it has distinct firing semantics).
/// `accum` = union of every key that went down since the last fire.
/// Firing flushes the relevant bits from `accum`.
///
/// When `word_held` is true, each hand fires independently the moment
/// that hand's `live` portion drops to zero (phoneme-per-hand).
/// When `word_held` is false, both hands accumulate into a single chord
/// that fires only when all chord keys release (rolls / briefs).
#[derive(Debug)]
pub struct StateMachine {
    live: KeyMask,
    accum: KeyMask,
    word_held: bool,
    /// Set whenever any chord key is pressed during a word-held session.
    /// A word key that goes down and up with this still false = solo tap
    /// = backspace gesture.
    fingers_during_word: bool,
    /// Scancode of the first key pressed since the last fire. Used to
    /// disambiguate ordered briefs. Reset when the accumulator clears.
    first_down: Option<u8>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            live: KeyMask::EMPTY,
            accum: KeyMask::EMPTY,
            word_held: false,
            fingers_during_word: false,
            first_down: None,
        }
    }

    pub fn feed(&mut self, event: KeyEvent) -> Vec<Event> {
        if event.scan == scan::WORD {
            self.handle_word(event.direction)
        } else {
            self.handle_chord_key(event.scan, event.direction)
        }
    }

    fn handle_word(&mut self, direction: KeyDirection) -> Vec<Event> {
        match direction {
            KeyDirection::Down => {
                self.word_held = true;
                self.fingers_during_word = !(self.live & BOTH_HANDS).is_empty();
                self.accum = KeyMask::EMPTY;
                self.first_down = None;
                vec![]
            }
            KeyDirection::Up => {
                self.word_held = false;
                if !self.fingers_during_word && (self.live & BOTH_HANDS).is_empty() {
                    vec![Event::Backspace]
                } else {
                    vec![Event::SpaceUp]
                }
            }
        }
    }

    fn handle_chord_key(&mut self, scan: u8, direction: KeyDirection) -> Vec<Event> {
        self.fingers_during_word = true;
        match direction {
            KeyDirection::Down => {
                // Record the first chord key of this accumulation cycle.
                // Cleared by `try_fire` after the chord emits.
                if self.accum.is_empty() {
                    self.first_down = Some(scan);
                }
                self.live.set(scan);
                self.accum.set(scan);
                vec![]
            }
            KeyDirection::Up => {
                self.live.clear(scan);
                self.try_fire(scan)
            }
        }
    }

    /// Fire logic:
    /// - Word held → fire this hand the moment its live portion drops to
    ///   zero, carrying just that hand's accumulated bits.
    /// - Word not held → fire a single combined chord when both hands go
    ///   to zero, carrying everything accumulated since the last fire.
    fn try_fire(&mut self, released_scan: u8) -> Vec<Event> {
        if self.word_held {
            let hand_mask = match scan_hand_mask(released_scan) {
                Some(m) => m,
                None => return vec![],
            };
            if (self.live & hand_mask).is_empty() {
                let hand_accum = self.accum & hand_mask;
                if !hand_accum.is_empty() {
                    self.accum &= !hand_mask;
                    // Per-hand phoneme fire — `first_down` is preserved
                    // for a possible later all-zero brief fire but
                    // passed along here too (phoneme lookup ignores it).
                    let first_down = self.first_down;
                    if self.accum.is_empty() {
                        self.first_down = None;
                    }
                    return vec![Event::Chord {
                        key: ChordKey::from_mask(hand_accum),
                        space_held: true,
                        first_down,
                    }];
                }
            }
            vec![]
        } else {
            if !(self.live & BOTH_HANDS).is_empty() {
                return vec![];
            }
            if self.accum.is_empty() {
                return vec![];
            }
            let key = ChordKey::from_mask(self.accum);
            let first_down = self.first_down;
            self.accum = KeyMask::EMPTY;
            self.first_down = None;
            vec![Event::Chord {
                key,
                space_held: false,
                first_down,
            }]
        }
    }
}

const BOTH_HANDS: KeyMask = KeyMask::from_raw([
    LEFT_MASK.as_raw()[0] | RIGHT_MASK.as_raw()[0],
    LEFT_MASK.as_raw()[1] | RIGHT_MASK.as_raw()[1],
    LEFT_MASK.as_raw()[2] | RIGHT_MASK.as_raw()[2],
    LEFT_MASK.as_raw()[3] | RIGHT_MASK.as_raw()[3],
]);

/// Which hand mask owns this scancode? `None` for anything outside the
/// chord keyspace.
fn scan_hand_mask(scan: u8) -> Option<KeyMask> {
    if LEFT_MASK.test(scan) {
        Some(LEFT_MASK)
    } else if RIGHT_MASK.test(scan) {
        Some(RIGHT_MASK)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(scan: u8, dir: KeyDirection) -> KeyEvent {
        KeyEvent { scan, direction: dir }
    }
    fn word(dir: KeyDirection) -> KeyEvent { ev(scan::WORD, dir) }
    fn l_pinky(dir: KeyDirection) -> KeyEvent { ev(scan::L_PINKY, dir) }
    fn r_idx(dir: KeyDirection) -> KeyEvent { ev(scan::R_IDX, dir) }
    fn r_mid(dir: KeyDirection) -> KeyEvent { ev(scan::R_MID, dir) }
    fn r_thumb(dir: KeyDirection) -> KeyEvent { ev(scan::R_THUMB, dir) }

    fn feed_all(sm: &mut StateMachine, events: &[KeyEvent]) -> Vec<Event> {
        events.iter().flat_map(|e| sm.feed(*e)).collect()
    }

    #[test]
    fn single_right_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[r_idx(KeyDirection::Down), r_idx(KeyDirection::Up)],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.right_bits(), 0b0001);
        assert!(!key.has_mod());
    }

    #[test]
    fn right_thumb_is_mod() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                r_idx(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_idx(KeyDirection::Up),
                r_thumb(KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.right_bits(), 0b0001);
        assert!(key.has_mod()); // thumb = mod bit
    }

    #[test]
    fn single_left_finger() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[l_pinky(KeyDirection::Down), l_pinky(KeyDirection::Up)],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.left_bits(), 0b1000);
        assert!(!key.has_mod());
    }

    #[test]
    fn rolled_hands_fire_as_one_chord() {
        let mut sm = StateMachine::new();
        // Left down, right down, left up, right up — fires ONE combined chord
        let events = feed_all(
            &mut sm,
            &[
                l_pinky(KeyDirection::Down),
                r_idx(KeyDirection::Down),
                l_pinky(KeyDirection::Up),
                r_idx(KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.left_bits(), 0b1000);
        assert_eq!(key.right_bits(), 0b0001);
    }

    #[test]
    fn mod_in_combined_chord() {
        let mut sm = StateMachine::new();
        // Thumb + both hands → one chord with mod
        let events = feed_all(
            &mut sm,
            &[
                r_thumb(KeyDirection::Down),
                l_pinky(KeyDirection::Down),
                r_idx(KeyDirection::Down),
                l_pinky(KeyDirection::Up),
                r_idx(KeyDirection::Up),
                r_thumb(KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.left_bits(), 0b1000);
        assert_eq!(key.right_bits(), 0b0001);
        assert!(key.has_mod());
    }

    #[test]
    fn word_up_commits() {
        let mut sm = StateMachine::new();
        sm.feed(word(KeyDirection::Down));
        feed_all(
            &mut sm,
            &[r_idx(KeyDirection::Down), r_idx(KeyDirection::Up)],
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
    fn backspace_after_word_commit() {
        let mut sm = StateMachine::new();
        // Type a word: word + finger chord + word-up.
        feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_idx(KeyDirection::Down),
                r_idx(KeyDirection::Up),
                word(KeyDirection::Up),
            ],
        );
        // Now a solo word tap — should fire Backspace.
        let events = feed_all(&mut sm, &[word(KeyDirection::Down), word(KeyDirection::Up)]);
        assert!(
            events.contains(&Event::Backspace),
            "expected Backspace after word commit, got {:?}",
            events
        );
    }

    #[test]
    fn multi_finger_accumulates() {
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                r_idx(KeyDirection::Down),
                r_mid(KeyDirection::Down),
                r_idx(KeyDirection::Up),
                r_mid(KeyDirection::Up),
            ],
        );
        assert_eq!(events.len(), 1);
        let Event::Chord { key, .. } = &events[0] else {
            panic!()
        };
        assert_eq!(key.right_bits(), 0b0011);
    }

    #[test]
    fn word_down_resets_accumulators() {
        let mut sm = StateMachine::new();
        sm.feed(word(KeyDirection::Down));
        feed_all(
            &mut sm,
            &[r_idx(KeyDirection::Down), r_idx(KeyDirection::Up)],
        );
        sm.feed(word(KeyDirection::Up));
        sm.feed(word(KeyDirection::Down));
        assert!(sm.accum.is_empty());
    }
}
