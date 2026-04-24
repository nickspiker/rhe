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
    /// Delete N characters (undo last emitted word).
    Backspace(usize),
    /// Suffix: backspace 1 (trailing space), then emit suffix + space.
    Suffix(String),
    /// Backspace N characters, then emit text. Used by number-form
    /// transforms (L-ring after a number commit → replace digits
    /// with ordinal: backspace `"42 "` and emit `"forty-second "`).
    Replace(usize, String),
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
    mode: Mode,
    /// Digits typed in the current number-mode session. Appended on
    /// each plain-digit emission, cleared on symbol / decimal /
    /// spelled-digit (any non-integer emission invalidates "this was
    /// a pure integer"). At word-release, if non-empty, copied to
    /// `last_number` and cleared.
    number_buffer: String,
    /// Set when the most recent commit was a pure-integer cardinal
    /// ("1921", "42", "0"). Cleared by any subsequent emission that
    /// isn't a number-form transform. Enables the L-ring-after-
    /// number gesture to replace the emitted digits with an ordinal
    /// spelling ("forty-second" for "42").
    last_number: Option<String>,
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

    pub fn process(&mut self, event: &Event) -> Option<Action> {
        match event {
            Event::Chord { key, space_held, first_down } => {
                if self.mode == Mode::Number {
                    // Number mode dispatch:
                    //   no mod → digit ("5")
                    //   mod, first_down = thumb → symbol ("+")
                    //   mod, first_down = finger → spelled word ("five")
                    // The gesture-order split gives three distinct
                    // outputs from the same ten-finger layout without
                    // forcing the user to leave number mode for prose.
                    if !key.has_mod() {
                        let d = crate::number_data::chord_to_digit(*key);
                        return d.map(|c| {
                            self.number_buffer.push(c);
                            self.emit_history.push(1);
                            Action::Emit(c.to_string())
                        });
                    } else if *first_down == Some(crate::scan::R_THUMB) {
                        // Symbol — invalidates pure-integer buffer.
                        self.number_buffer.clear();
                        return crate::number_data::chord_to_symbol(*key).map(|c| {
                            self.emit_history.push(1);
                            Action::Emit(c.to_string())
                        });
                    } else {
                        // Spelled digit — invalidates pure-integer buffer.
                        self.number_buffer.clear();
                        return crate::number_data::chord_to_digit_word(*key).map(|s| {
                            let text = s.to_string();
                            self.emit_history.push(text.chars().count());
                            Action::Emit(text)
                        });
                    }
                }
                if *space_held {
                    // Phoneme emission — invalidate number-form context.
                    self.last_number = None;
                    if let Some(phoneme) = self.phonemes.lookup(*key) {
                        self.buffer.push(phoneme);
                    }
                    None
                } else {
                    // Brief-mode chord. First check for a number-form
                    // transform: L-ring alone after a pure-integer
                    // commit fires the ordinal form, replacing the
                    // emitted digits with the spelled ordinal.
                    // (L-ring is the second-fastest single-finger
                    // chord by bench timings; ordinal is the most
                    // common number form after raw cardinals, so the
                    // value×effort ranking puts it here.)
                    let is_l_ring_alone = key.left_bits() == 0b0100
                        && key.right_bits() == 0
                        && !key.has_mod();
                    if is_l_ring_alone {
                        if let Some(num) = self.last_number.take() {
                            if let Some(ord) = crate::number_forms::ordinal(&num) {
                                let text = format!("{} ", ord);
                                let back = num.chars().count() + 1;
                                self.emit_history.push(text.chars().count());
                                return Some(Action::Replace(back, text));
                            }
                            // Ordinal lookup failed (number too large
                            // for v1 table); fall through to English
                            // suffix path below. last_number already
                            // consumed by `.take()`.
                        }
                    }
                    // Any other brief-mode chord clears the number-
                    // form context.
                    self.last_number = None;
                    self.briefs.lookup(*key, *first_down).map(|s| {
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
            Event::ModTap => {
                // First mod-tap during a word-held session switches
                // us into Number mode; subsequent mod-taps (same
                // session) emit a decimal point. Either way there
                // should be no pending phoneme buffer — clearing it
                // is defensive.
                match self.mode {
                    Mode::Normal => {
                        // Entering a new number session invalidates
                        // any prior-number-form context.
                        self.last_number = None;
                        self.mode = Mode::Number;
                        self.buffer.clear();
                        self.number_buffer.clear();
                        None
                    }
                    Mode::Number => {
                        // Decimal point — invalidates pure-integer buffer.
                        self.number_buffer.clear();
                        self.emit_history.push(1);
                        Some(Action::Emit(".".to_string()))
                    }
                }
            }
            Event::SpaceUp => {
                if self.mode == Mode::Number {
                    // Exit number mode with a trailing space. The
                    // emit_history push keeps Backspace symmetrical —
                    // one tap removes the space, next tap the last
                    // digit/symbol, and so on.
                    // If the number-mode session produced only plain
                    // digits (no symbols / spelled / decimal), arm
                    // the number-form context — the next L-pinky
                    // chord will transform these digits into an
                    // ordinal instead of suffixing `-ing`.
                    let buf = std::mem::take(&mut self.number_buffer);
                    self.last_number = if buf.is_empty() { None } else { Some(buf) };
                    self.mode = Mode::Normal;
                    self.emit_history.push(1);
                    return Some(Action::Emit(" ".to_string()));
                }
                self.last_number = None;
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
                self.last_number = None;
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
        assert_eq!(action, Action::Replace(2, "third ".to_string()));
    }

    #[test]
    fn ordinal_transform_on_multi_digit_number() {
        let mut interp = setup();
        interp.process(&Event::ModTap);
        // "42" — R_IDX_INNER is digit 4, R_MID is digit 2.
        interp.process(&digit_event(crate::scan::R_IDX_INNER));
        interp.process(&digit_event(crate::scan::R_MID));
        interp.process(&Event::SpaceUp);
        let action = interp.process(&l_ring_brief_event()).unwrap();
        assert_eq!(action, Action::Replace(3, "forty-second ".to_string()));
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
