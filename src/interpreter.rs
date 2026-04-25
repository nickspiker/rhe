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
    /// Delete N characters. Produced by Backspace over a plain emit.
    Backspace(usize),
    /// Backspace the `before` text, emit `after`. Covers both
    /// suffixes (before = trailing space " ", after = suffix text)
    /// and number-form transforms (before = previous emission,
    /// after = new form output). The before field is what makes
    /// undo reversible — Backspace swaps roles to restore.
    Replace { before: String, after: String },
}

/// Map a brief-mode chord (word not held, left-hand only) to a
/// number-form if it matches one of the form slots. Returns `None`
/// for any chord that isn't a form trigger — caller falls through
/// to the regular brief / English-suffix path.
///
/// Slot ranking mirrors the existing SUFFIXES table's bench-measured
/// effort order: fastest single-finger chords go to the most common
/// forms.
///
/// Pub so the tutor's adaptive cell labels can reuse the same
/// chord→form mapping when `MODE_FLAG_HAS_NUMBER` is set.
pub fn chord_to_form(key: crate::chord_map::ChordKey) -> Option<crate::number_forms::Form> {
    use crate::number_forms::Form;
    if key.right_bits() != 0 || key.has_mod() {
        return None;
    }
    match key.left_bits() {
        0b0001 => Some(Form::SpelledCardinal), // L-idx  (fastest alone, 668ms)
        0b0100 => Some(Form::Ordinal),         // L-ring (703ms)
        0b1000 => Some(Form::Multiplier),      // L-pinky (721ms)
        0b0010 => Some(Form::Group),           // L-mid  (739ms)
        0b0110 => Some(Form::Fraction),        // L-mid + L-ring (754ms)
        0b0011 => Some(Form::Prefix),          // L-idx + L-mid  (843ms)
        _ => None,
    }
}

/// Bit positions in the shared mode-flags atomic. The interpreter
/// writes on every mode transition; the tutor reads each frame to
/// pick adaptive cell labels (digits/symbols vs phonemes/briefs vs
/// number-form transforms). Pack future sub-modes into more bits.
pub const MODE_FLAG_NUMBER: u8 = 1 << 0;
/// `last_number` armed: the most recent committed token is still a
/// pure-integer the user can transform with an L-hand chord.
/// Brief-mode L-hand cells re-label to form names while this is set.
pub const MODE_FLAG_HAS_NUMBER: u8 = 1 << 1;

/// Allocate a fresh shared mode-flags atomic. Hand a clone to the
/// interpreter via `with_fallback_and_modes` and another clone to
/// any reader (the tutor window).
pub fn new_shared_mode_flags() -> Arc<AtomicU8> {
    Arc::new(AtomicU8::new(0))
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

/// One undo step. Backspace pops the top of `emit_history` and
/// inverts the corresponding emission: an `Emit` becomes a raw
/// backspace over its text; a `Replace` swaps `before`/`after` so
/// the prior visible state is restored. `prior_number_state`
/// snapshots the number-form context from before the emission so
/// undo can also re-arm or clear it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HistoryEntry {
    Emit {
        text: String,
        prior_number_state: Option<NumberContext>,
    },
    Replace {
        before: String,
        after: String,
        prior_number_state: Option<NumberContext>,
    },
}

