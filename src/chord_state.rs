use crate::hand::Finger;

/// Bitmask of which fingers are pressed on a single hand.
/// Bit 0 = Index, Bit 1 = Middle, Bit 2 = Ring, Bit 3 = Pinky.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FingerChord(pub u8);

impl FingerChord {
    pub const NONE: Self = Self(0);

    pub fn set(&mut self, finger: Finger) {
        self.0 |= Self::from_finger(finger).0;
    }

    pub fn clear(&mut self, finger: Finger) {
        self.0 &= !Self::from_finger(finger).0;
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn from_finger(finger: Finger) -> Self {
        match finger {
            Finger::Index => Self(1 << 0),
            Finger::Middle => Self(1 << 1),
            Finger::Ring => Self(1 << 2),
            Finger::Pinky => Self(1 << 3),
        }
    }
}

/// A fully resolved chord: everything the interpreter needs.
/// No mode — ordering doesn't matter. Just which keys were held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Chord {
    pub right: FingerChord,
    pub left: FingerChord,
    pub modkey: bool,
    /// Was space held when this chord completed?
    pub space_held: bool,
}
