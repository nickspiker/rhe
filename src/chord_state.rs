use crate::hand::{Finger, Hand};

/// Bitmask of which fingers are pressed on a single hand.
/// Bit 0 = Index, Bit 1 = Middle, Bit 2 = Ring, Bit 3 = Pinky.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FingerChord(pub u8);

impl FingerChord {
    pub const NONE: Self = Self(0);
    pub const INDEX: Self = Self(1 << 0);
    pub const MIDDLE: Self = Self(1 << 1);
    pub const RING: Self = Self(1 << 2);
    pub const PINKY: Self = Self(1 << 3);

    pub fn set(&mut self, finger: Finger) {
        self.0 |= Self::from_finger(finger).0;
    }

    pub fn clear(&mut self, finger: Finger) {
        self.0 &= !Self::from_finger(finger).0;
    }

    pub fn is_pressed(self, finger: Finger) -> bool {
        self.0 & Self::from_finger(finger).0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    fn from_finger(finger: Finger) -> Self {
        match finger {
            Finger::Index => Self::INDEX,
            Finger::Middle => Self::MIDDLE,
            Finger::Ring => Self::RING,
            Finger::Pinky => Self::PINKY,
        }
    }
}

/// Thumb state carried in a chord. Only ctrl matters here —
/// space is a word-level signal handled by the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ThumbState(pub u8);

impl ThumbState {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1);

    pub fn has_ctrl(self) -> bool {
        self.0 & Self::CTRL.0 != 0
    }
}

/// The four hand-order modes, determined by first-down and first-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Right first down, right first up.
    Mode1,
    /// Right first down, left first up.
    Mode2,
    /// Left first down, right first up.
    Mode3,
    /// Left first down, left first up.
    Mode4,
}

impl Mode {
    pub fn from_order(first_down: Hand, first_up: Hand) -> Self {
        match (first_down, first_up) {
            (Hand::Right, Hand::Right) => Self::Mode1,
            (Hand::Right, Hand::Left) => Self::Mode2,
            (Hand::Left, Hand::Right) => Self::Mode3,
            (Hand::Left, Hand::Left) => Self::Mode4,
        }
    }
}

/// A fully resolved chord: everything the interpreter needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub mode: Mode,
    pub right: FingerChord,
    pub left: FingerChord,
    pub thumbs: ThumbState,
}