/// Tracks the state needed for number-form transforms after a
/// number-mode commit. `digits` feeds form generators; `current` is
/// whatever's currently visible on screen for this number (digits+
/// space initially, then the previous form's output after each
/// transform) and serves as the `before` text of the next Replace.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NumberContext {
    digits: String,
    current: String,
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
    /// Stack of undoable emissions. Each entry carries enough info
    /// to reverse the visible change AND restore prior number-form
    /// context. `Backspace` pops one entry per press.
    emit_history: Vec<HistoryEntry>,
    fallback: Arc<AtomicU8>,
    /// Shared bitfield mirroring `mode` for outside readers (the
    /// tutor). Written on every transition in `process`; readers do
    /// a single relaxed load. See `MODE_FLAG_*`.
    mode_flags: Arc<AtomicU8>,
    mode: Mode,
    /// Digits typed in the current number-mode session. Appended on
    /// each plain-digit emission, cleared on symbol / decimal /
    /// spelled-digit (any non-integer emission invalidates "this was
    /// a pure integer"). At word-release, if non-empty, a
    /// `NumberContext` is armed and the buffer is cleared.
    number_buffer: String,
    /// Active number-form context. Armed on a pure-integer commit
    /// from number mode, consumed by non-form emissions that clear
    /// it. Form-transform chords use `digits` as input and update
    /// `current` on success so successive transforms replace in
    /// place.
    last_number: Option<NumberContext>,
}

impl Interpreter {
    /// Seed the fallback from the `RHE_FALLBACK` env var. The returned
    /// interpreter owns its own atomic — no runtime switching from outside.
    pub fn new(phonemes: PhonemeTable, briefs: BriefTable, dictionary: PhonemeDictionary) -> Self {
        Self::with_fallback_and_modes(
            phonemes,
            briefs,
            dictionary,
            FallbackMode::new_shared_from_env(),
            new_shared_mode_flags(),
        )
    }

    pub fn with_fallback(
        phonemes: PhonemeTable,
        briefs: BriefTable,
        dictionary: PhonemeDictionary,
        fallback: Arc<AtomicU8>,
    ) -> Self {
        Self::with_fallback_and_modes(phonemes, briefs, dictionary, fallback, new_shared_mode_flags())
    }

    /// Build with both shared atomics handed in. The mode_flags arc is
    /// the tutor's window into number/symbol/etc. sub-modes — clones
    /// of it can be given to any number of readers.
    pub fn with_fallback_and_modes(
        phonemes: PhonemeTable,
        briefs: BriefTable,
        dictionary: PhonemeDictionary,
        fallback: Arc<AtomicU8>,
        mode_flags: Arc<AtomicU8>,
    ) -> Self {
        Self {
            phonemes,
            briefs,
            dictionary,
            buffer: Vec::new(),
            emit_history: Vec::new(),
            fallback,
            mode_flags,
            mode: Mode::Normal,
            number_buffer: String::new(),
            last_number: None,
        }
    }

    /// Currently inside a word-held number sub-session? The tutor uses
    /// this to swap keyboard labels (digits/symbols vs phonemes/briefs)
    /// so the drill shows what the next press actually emits.
    pub fn in_number_mode(&self) -> bool {
        self.mode == Mode::Number
    }

    /// Recompute the shared mode-flags bitfield from `mode` +
    /// `last_number` and publish it for outside readers. Called
    /// once per `process()` regardless of the path taken — a single
    /// relaxed store is cheap, simpler than threading a write call
    /// onto every internal mutation site.
    fn write_mode_flags(&self) {
        let mut bits = 0u8;
        if self.mode == Mode::Number {
            bits |= MODE_FLAG_NUMBER;
        }
        if self.last_number.is_some() {
            bits |= MODE_FLAG_HAS_NUMBER;
        }
        self.mode_flags.store(bits, Ordering::Relaxed);
    }

    /// Push a plain Emit entry onto history, snapshotting the
    /// number-form context. Helper so every emission path records
    /// undo info consistently.
    fn record_emit(&mut self, text: String) -> Action {
        let prior_number_state = self.last_number.clone();
        self.emit_history.push(HistoryEntry::Emit {
            text: text.clone(),
            prior_number_state,
        });
        Action::Emit(text)
    }

    /// Push a Replace entry (backspace `before` text, emit `after`).
    fn record_replace(&mut self, before: String, after: String) -> Action {
        let prior_number_state = self.last_number.clone();
        self.emit_history.push(HistoryEntry::Replace {
            before: before.clone(),
            after: after.clone(),
            prior_number_state,
        });
        Action::Replace { before, after }
    }

