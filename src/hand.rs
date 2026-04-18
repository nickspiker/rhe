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
}

/// Thumb keys (one per thumb).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Thumb {
    /// Left thumb — mapped to Left Control.
    Ctrl,
    /// Right thumb — mapped to Space.
    Space,
}

/// A physical key on the keyboard, mapped to a hand+finger or thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    Finger(Hand, Finger),
    Thumb(Thumb),
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

/// Map a Dvorak home-row scancode to a PhysicalKey.
///
/// Dvorak home row (left to right):
///   Left hand:  A  O  E  U      (pinky, ring, middle, index)
///   Right hand: D  H  T  N  S   (index, middle, ring, pinky, [pinky stretch])
///
/// We use the 4 main keys per hand on Dvorak home row.
/// Left Control = left thumb, Space = right thumb.
pub fn scancode_to_key(scancode: u16) -> Option<PhysicalKey> {
    // macOS virtual keycodes for Dvorak home row
    // These are hardware scancodes, not affected by software layout.
    //
    // Key positions (QWERTY labels → Dvorak produces):
    //   A(0x00)→A  S(0x01)→O  D(0x02)→E  F(0x03)→U
    //   J(0x26)→D  K(0x28)→H  L(0x25)→T  ;(0x29)→N  (wait, macOS uses different codes)
    //
    // Actually, macOS keycodes are positional (QWERTY-based):
    //   A=0x00  S=0x01  D=0x02  F=0x03  (left hand home row)
    //   J=0x26  K=0x28  L=0x25  ;=0x29  (right hand home row)
    //   Space=0x31  Left Control=0x3B
    //
    // On Dvorak these physical positions produce: a o e u / d h t n
    // But we don't care about the letters — we care about the physical positions.

    match scancode {
        // Left hand home row (physical QWERTY A S D F positions)
        0x00 => Some(PhysicalKey::Finger(Hand::Left, Finger::Pinky)),
        0x01 => Some(PhysicalKey::Finger(Hand::Left, Finger::Ring)),
        0x02 => Some(PhysicalKey::Finger(Hand::Left, Finger::Middle)),
        0x03 => Some(PhysicalKey::Finger(Hand::Left, Finger::Index)),

        // Right hand home row (physical QWERTY J K L ; positions)
        0x26 => Some(PhysicalKey::Finger(Hand::Right, Finger::Index)),
        0x28 => Some(PhysicalKey::Finger(Hand::Right, Finger::Middle)),
        0x25 => Some(PhysicalKey::Finger(Hand::Right, Finger::Ring)),
        0x29 => Some(PhysicalKey::Finger(Hand::Right, Finger::Pinky)),

        // Thumbs
        0x31 => Some(PhysicalKey::Thumb(Thumb::Space)),
        0x3B => Some(PhysicalKey::Thumb(Thumb::Ctrl)),

        _ => None,
    }
}
