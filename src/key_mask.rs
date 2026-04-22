//! 256-bit bitmask indexed by HID keyboard usage code (0–255).
//!
//! Covers the full standard HID keyboard usage page in a fixed 32-byte
//! value. Operations compile to a handful of `u64` instructions — bitwise
//! OR/AND/XOR across the four underlying words, no heap, no iteration.
//!
//! Used by the chord pipeline to represent "which physical keys are part
//! of this chord" without caring about hand/finger layout. Lookups against
//! phoneme/brief/digit tables are `HashMap<KeyMask, _>` — `Hash` + `Eq` are
//! trivially derived from the raw array.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

/// Fixed-size 256-bit bitmask. Bit `n` (for `n ∈ 0..256`) corresponds to
/// HID usage code `n`. Storage layout: `0[0]` holds bits 0–63, `0[3]` holds
/// bits 192–255.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct KeyMask([u64; 4]);

impl KeyMask {
    /// All zeros. `const` so it can initialize `static`/`const` items.
    pub const EMPTY: Self = Self([0; 4]);

    /// Return a new mask with `code` set. `const`-compatible so chord
    /// literals can be built at compile time: `KeyMask::EMPTY.with(30)`.
    pub const fn with(self, code: u8) -> Self {
        let mut raw = self.0;
        let idx = (code >> 6) as usize;
        raw[idx] |= 1u64 << (code & 63);
        Self(raw)
    }

    /// Set the bit for scancode `code`.
    pub fn set(&mut self, code: u8) {
        self.0[(code >> 6) as usize] |= 1u64 << (code & 63);
    }

    /// Clear the bit for scancode `code`.
    pub fn clear(&mut self, code: u8) {
        self.0[(code >> 6) as usize] &= !(1u64 << (code & 63));
    }

    /// True when bit `code` is set.
    pub fn test(&self, code: u8) -> bool {
        self.0[(code >> 6) as usize] & (1u64 << (code & 63)) != 0
    }

    /// True when no bits are set.
    pub fn is_empty(&self) -> bool {
        self.0 == [0; 4]
    }

    /// Count of set bits.
    pub fn count_ones(&self) -> u32 {
        self.0[0].count_ones()
            + self.0[1].count_ones()
            + self.0[2].count_ones()
            + self.0[3].count_ones()
    }

    /// Iterate over set scancodes in ascending order.
    pub fn iter(&self) -> KeyMaskIter<'_> {
        KeyMaskIter {
            mask: self,
            word_idx: 0,
            current: self.0[0],
        }
    }

    /// Raw underlying words (bits 0–63 in `[0]`, 192–255 in `[3]`).
    pub const fn from_raw(raw: [u64; 4]) -> Self {
        Self(raw)
    }

    pub const fn as_raw(&self) -> [u64; 4] {
        self.0
    }
}

/// Ascending iterator over set scancodes.
pub struct KeyMaskIter<'a> {
    mask: &'a KeyMask,
    word_idx: usize,
    current: u64,
}

impl<'a> Iterator for KeyMaskIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        loop {
            if self.current != 0 {
                // trailing_zeros gives the position of the lowest set bit
                let bit = self.current.trailing_zeros() as u8;
                self.current &= self.current - 1; // clear lowest set bit
                return Some(self.word_idx as u8 * 64 + bit);
            }
            self.word_idx += 1;
            if self.word_idx >= 4 {
                return None;
            }
            self.current = self.mask.0[self.word_idx];
        }
    }
}

impl BitOr for KeyMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
            self.0[3] | rhs.0[3],
        ])
    }
}

impl BitOrAssign for KeyMask {
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..4 {
            self.0[i] |= rhs.0[i];
        }
    }
}

impl BitAnd for KeyMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
            self.0[3] & rhs.0[3],
        ])
    }
}

impl BitAndAssign for KeyMask {
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..4 {
            self.0[i] &= rhs.0[i];
        }
    }
}

impl BitXor for KeyMask {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self([
            self.0[0] ^ rhs.0[0],
            self.0[1] ^ rhs.0[1],
            self.0[2] ^ rhs.0[2],
            self.0[3] ^ rhs.0[3],
        ])
    }
}

impl BitXorAssign for KeyMask {
    fn bitxor_assign(&mut self, rhs: Self) {
        for i in 0..4 {
            self.0[i] ^= rhs.0[i];
        }
    }
}

impl Not for KeyMask {
    type Output = Self;
    fn not(self) -> Self {
        Self([!self.0[0], !self.0[1], !self.0[2], !self.0[3]])
    }
}

impl std::fmt::Debug for KeyMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeyMask{")?;
        let mut first = true;
        for code in self.iter() {
            if !first {
                f.write_str(", ")?;
            }
            write!(f, "{}", code)?;
            first = false;
        }
        f.write_str("}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mask() {
        let m = KeyMask::EMPTY;
        assert!(m.is_empty());
        assert_eq!(m.count_ones(), 0);
        assert_eq!(m.iter().count(), 0);
    }

    #[test]
    fn set_test_clear() {
        let mut m = KeyMask::EMPTY;
        m.set(0);
        m.set(63);
        m.set(64);
        m.set(128);
        m.set(255);
        assert!(m.test(0));
        assert!(m.test(63));
        assert!(m.test(64));
        assert!(m.test(128));
        assert!(m.test(255));
        assert!(!m.test(1));
        assert!(!m.test(127));
        assert_eq!(m.count_ones(), 5);

        m.clear(64);
        assert!(!m.test(64));
        assert_eq!(m.count_ones(), 4);
    }

    #[test]
    fn with_is_const() {
        const MASK: KeyMask = KeyMask::EMPTY.with(30).with(31).with(36);
        assert!(MASK.test(30));
        assert!(MASK.test(31));
        assert!(MASK.test(36));
        assert!(!MASK.test(32));
        assert_eq!(MASK.count_ones(), 3);
    }

    #[test]
    fn iter_ascending() {
        let mask = KeyMask::EMPTY.with(200).with(5).with(64).with(63);
        let codes: Vec<u8> = mask.iter().collect();
        assert_eq!(codes, vec![5, 63, 64, 200]);
    }

    #[test]
    fn bitwise_ops() {
        let a = KeyMask::EMPTY.with(10).with(20);
        let b = KeyMask::EMPTY.with(20).with(30);

        let or = a | b;
        assert_eq!(or.count_ones(), 3);
        assert!(or.test(10) && or.test(20) && or.test(30));

        let and = a & b;
        assert_eq!(and.count_ones(), 1);
        assert!(and.test(20));

        let xor = a ^ b;
        assert_eq!(xor.count_ones(), 2);
        assert!(xor.test(10) && xor.test(30) && !xor.test(20));

        let mut assign = a;
        assign |= b;
        assert_eq!(assign, or);
    }

    #[test]
    fn hash_equality() {
        use std::collections::HashMap;
        let a = KeyMask::EMPTY.with(30).with(31);
        let b = KeyMask::EMPTY.with(31).with(30);
        assert_eq!(a, b);

        let mut map: HashMap<KeyMask, &'static str> = HashMap::new();
        map.insert(a, "hello");
        assert_eq!(map.get(&b), Some(&"hello"));
    }

    #[test]
    fn raw_roundtrip() {
        let m = KeyMask::EMPTY.with(0).with(100).with(200);
        let raw = m.as_raw();
        let reconstructed = KeyMask::from_raw(raw);
        assert_eq!(m, reconstructed);
    }
}