    pub fn process(&mut self, event: &Event) -> Option<Action> {
        let result = self.process_inner(event);
        // Single publish per event, regardless of path. Covers every
        // mode + last_number transition without sprinkling the body
        // with explicit calls.
        self.write_mode_flags();
        result
    }

    fn process_inner(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::Chord { key, space_held, first_down } => {
                if self.mode == Mode::Number {
                    // Number mode dispatch:
                    //   no mod → digit ("5")
                    //   mod, first_down = thumb → symbol ("+")
                    //   mod, first_down = finger → spelled word ("five")
                    if !key.has_mod() {
                        return crate::number_data::chord_to_digit(*key).map(|c| {
                            self.number_buffer.push(c);
                            self.record_emit(c.to_string())
                        });
                    } else if *first_down == Some(crate::scan::R_THUMB) {
                        self.number_buffer.clear();
                        return crate::number_data::chord_to_symbol(*key)
                            .map(|c| self.record_emit(c.to_string()));
                    } else {
                        self.number_buffer.clear();
                        return crate::number_data::chord_to_digit_word(*key)
                            .map(|s| self.record_emit(s.to_string()));
                    }
                }
                if *space_held {
                    self.last_number = None;
                    if let Some(phoneme) = self.phonemes.lookup(*key) {
                        self.buffer.push(phoneme);
                    }
                    None
                } else {
                    // Form transform check.
                    if let Some(form) = chord_to_form(*key) {
                        if let Some(ctx) = self.last_number.clone() {
                            if let Some(out) = crate::number_forms::apply(form, &ctx.digits) {
                                let after = format!("{} ", out);
                                let before = ctx.current.clone();
                                // Snapshot must happen BEFORE we
                                // update last_number so undo can
                                // restore the pre-transform ctx.
                                let action = self.record_replace(before, after.clone());
                                self.last_number = Some(NumberContext {
                                    digits: ctx.digits,
                                    current: after,
                                });
                                return Some(action);
                            }
                            // Form recognized but this number isn't
                            // in its range — no-op, preserve context.
                            return None;
                        }
                        // No number context — fall through to suffix.
                    }
                    // Any non-form brief-mode chord clears context.
                    self.last_number = None;
                    // Materialize into an owned String up front so
                    // the immutable borrow of self.briefs is dropped
                    // before we call record_* which wants &mut self.
                    let brief = self
                        .briefs
                        .lookup(*key, *first_down)
                        .map(|s| s.to_string());
                    brief.map(|s| {
                        if let Some(suffix) = s.strip_prefix('\x01') {
                            self.record_replace(" ".to_string(), suffix.to_string())
                        } else {
                            self.record_emit(s)
                        }
                    })
                }
            }
            Event::ModTap => {
                match self.mode {
                    Mode::Normal => {
                        self.last_number = None;
                        self.mode = Mode::Number;
                        self.buffer.clear();
                        self.number_buffer.clear();
                        None
                    }
                    Mode::Number => {
                        // Decimal — invalidates pure-integer buffer.
                        self.number_buffer.clear();
                        Some(self.record_emit(".".to_string()))
                    }
                }
            }
            Event::SpaceUp => {
                if self.mode == Mode::Number {
                    // Exit number mode with a trailing space.
                    //
                    // If the session produced only plain digits,
                    // consolidate the per-digit history entries into
                    // a single combined one covering the whole
                    // number + space, and arm the form context so
                    // L-hand chords can transform it.
                    self.mode = Mode::Normal;
                    let buf = std::mem::take(&mut self.number_buffer);
                    if buf.is_empty() {
                        self.last_number = None;
                        return Some(self.record_emit(" ".to_string()));
                    }
                    let digit_count = buf.chars().count();
                    // Pop the per-digit entries (they'll be subsumed
                    // by the single combined entry). Each was pushed
                    // individually during number mode so per-digit
                    // backspace works mid-typing.
                    for _ in 0..digit_count {
                        self.emit_history.pop();
                    }
                    let combined = format!("{} ", buf);
                    // Push combined with prior = None (the number
                    // context didn't exist before this commit — it's
                    // the emission that's arming it).
                    self.emit_history.push(HistoryEntry::Emit {
                        text: combined.clone(),
                        prior_number_state: None,
                    });
                    self.last_number = Some(NumberContext {
                        digits: buf,
                        current: combined,
                    });
                    return Some(Action::Emit(" ".to_string()));
                }
                // Phoneme commit from Normal mode.
                self.last_number = None;
                if self.buffer.is_empty() {
                    return None;
                }
                let phonemes = std::mem::take(&mut self.buffer);
                let mode = FallbackMode::from_u8(self.fallback.load(Ordering::Relaxed));
                let text = if mode == FallbackMode::Ipa {
                    let ipa: String = phonemes.iter().map(|p| p.to_ipa()).collect();
                    format!("{} ", ipa)
                } else if let Some(word) = self.dictionary.lookup(&phonemes) {
                    format!("{} ", word)
                } else {
                    let fallback: String = match mode {
                        FallbackMode::Autospell => {
                            phonemes.iter().map(|p| p.to_grapheme()).collect()
                        }
                        FallbackMode::Ipa => unreachable!(),
                    };
                    format!("{} ", fallback)
                };
                Some(self.record_emit(text))
            }
            Event::Backspace => {
                // Pop one history entry and invert it. A plain Emit
                // becomes a backspace of its text. A Replace becomes
                // a Replace with before/after swapped, restoring the
                // prior visible state. Either way, last_number is
                // set back to whatever it was before the popped
                // emission, so undo restores number-form context.
                match self.emit_history.pop() {
                    Some(HistoryEntry::Emit { text, prior_number_state }) => {
                        self.last_number = prior_number_state;
                        Some(Action::Backspace(text.chars().count()))
                    }
                    Some(HistoryEntry::Replace {
                        before,
                        after,
                        prior_number_state,
                    }) => {
                        self.last_number = prior_number_state;
                        Some(Action::Replace {
                            before: after,
                            after: before,
                        })
                    }
                    None => {
                        self.last_number = None;
                        Some(Action::Backspace(1))
                    }
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

    fn digit_event(scan: u8) -> Event {
        use crate::key_mask::KeyMask;
        Event::Chord {
            key: ChordKey::from_mask(KeyMask::EMPTY.with(scan)),
            space_held: true,
            first_down: Some(scan),
        }
    }

    fn l_ring_brief_event() -> Event {
        use crate::key_mask::KeyMask;
        Event::Chord {
            key: ChordKey::from_mask(KeyMask::EMPTY.with(crate::scan::L_RING)),
            space_held: false,
            first_down: Some(crate::scan::L_RING),
        }
    }

    #[test]
    fn ordinal_transform_on_pinky_after_integer() {
        let mut interp = setup();
        interp.process(&Event::ModTap); // enter number mode
        // Type "3"
        interp.process(&digit_event(crate::scan::R_IDX));
        interp.process(&Event::SpaceUp); // commit → emits " ", arms last_number
        // L-pinky brief chord → should fire ordinal transform.
        let action = interp.process(&l_ring_brief_event()).unwrap();
        assert_eq!(
            action,
            Action::Replace {
                before: "3 ".to_string(),
                after: "third ".to_string()
            }
        );
    }

    #[test]
    fn ordinal_transform_on_multi_digit_number() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX_INNER));
        interp.process(&digit_event(crate::scan::R_MID));
        interp.process(&Event::SpaceUp);
        let action = interp.process(&l_ring_brief_event()).unwrap();
        assert_eq!(
            action,
            Action::Replace {
                before: "42 ".to_string(),
                after: "forty-second ".to_string()
            }
        );
    }

