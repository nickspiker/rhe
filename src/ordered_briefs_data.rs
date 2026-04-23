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

pub const ORDERED_BRIEFS: &[(u8, u8, u8, &str)] = &[
    // Homophones T+UW — same chord, split by roll direction.
    //   L-idx + R-idx, right-index first → "to"
    //   L-idx + R-idx, left-index first  → "too"
    (0b0001, 0b00001, scan::R_IDX, "to"),
    (0b0001, 0b00001, scan::L_IDX, "too"),

    // "four" — phoneme path hits "for" since both are F-AO-R in CMU.
    // Give "four" a dedicated 3-finger right chord (index+middle+ring)
    // activated by middle-finger-first lead.
    (0b0000, 0b00111, scan::R_MID, "four"),
];
