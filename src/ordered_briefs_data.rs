//! Hand-curated briefs that require a specific down-order.
//!
//! Each entry claims the `(left, right)` chord slot — it locks out the
//! unordered brief table, so `gen_briefs` should skip those slots. At
//! lookup time the brief fires only when the user's first down-key
//! matches the `first_down` field; any other starting finger on a
//! claimed chord emits nothing.
//!
//! Keep the companion list in `src/bin/gen_briefs.rs` (`ORDERED_CLAIMED`)
//! in sync — `gen_briefs` reads that to avoid double-assigning these
//! chord slots to unordered briefs.
//!
//! Format: `(left_4bits, right_5bits, first_down_scancode, "word")`

use crate::scan;

// Finger difficulty for LEADING a chord (easiest → hardest):
//     thumb < pinky < index < middle < ring
//
// Thumb is the easiest lead because the thumb is stronger and more
// independent than any home-row finger. Pinky has its own tendon so
// it's easy to drop first. Ring is tendon-coupled to middle and the
// hardest finger to move alone.
//
// Assignment convention:
//   * Easiest-available finger in the chord  → most-common word
//   * Medium fingers                         → mid-frequency word
//   * Hardest-available finger               → rare word
//
// For 2-way pairs: the rare variant rides on ring (or middle, if no
// ring is in the chord); every other finger fires the primary.
// For 3-way sets: easiest = most common, medium = middle, hardest = rare.
//
// Exception: symmetric same-finger-per-hand splits (no/know,
// here/hear, right/write) use right-lead = more common, left-lead =
// less common as a hand-dominance cue instead of finger difficulty,
// since both fingers in the pair are the same and thus equally hard.
pub const ORDERED_BRIEFS: &[(u8, u8, u8, &str)] = &[
    // ─── 2-way symmetric splits (hand-dominance rule) ──────────────
    // no / know — L-mid + R-mid (N+OW)
    (0b0010, 0b00010, scan::R_MID, "no"),
    (0b0010, 0b00010, scan::L_MID, "know"),
    // here / hear — L-pinky + R-pinky (HH+IY+R)
    (0b1000, 0b01000, scan::R_PINKY, "here"),
    (0b1000, 0b01000, scan::L_PINKY, "hear"),
    // right / write — L-ring + R-ring (R+AY+T)
    (0b0100, 0b00100, scan::R_RING, "right"),
    (0b0100, 0b00100, scan::L_RING, "write"),

    // ─── Single-hand + thumb chords (thumb = easy, ring/mid = rare) ─
    // to / too / two — R-idx + R-mid + thumb
    (0b0000, 0b10011, scan::R_THUMB, "to"),
    (0b0000, 0b10011, scan::R_IDX, "too"),
    (0b0000, 0b10011, scan::R_MID, "two"),
    // in / inn — R-mid + thumb (no ring; middle = rare)
    (0b0000, 0b10010, scan::R_THUMB, "in"),
    (0b0000, 0b10010, scan::R_MID, "inn"),
    // do / due — R-idx + R-ring + thumb (ring = rare)
    (0b0000, 0b10101, scan::R_THUMB, "do"),
    (0b0000, 0b10101, scan::R_IDX, "do"),
    (0b0000, 0b10101, scan::R_RING, "due"),
    // not / knot — L-ring + thumb (ring = rare)
    (0b0100, 0b10000, scan::R_THUMB, "not"),
    (0b0100, 0b10000, scan::L_RING, "knot"),
    // be / bee — L-mid+ring+pinky + thumb (ring = rare)
    (0b1110, 0b10000, scan::R_THUMB, "be"),
    (0b1110, 0b10000, scan::L_PINKY, "be"),
    (0b1110, 0b10000, scan::L_MID, "be"),
    (0b1110, 0b10000, scan::L_RING, "bee"),
    // but / butt — L-mid + thumb (middle = rare, no ring)
    (0b0010, 0b10000, scan::R_THUMB, "but"),
    (0b0010, 0b10000, scan::L_MID, "butt"),
    // there / their — R-mid+ring + thumb (ring = rare)
    (0b0000, 0b10110, scan::R_THUMB, "there"),
    (0b0000, 0b10110, scan::R_MID, "there"),
    (0b0000, 0b10110, scan::R_RING, "their"),
    // read / red — L-ring+pinky + R-idx+mid+pinky + thumb (ring = rare)
    (0b1010, 0b11011, scan::R_THUMB, "read"),
    (0b1010, 0b11011, scan::R_IDX, "read"),
    (0b1010, 0b11011, scan::R_MID, "read"),
    (0b1010, 0b11011, scan::R_PINKY, "read"),
    (0b1010, 0b11011, scan::L_PINKY, "read"),
    (0b1010, 0b11011, scan::L_RING, "red"),
    // son / sun — L-ring + R-ring+pinky + thumb (ring = rare)
    (0b0100, 0b11100, scan::R_THUMB, "son"),
    (0b0100, 0b11100, scan::R_PINKY, "son"),
    (0b0100, 0b11100, scan::L_RING, "sun"),
    (0b0100, 0b11100, scan::R_RING, "sun"),
    // meet / meat — L-idx+mid+ring + R-mid+ring+pinky + thumb (ring = rare)
    (0b0111, 0b11110, scan::R_THUMB, "meet"),
    (0b0111, 0b11110, scan::R_PINKY, "meet"),
    (0b0111, 0b11110, scan::L_IDX, "meet"),
    (0b0111, 0b11110, scan::L_MID, "meet"),
    (0b0111, 0b11110, scan::R_MID, "meet"),
    (0b0111, 0b11110, scan::L_RING, "meat"),
    (0b0111, 0b11110, scan::R_RING, "meat"),
    // wait / weight — L-mid + R-mid + thumb (middle = rare, no ring)
    (0b0010, 0b10010, scan::R_THUMB, "wait"),
    (0b0010, 0b10010, scan::L_MID, "weight"),
    (0b0010, 0b10010, scan::R_MID, "weight"),
    // thru / through / threw — L-idx+ring + R-idx + thumb
    (0b0101, 0b10001, scan::R_THUMB, "thru"),
    (0b0101, 0b10001, scan::L_IDX, "through"),
    (0b0101, 0b10001, scan::R_IDX, "through"),
    (0b0101, 0b10001, scan::L_RING, "threw"),
    // which / witch — L-idx+mid + R-ring + thumb (ring = rare)
    (0b0011, 0b10100, scan::R_THUMB, "which"),
    (0b0011, 0b10100, scan::L_IDX, "which"),
    (0b0011, 0b10100, scan::L_MID, "which"),
    (0b0011, 0b10100, scan::R_RING, "witch"),

    // ─── No-thumb chords (ring = rare; middle if no ring) ──────────
    // our / hour — L-ring+pinky + R-pinky (ring = rare)
    (0b1100, 0b01000, scan::L_PINKY, "our"),
    (0b1100, 0b01000, scan::R_PINKY, "our"),
    (0b1100, 0b01000, scan::L_RING, "hour"),
    // where / wear — L-idx + R-idx+mid+ring (ring = rare)
    (0b0001, 0b00111, scan::L_IDX, "where"),
    (0b0001, 0b00111, scan::R_IDX, "where"),
    (0b0001, 0b00111, scan::R_MID, "where"),
    (0b0001, 0b00111, scan::R_RING, "wear"),
    // new / knew — L-idx+mid+pinky + R-pinky (no ring; middle = rare)
    (0b1101, 0b01000, scan::L_IDX, "new"),
    (0b1101, 0b01000, scan::L_PINKY, "new"),
    (0b1101, 0b01000, scan::R_PINKY, "new"),
    (0b1101, 0b01000, scan::L_MID, "knew"),
    // week / weak — L-ring+pinky + R-idx+mid+pinky (ring = rare)
    (0b1010, 0b01011, scan::L_PINKY, "week"),
    (0b1010, 0b01011, scan::R_IDX, "week"),
    (0b1010, 0b01011, scan::R_MID, "week"),
    (0b1010, 0b01011, scan::R_PINKY, "week"),
    (0b1010, 0b01011, scan::L_RING, "weak"),
    // would / wood — L-pinky + R-mid+ring (ring = rare)
    (0b1000, 0b00110, scan::L_PINKY, "would"),
    (0b1000, 0b00110, scan::R_MID, "would"),
    (0b1000, 0b00110, scan::R_RING, "wood"),
    // whole / hole — L-ring + R-idx+ring+pinky (either ring = rare)
    (0b0100, 0b01101, scan::R_IDX, "whole"),
    (0b0100, 0b01101, scan::R_PINKY, "whole"),
    (0b0100, 0b01101, scan::L_RING, "hole"),
    (0b0100, 0b01101, scan::R_RING, "hole"),
    // see / sea — L-idx + R-idx+ring (ring = rare)
    (0b0001, 0b00101, scan::L_IDX, "see"),
    (0b0001, 0b00101, scan::R_IDX, "see"),
    (0b0001, 0b00101, scan::R_RING, "sea"),
    // night / knight — L-all + R-idx+ring (either ring = rare)
    (0b1111, 0b00101, scan::L_IDX, "night"),
    (0b1111, 0b00101, scan::L_MID, "night"),
    (0b1111, 0b00101, scan::L_PINKY, "night"),
    (0b1111, 0b00101, scan::R_IDX, "night"),
    (0b1111, 0b00101, scan::L_RING, "knight"),
    (0b1111, 0b00101, scan::R_RING, "knight"),

    // ─── 3-way ────────────────────────────────────────────────────
    // for / four / fore — all 4 right-hand fingers.
    //   pinky (easy)   → for  (most common, 5.2M)
    //   index (easy)   → for  fallback
    //   middle (med)   → four (middle freq, 167K)
    //   ring (hardest) → fore (rare, 1.2K)
    (0b0000, 0b01111, scan::R_PINKY, "for"),
    (0b0000, 0b01111, scan::R_IDX, "four"),
    (0b0000, 0b01111, scan::R_MID, "four"),
    (0b0000, 0b01111, scan::R_RING, "fore"),
    // by / buy / bye — L-mid+ring + R-ring+pinky
    //   pinky (easy)   → by (most common)
    //   middle (med)   → buy
    //   ring (hardest) → bye
    (0b0110, 0b01100, scan::R_PINKY, "by"),
    (0b0110, 0b01100, scan::L_MID, "buy"),
    (0b0110, 0b01100, scan::L_RING, "bye"),
    (0b0110, 0b01100, scan::R_RING, "bye"),
];