    #[test]
    fn ordinal_transform_falls_back_when_number_too_large() {
        // 1921 is outside v1 ordinal table (>999); L-pinky should
        // fall through to the standard -ing suffix behavior.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        // "1921" — R_RING=1, R_RING=9? actually 9 is L_PINKY, 2=R_MID, 1=R_RING.
        // Using positions: 1=R_RING, 9=L_PINKY, 2=R_MID, 1=R_RING.
        interp.process(&digit_event(crate::scan::R_RING));
        interp.process(&digit_event(crate::scan::L_PINKY));
        interp.process(&digit_event(crate::scan::R_MID));
        interp.process(&digit_event(crate::scan::R_RING));
        interp.process(&Event::SpaceUp);
        // L-pinky with last_number=Some("1921") but ordinal("1921")
        // returns None (v1 only handles 0-999). Should fall through
        // to the English suffix path. Since the brief table in setup()
        // doesn't register L-pinky as a suffix, the result is None.
        let result = interp.process(&l_ring_brief_event());
        // No transform emitted (ordinal failed, brief table empty).
        assert!(result.is_none());
    }

    #[test]
    fn ordinal_not_triggered_after_symbol() {
        // Typing "3+" leaves the buffer invalidated by the symbol;
        // last_number should be None on commit, so L-pinky falls
        // through to regular suffix behavior.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX)); // "3"
        // "+" is R_IDX with mod, first_down=R_THUMB
        use crate::key_mask::KeyMask;
        let plus = Event::Chord {
            key: ChordKey::from_mask(
                KeyMask::EMPTY.with(crate::scan::R_IDX).with(crate::scan::R_THUMB),
            ),
            space_held: true,
            first_down: Some(crate::scan::R_THUMB),
        };
        interp.process(&plus);
        interp.process(&Event::SpaceUp);
        // last_number should be None; L-pinky = no ordinal, no brief
        // (empty setup table) → None.
        assert!(interp.process(&l_ring_brief_event()).is_none());
    }

