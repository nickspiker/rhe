/// Which hand a physical key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hand {
    Left,
    Right,
}

/// Which finger within a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finger {
    Index,
    Middle,
    Ring,
    Pinky,
    /// Right thumb (spacebar) — the 5th bit for right hand.
    Thumb,
}

/// A physical key on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    /// A finger key (left or right hand). Right hand includes thumb/spacebar.
    Finger(Hand, Finger),
    /// Word boundary key (left ⌘). Not part of any chord.
    Word,
}

/// Raw key event from the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDirection {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: PhysicalKey,
    pub direction: KeyDirection,
}
