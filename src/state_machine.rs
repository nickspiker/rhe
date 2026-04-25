//! Chord detection: accumulates keys, fires on all-zero.
//! Dual mode: per-hand (word held) vs all-zero (rolls).

use crate::preferences::chord_map::ChordKey;
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
    /// Mod (right thumb) tapped cleanly during a word-held session —
    /// pressed and released with no chord fingers held alongside it.
    /// Fires on thumb-release, not on word-release: word is a
    /// sub-session and the user may mod-tap multiple times inside it.
    /// First tap enters number mode; subsequent taps emit a decimal
    /// point. The interpreter makes that distinction based on its
    /// mode state — the state machine only flags the gesture.
    ModTap,
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
    /// Any chord-bearing activity during the current word-held session:
    /// a finger press, a mod-tap fire, or a pre-held finger at the
    /// moment word went down. On word-release, activity = SpaceUp
    /// (commit whatever is pending); no activity + empty live =
    /// Backspace (the solo-word-tap gesture).
    activity_during_word: bool,
    /// Thumb is held alone right now (no non-thumb fingers alongside it)
    /// during a word-held session. Set on thumb-down from a clean
    /// state, or seeded at word-down if thumb was pre-held alone.
    /// Cleared the moment any non-thumb finger goes down — that finger
    /// pins the gesture to the phoneme path. On thumb-release while
    /// still eligible, we emit `ModTap` and skip the chord fire.
    mod_tap_eligible: bool,
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
            activity_during_word: false,
            mod_tap_eligible: false,
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
                let thumb_mask = KeyMask::EMPTY.with(scan::R_THUMB);
                let non_thumb = self.live & BOTH_HANDS & !thumb_mask;
                // A pre-held finger counts as activity: word-up will
                // commit via SpaceUp even if nothing else happens in
                // between. A pre-held thumb *alone* doesn't count —
                // it just arms the mod-tap detector.
                self.activity_during_word = !non_thumb.is_empty();
                self.mod_tap_eligible =
                    self.live.test(scan::R_THUMB) && non_thumb.is_empty();
                self.accum = KeyMask::EMPTY;
                self.first_down = None;
                vec![]
            }
            KeyDirection::Up => {
                self.word_held = false;
                let activity = self.activity_during_word;
                // User released word while still holding thumb cleanly
                // — the "press both, release both" gesture. Catch it
                // here so a mod-tap fires regardless of release order
                // (otherwise word-first-release silently eats the tap).
                let mod_tap_on_word_up = self.mod_tap_eligible;
                self.activity_during_word = false;
                self.mod_tap_eligible = false;

                let mut events = Vec::new();
                if mod_tap_on_word_up {
                    self.accum.clear(scan::R_THUMB);
                    if self.accum.is_empty() {
                        self.first_down = None;
                    }
                    events.push(Event::ModTap);
                    events.push(Event::SpaceUp);
                } else if activity {
                    events.push(Event::SpaceUp);
                } else if (self.live & BOTH_HANDS).is_empty() {
                    events.push(Event::Backspace);
                }
                events
            }
        }
    }

    fn handle_chord_key(&mut self, scan: u8, direction: KeyDirection) -> Vec<Event> {
        match direction {
            KeyDirection::Down => {
                // Word-held bookkeeping: track mod-tap eligibility
                // (thumb alone = candidate tap) and activity (finger
                // pressed = word commits on release).
                if self.word_held {
                    if scan == crate::scan::R_THUMB {
                        let thumb_mask = KeyMask::EMPTY.with(crate::scan::R_THUMB);
                        let non_thumb_live = self.live & BOTH_HANDS & !thumb_mask;
                        if non_thumb_live.is_empty() {
                            self.mod_tap_eligible = true;
                        }
                    } else {
                        self.mod_tap_eligible = false;
                        self.activity_during_word = true;
                    }
                }
                if self.accum.is_empty() {
                    self.first_down = Some(scan);
                }
                self.live.set(scan);
                self.accum.set(scan);
                vec![]
            }
            KeyDirection::Up => {
                self.live.clear(scan);
                // Clean mod-tap: thumb released during word-held with
                // no finger ever joining it. Fire ModTap immediately
                // and skip try_fire — the user may tap mod again
                // inside this same word-held session (for a decimal).
                if self.word_held
                    && scan == crate::scan::R_THUMB
                    && self.mod_tap_eligible
                {
                    self.mod_tap_eligible = false;
                    self.activity_during_word = true;
                    self.accum.clear(crate::scan::R_THUMB);
                    if self.accum.is_empty() {
                        self.first_down = None;
                    }
                    return vec![Event::ModTap];
                }
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
                    // Thumb-only residue: the user pressed thumb
                    // alone during word-held and released it. No
                    // phoneme is mapped to mod-only, and firing the
                    // chord would only produce noise. Clear the bits
                    // silently — the `ModTap` event will be emitted
                    // later on word-up if this gesture sits alone.
                    let thumb_only = hand_accum.count_ones() == 1
                        && hand_accum.test(scan::R_THUMB);
                    if thumb_only {
                        self.accum &= !hand_mask;
                        if self.accum.is_empty() {
                            self.first_down = None;
                        }
                        return vec![];
                    }
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

    #[test]
    fn word_thumb_tap_fires_mod_tap() {
        // word-down, thumb-down, thumb-up, word-up → ModTap fires
        // immediately on thumb-up (number mode is a sub-session — the
        // user may tap mod again for a decimal before releasing word).
        // A mod-tap counts as activity, so word-up emits SpaceUp (which
        // the interpreter uses to exit number mode with a trailing
        // space).
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_thumb(KeyDirection::Up),
                word(KeyDirection::Up),
            ],
        );
        assert_eq!(events, vec![Event::ModTap, Event::SpaceUp]);
    }

    #[test]
    fn word_up_before_thumb_up_still_fires_mod_tap() {
        // Natural "press both, release both" gesture where the user
        // happens to release word before thumb. Without catching this
        // on word-up, the mod-tap gets silently eaten (thumb-up later
        // sees word_held=false and the eligibility path is skipped).
        // Expect the same ModTap + SpaceUp that the canonical
        // thumb-first-release order produces.
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                word(KeyDirection::Up),
                r_thumb(KeyDirection::Up),
            ],
        );
        assert_eq!(events, vec![Event::ModTap, Event::SpaceUp]);
    }

    #[test]
    fn word_two_mod_taps_in_one_session() {
        // word-held stays down across two thumb taps: the first enters
        // number mode (interpreter side), the second emits a decimal.
        // The state machine just needs to produce two ModTap events,
        // followed by SpaceUp on word release.
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_thumb(KeyDirection::Up),
                r_thumb(KeyDirection::Down),
                r_thumb(KeyDirection::Up),
                word(KeyDirection::Up),
            ],
        );
        assert_eq!(
            events,
            vec![Event::ModTap, Event::ModTap, Event::SpaceUp]
        );
    }

    #[test]
    fn word_finger_and_thumb_is_spaceup_not_mod_tap() {
        // Fingers take priority — any non-thumb finger press during
        // word-held commits via SpaceUp, regardless of thumb taps.
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_idx(KeyDirection::Down),
                r_idx(KeyDirection::Up),
                r_thumb(KeyDirection::Up),
                word(KeyDirection::Up),
            ],
        );
        // Exactly one Chord (thumb+idx fires as voiced consonant) and
        // one SpaceUp. No ModTap.
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Event::Chord { .. }));
        assert_eq!(events[1], Event::SpaceUp);
    }

    #[test]
    fn thumb_only_chord_is_suppressed() {
        // Thumb-alone during word-held must not fire a chord (no mod-
        // only phoneme exists). Instead the gesture fires exactly one
        // ModTap on thumb-up and leaves accum clear so the next
        // gesture starts from a clean slate.
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_thumb(KeyDirection::Up),
            ],
        );
        assert_eq!(events, vec![Event::ModTap]);
        assert!(sm.accum.is_empty());
    }

    #[test]
    fn mod_tap_then_phoneme_fires_cleanly() {
        // After a clean mod-tap inside word-held, the thumb bit must
        // be cleared from accum — otherwise the next per-hand phoneme
        // fire would spuriously carry the mod bit.
        let mut sm = StateMachine::new();
        let events = feed_all(
            &mut sm,
            &[
                word(KeyDirection::Down),
                r_thumb(KeyDirection::Down),
                r_thumb(KeyDirection::Up),
                r_idx(KeyDirection::Down),
                r_idx(KeyDirection::Up),
                word(KeyDirection::Up),
            ],
        );
        // Expect: ModTap, Chord{r_idx, no mod}, SpaceUp.
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], Event::ModTap);
        let Event::Chord { key, .. } = &events[1] else {
            panic!("expected Chord, got {:?}", events[1]);
        };
        assert_eq!(key.right_bits(), 0b0001);
        assert!(!key.has_mod(), "thumb bit leaked into chord after mod-tap");
        assert_eq!(events[2], Event::SpaceUp);
    }
}