    fn l_chord_event(left_bits: u8) -> Event {
        use crate::key_mask::KeyMask;
        let mut mask = KeyMask::EMPTY;
        // left_bits encoding: I=0001, M=0010, R=0100, P=1000 (per briefs_data)
        if left_bits & 0b0001 != 0 { mask.set(crate::scan::L_IDX); }
        if left_bits & 0b0010 != 0 { mask.set(crate::scan::L_MID); }
        if left_bits & 0b0100 != 0 { mask.set(crate::scan::L_RING); }
        if left_bits & 0b1000 != 0 { mask.set(crate::scan::L_PINKY); }
        let first_down = if left_bits & 0b1000 != 0 { crate::scan::L_PINKY }
            else if left_bits & 0b0100 != 0 { crate::scan::L_RING }
            else if left_bits & 0b0010 != 0 { crate::scan::L_MID }
            else { crate::scan::L_IDX };
        Event::Chord {
            key: ChordKey::from_mask(mask),
            space_held: false,
            first_down: Some(first_down),
        }
    }

    #[test]
    fn forms_swap_in_place_on_same_number() {
        // Type "3" then cycle through three forms on the same number
        // without retyping. Each Replace backspaces the prior
        // emission and emits the new one.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX)); // "3"
        interp.process(&Event::SpaceUp);
        // Ordinal (L-ring)
        let a1 = interp.process(&l_chord_event(0b0100)).unwrap();
        assert_eq!(
            a1,
            Action::Replace {
                before: "3 ".to_string(),
                after: "third ".to_string()
            }
        );
        let a2 = interp.process(&l_chord_event(0b1000)).unwrap();
        assert_eq!(
            a2,
            Action::Replace {
                before: "third ".to_string(),
                after: "thrice ".to_string()
            }
        );
        let a3 = interp.process(&l_chord_event(0b0010)).unwrap();
        assert_eq!(
            a3,
            Action::Replace {
                before: "thrice ".to_string(),
                after: "triple ".to_string()
            }
        );
        let a4 = interp.process(&l_chord_event(0b0011)).unwrap();
        assert_eq!(
            a4,
            Action::Replace {
                before: "triple ".to_string(),
                after: "tri ".to_string()
            }
        );
    }

