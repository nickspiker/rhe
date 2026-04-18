use crate::hand::KeyEvent;

/// Platform-agnostic input trait.
pub trait KeyInput {
    /// Block until the next key event. Returns None if the source is closed.
    fn next_event(&mut self) -> Option<KeyEvent>;
}

pub mod rdev_backend;