    #[test]
    fn backspace_restores_number_context_after_form() {
        // Type "3" → ordinal → backspace should restore "3 " AND
        // re-arm last_number so a different form can be applied.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX)); // "3"
        interp.process(&Event::SpaceUp);
        let a1 = interp.process(&l_chord_event(0b0100)).unwrap(); // ordinal
        assert_eq!(
            a1,
            Action::Replace {
                before: "3 ".to_string(),
                after: "third ".to_string()
            }
        );
        // Undo — should restore "3 " AND re-arm context.
        let undo = interp.process(&Event::Backspace).unwrap();
        assert_eq!(
            undo,
            Action::Replace {
                before: "third ".to_string(),
                after: "3 ".to_string()
            }
        );
        // Now a different form should work on the same number.
        let a2 = interp.process(&l_chord_event(0b1000)).unwrap(); // multiplier
        assert_eq!(
            a2,
            Action::Replace {
                before: "3 ".to_string(),
                after: "thrice ".to_string()
            }
        );
    }

    #[test]
    fn backspace_unwinds_chain_of_forms() {
        // Cycle through three forms, then backspace three times.
        // Each backspace reverses one step.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX)); // "3"
        interp.process(&Event::SpaceUp);
        interp.process(&l_chord_event(0b0100)); // → third
        interp.process(&l_chord_event(0b1000)); // → thrice
        interp.process(&l_chord_event(0b0010)); // → triple

        // Three backspaces, each should unwind one step.
        let u1 = interp.process(&Event::Backspace).unwrap();
        assert_eq!(
            u1,
            Action::Replace {
                before: "triple ".to_string(),
                after: "thrice ".to_string()
            }
        );
        let u2 = interp.process(&Event::Backspace).unwrap();
        assert_eq!(
            u2,
            Action::Replace {
                before: "thrice ".to_string(),
                after: "third ".to_string()
            }
        );
        let u3 = interp.process(&Event::Backspace).unwrap();
        assert_eq!(
            u3,
            Action::Replace {
                before: "third ".to_string(),
                after: "3 ".to_string()
            }
        );
        // One more should pop the commit entry and plain-backspace "3 ".
        let u4 = interp.process(&Event::Backspace).unwrap();
        assert_eq!(u4, Action::Backspace(2));
    }

    #[test]
    fn form_for_larger_number_falls_back() {
        // Type "1921" (outside group's 10-max range). L-mid should
        // fall back to the English -ly suffix.
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_RING));  // 1
        interp.process(&digit_event(crate::scan::L_PINKY)); // 9
        interp.process(&digit_event(crate::scan::R_MID));   // 2
        interp.process(&digit_event(crate::scan::R_RING));  // 1
        interp.process(&Event::SpaceUp);
        // L-mid = group, but group("1921") is None (>10). Falls
        // through to English -ly suffix. The setup() brief table
        // doesn't register -ly, so the result is None.
        let result = interp.process(&l_chord_event(0b0010));
        assert!(result.is_none());
    }

    #[test]
    fn ordinal_context_cleared_by_intervening_word() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        interp.process(&digit_event(crate::scan::R_IDX)); // "3"
        interp.process(&Event::SpaceUp); // last_number = Some("3")
        // Intervening word: word-held + phoneme chord + word-up.
        // (Setup's empty phoneme table means no phoneme buffered, but
        // the SpaceUp in Normal mode still clears last_number.)
        interp.process(&Event::SpaceUp);
        // Now L-pinky should NOT see ordinal context anymore.
        assert!(interp.process(&l_ring_brief_event()).is_none());
    }
}
