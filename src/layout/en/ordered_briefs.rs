//! Hand-curated briefs that require a specific down-order.
//!
//! Each entry claims the `(left, right)` chord slot — it locks out the unordered brief table, so `gen_briefs` should skip those slots. At lookup time the brief fires only when the user's first down-key matches the `first_down` field; any other starting finger on a claimed chord emits nothing.
//!
//! Keep the companion list in `src/bin/gen_briefs.rs` (`ORDERED_CLAIMED`) in sync — `gen_briefs` reads that to avoid double-assigning these chord slots to unordered briefs.
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
    // ─── Morphological pairs (base + suffix on one chord) ───────────
    // Same chord serves both forms; first-down disambiguates. The
    // base + suffix path still works (`go` typed phonetically + `+ing`
    // suffix still emits `going`) — this just gives the inflected form
    // its own one-chord shortcut without consuming an extra slot.
    //
    // Convention for these pairs:
    //   thumb → more-common form; non-thumb fingers → less-common form.
    //
    // ─── Spelled numbers, position-aligned with number-mode ───────
    // one / won — R-ring + R-thumb (number-mode position 1 = R-ring)
    //   thumb (easy)   → one (more common, the spelled digit)
    //   ring  (hard)   → won (less common, past tense of "win")
    (0b0000, 0b10100, scan::R_THUMB, "one"),
    (0b0000, 0b10100, scan::R_RING, "won"),

    // ─── Homophone pairs reusing existing brief chords ────────────
    // ok / okay — okay's chord (R-idx + all 4 L-fingers, no thumb)
    //   easy fingers (L-pinky/L-idx/R-idx/L-mid) → okay (more common)
    //   ring (hardest, no thumb)                  → ok
    (0b1111, 0b00001, scan::L_PINKY, "okay"),
    (0b1111, 0b00001, scan::L_IDX, "okay"),
    (0b1111, 0b00001, scan::R_IDX, "okay"),
    (0b1111, 0b00001, scan::L_MID, "okay"),
    (0b1111, 0b00001, scan::L_RING, "ok"),
    // seen / scene — seen's chord (R-ring + R-pinky + L-mid+ring+pinky)
    //   easy fingers → seen  ring → scene
    (0b1110, 0b01100, scan::L_PINKY, "seen"),
    (0b1110, 0b01100, scan::R_PINKY, "seen"),
    (0b1110, 0b01100, scan::L_MID, "seen"),
    (0b1110, 0b01100, scan::R_RING, "seen"),
    (0b1110, 0b01100, scan::L_RING, "scene"),
    // knows / nose — knows's chord (R-idx+R-ring+R-pinky + L-idx+L-ring+L-pinky)
    //   easy fingers → knows  rings → nose
    (0b1101, 0b01101, scan::L_PINKY, "knows"),
    (0b1101, 0b01101, scan::L_IDX, "knows"),
    (0b1101, 0b01101, scan::R_IDX, "knows"),
    (0b1101, 0b01101, scan::R_PINKY, "knows"),
    (0b1101, 0b01101, scan::L_RING, "nose"),
    (0b1101, 0b01101, scan::R_RING, "nose"),
    // damn / dam — damn's chord (L-idx+L-mid+L-pinky + R-mid+R-ring+R-pinky)
    //   easy fingers → damn  R-ring → dam
    (0b1011, 0b01110, scan::L_PINKY, "damn"),
    (0b1011, 0b01110, scan::L_IDX, "damn"),
    (0b1011, 0b01110, scan::L_MID, "damn"),
    (0b1011, 0b01110, scan::R_MID, "damn"),
    (0b1011, 0b01110, scan::R_PINKY, "damn"),
    (0b1011, 0b01110, scan::R_RING, "dam"),

    // ─── Colour words ─────────────────────────────────────────────────
    // colour / color — L-idx+pinky + R-idx+mid+ring (no thumb; ring = rare)
    //   British preferred: colour gets easy fingers, color gets ring
    (0b1001, 0b00111, scan::L_IDX, "colour"),
    (0b1001, 0b00111, scan::L_PINKY, "colour"),
    (0b1001, 0b00111, scan::R_IDX, "colour"),
    (0b1001, 0b00111, scan::R_MID, "colour"),
    (0b1001, 0b00111, scan::R_RING, "color"),
    // blue / blew — all 4 L-fingers + all 4 R-fingers (no thumb; ring = rare)
    (0b1111, 0b01111, scan::L_IDX, "blue"),
    (0b1111, 0b01111, scan::L_MID, "blue"),
    (0b1111, 0b01111, scan::L_PINKY, "blue"),
    (0b1111, 0b01111, scan::R_IDX, "blue"),
    (0b1111, 0b01111, scan::R_MID, "blue"),
    (0b1111, 0b01111, scan::R_PINKY, "blue"),
    (0b1111, 0b01111, scan::L_RING, "blew"),
    (0b1111, 0b01111, scan::R_RING, "blew"),
    // green / greene — L-pinky + R-idx+mid+thumb (has thumb; no ring, mid = rare)
    (0b1000, 0b10011, scan::R_THUMB, "green"),
    (0b1000, 0b10011, scan::R_IDX, "green"),
    (0b1000, 0b10011, scan::L_PINKY, "green"),
    (0b1000, 0b10011, scan::R_MID, "greene"),
    // grey / gray — L-ring+pinky + R-mid+thumb (has thumb; ring = rare)
    (0b1100, 0b10010, scan::R_THUMB, "grey"),
    (0b1100, 0b10010, scan::L_PINKY, "grey"),
    (0b1100, 0b10010, scan::R_MID, "grey"),
    (0b1100, 0b10010, scan::L_RING, "gray"),

    // ─── Unreachable-homophone pairs needing new chord allocations ──
    // Each pair takes one chord slot from the regular brief assignment
    // pool. Lower-freq word goes on the hardest-available finger of the
    // chord; higher-freq word claims the easy fingers (thumb-lead first).
    //
    // peace / piece — chord R-I+R-M+R-P+R-thumb+L-M (displaces `black`)
    (0b0010, 0b11011, scan::R_THUMB, "piece"),
    (0b0010, 0b11011, scan::L_MID, "piece"),
    (0b0010, 0b11011, scan::R_IDX, "piece"),
    (0b0010, 0b11011, scan::R_PINKY, "piece"),
    (0b0010, 0b11011, scan::R_MID, "peace"),
    // weather / whether — chord R-M+R-thumb+L-I+L-M+L-P (displaces `funny`)
    (0b1011, 0b10010, scan::R_THUMB, "whether"),
    (0b1011, 0b10010, scan::L_IDX, "whether"),
    (0b1011, 0b10010, scan::L_PINKY, "whether"),
    (0b1011, 0b10010, scan::R_MID, "whether"),
    (0b1011, 0b10010, scan::L_MID, "weather"),
    // cell / sell — chord R-I+R-M+R-P+R-thumb+L-P (displaces `may`)
    (0b1000, 0b11011, scan::R_THUMB, "sell"),
    (0b1000, 0b11011, scan::L_PINKY, "sell"),
    (0b1000, 0b11011, scan::R_IDX, "sell"),
    (0b1000, 0b11011, scan::R_PINKY, "sell"),
    (0b1000, 0b11011, scan::R_MID, "cell"),
    // led / lead — chord R-P+R-thumb+L-I+L-M+L-P (displaces `obviously`)
    (0b1011, 0b11000, scan::R_THUMB, "lead"),
    (0b1011, 0b11000, scan::L_IDX, "lead"),
    (0b1011, 0b11000, scan::R_PINKY, "lead"),
    (0b1011, 0b11000, scan::L_PINKY, "lead"),
    (0b1011, 0b11000, scan::L_MID, "led"),
    // role / roll — chord R-I+R-M+R-P+R-thumb+L-R (displaces `order`;
    // L-RING is in the chord — perfect "ring = hardest" assignment)
    (0b0100, 0b11011, scan::R_THUMB, "roll"),
    (0b0100, 0b11011, scan::R_IDX, "roll"),
    (0b0100, 0b11011, scan::R_PINKY, "roll"),
    (0b0100, 0b11011, scan::R_MID, "roll"),
    (0b0100, 0b11011, scan::L_RING, "role"),
    // site / sight — chord R-R+R-thumb+L-I+L-M+L-P (displaces `clear`;
    // R-RING in chord = hardest finger for the lower-freq word)
    (0b1011, 0b10100, scan::R_THUMB, "sight"),
    (0b1011, 0b10100, scan::L_IDX, "sight"),
    (0b1011, 0b10100, scan::L_PINKY, "sight"),
    (0b1011, 0b10100, scan::L_MID, "sight"),
    (0b1011, 0b10100, scan::R_RING, "site"),

    // ─── Morphological pairs (base + suffix on one chord) ───────────
    // go / going — R-idx + R-pinky + R-thumb (going's chord)
    //   thumb           → going (more common, auxiliary "going to")
    //   pinky / index   → go    (less common, distinct verb)
    (0b0000, 0b11001, scan::R_THUMB, "going"),
    (0b0000, 0b11001, scan::R_PINKY, "go"),
    (0b0000, 0b11001, scan::R_IDX, "go"),
    // have / having — R-idx + R-mid + R-thumb + L-mid (having's chord)
    (0b0010, 0b10011, scan::R_THUMB, "have"),
    (0b0010, 0b10011, scan::R_IDX, "having"),
    (0b0010, 0b10011, scan::R_MID, "having"),
    (0b0010, 0b10011, scan::L_MID, "having"),
    // work / working — R-idx + R-thumb + L-ring + L-pinky (working's chord)
    (0b1100, 0b10001, scan::R_THUMB, "work"),
    (0b1100, 0b10001, scan::R_IDX, "working"),
    (0b1100, 0b10001, scan::L_RING, "working"),
    (0b1100, 0b10001, scan::L_PINKY, "working"),
    // (thing/things removed — thing now in pinned thing-family pattern;
    // things reachable via thing + s suffix.)
    // year / years — R-ring + R-thumb + L-mid (years's chord)
    (0b0010, 0b10100, scan::R_THUMB, "year"),
    (0b0010, 0b10100, scan::R_RING, "years"),
    (0b0010, 0b10100, scan::L_MID, "years"),
    // take / taking — R-idx + R-thumb + L-mid + L-ring + L-pinky (taking's chord)
    (0b1110, 0b10001, scan::R_THUMB, "take"),
    (0b1110, 0b10001, scan::R_IDX, "taking"),
    (0b1110, 0b10001, scan::L_MID, "taking"),
    (0b1110, 0b10001, scan::L_RING, "taking"),
    (0b1110, 0b10001, scan::L_PINKY, "taking"),
    // tell / telling — R-mid + R-pinky + R-thumb + L-ring (telling's chord)
    (0b0100, 0b11100, scan::R_THUMB, "tell"),
    (0b0100, 0b11100, scan::R_MID, "telling"),
    (0b0100, 0b11100, scan::R_PINKY, "telling"),
    (0b0100, 0b11100, scan::L_RING, "telling"),
    // real / really — R-mid + R-pinky + R-thumb (really's chord)
    (0b0000, 0b11010, scan::R_THUMB, "real"),
    (0b0000, 0b11010, scan::R_MID, "really"),
    (0b0000, 0b11010, scan::R_PINKY, "really"),
    // live / living — R-idx + R-ring + R-pinky + R-thumb + L-idx (living's chord)
    (0b0001, 0b11101, scan::R_THUMB, "live"),
    (0b0001, 0b11101, scan::R_IDX, "living"),
    (0b0001, 0b11101, scan::R_RING, "living"),
    (0b0001, 0b11101, scan::R_PINKY, "living"),
    (0b0001, 0b11101, scan::L_IDX, "living"),
    // serious / seriously — R-mid + R-pinky + R-thumb + L-idx + L-mid (seriously's chord)
    (0b0011, 0b11010, scan::R_THUMB, "serious"),
    (0b0011, 0b11010, scan::R_MID, "seriously"),
    (0b0011, 0b11010, scan::R_PINKY, "seriously"),
    (0b0011, 0b11010, scan::L_IDX, "seriously"),
    (0b0011, 0b11010, scan::L_MID, "seriously"),
    // your / yours — R-idx + R-pinky + R-thumb + L-idx + L-ring + L-pinky (yours's chord)
    (0b1101, 0b11001, scan::R_THUMB, "your"),
    (0b1101, 0b11001, scan::R_IDX, "yours"),
    (0b1101, 0b11001, scan::R_PINKY, "yours"),
    (0b1101, 0b11001, scan::L_IDX, "yours"),
    (0b1101, 0b11001, scan::L_RING, "yours"),
    (0b1101, 0b11001, scan::L_PINKY, "yours"),
    // speak / speaking — R-idx + R-ring + R-thumb + L-idx + L-ring + L-pinky (speaking's chord)
    (0b1101, 0b10101, scan::R_THUMB, "speak"),
    (0b1101, 0b10101, scan::R_IDX, "speaking"),
    (0b1101, 0b10101, scan::R_RING, "speaking"),
    (0b1101, 0b10101, scan::L_IDX, "speaking"),
    (0b1101, 0b10101, scan::L_RING, "speaking"),
    (0b1101, 0b10101, scan::L_PINKY, "speaking"),
    // long / longer — R-idx + R-mid + R-pinky + R-thumb + L-idx + L-ring + L-pinky (longer's chord)
    (0b1101, 0b11011, scan::R_THUMB, "long"),
    (0b1101, 0b11011, scan::R_IDX, "longer"),
    (0b1101, 0b11011, scan::R_MID, "longer"),
    (0b1101, 0b11011, scan::R_PINKY, "longer"),
    (0b1101, 0b11011, scan::L_IDX, "longer"),
    (0b1101, 0b11011, scan::L_RING, "longer"),
    (0b1101, 0b11011, scan::L_PINKY, "longer"),
    // question / questions — R-mid + R-ring + R-thumb + L-mid + L-ring + L-pinky (questions's chord)
    (0b1110, 0b10110, scan::R_THUMB, "question"),
    (0b1110, 0b10110, scan::R_MID, "questions"),
    (0b1110, 0b10110, scan::R_RING, "questions"),
    (0b1110, 0b10110, scan::L_MID, "questions"),
    (0b1110, 0b10110, scan::L_RING, "questions"),
    (0b1110, 0b10110, scan::L_PINKY, "questions"),
    // (minute/minutes removed — chord (0b1000, 0b10011) now used by
    // green/greene; minute reachable via regular brief, minutes via
    // minute + s suffix.)
    // think / thinking — R-thumb + L-mid + L-ring (think's chord)
    (0b0110, 0b10000, scan::R_THUMB, "think"),
    (0b0110, 0b10000, scan::L_MID, "thinking"),
    (0b0110, 0b10000, scan::L_RING, "thinking"),
    // start / started — R-idx + R-mid + R-pinky + R-thumb (start's chord)
    (0b0000, 0b11011, scan::R_THUMB, "start"),
    (0b0000, 0b11011, scan::R_IDX, "started"),
    (0b0000, 0b11011, scan::R_MID, "started"),
    (0b0000, 0b11011, scan::R_PINKY, "started"),
    // friend / friends — R-idx + R-mid + R-ring + R-pinky + R-thumb + L-ring (friends's chord)
    (0b0100, 0b11111, scan::R_THUMB, "friend"),
    (0b0100, 0b11111, scan::R_IDX, "friends"),
    (0b0100, 0b11111, scan::R_MID, "friends"),
    (0b0100, 0b11111, scan::R_RING, "friends"),
    (0b0100, 0b11111, scan::R_PINKY, "friends"),
    (0b0100, 0b11111, scan::L_RING, "friends"),
    // hand / hands — R-mid + R-thumb + L-idx + L-ring (hands's chord; hands more common)
    (0b0101, 0b10010, scan::R_THUMB, "hands"),
    (0b0101, 0b10010, scan::R_MID, "hand"),
    (0b0101, 0b10010, scan::L_IDX, "hand"),
    (0b0101, 0b10010, scan::L_RING, "hand"),
    // kill / killed — R-mid + R-ring + R-pinky + R-thumb + L-idx (killed's chord; killed slightly more common)
    (0b0001, 0b11110, scan::R_THUMB, "killed"),
    (0b0001, 0b11110, scan::R_MID, "kill"),
    (0b0001, 0b11110, scan::R_RING, "kill"),
    (0b0001, 0b11110, scan::R_PINKY, "kill"),
    (0b0001, 0b11110, scan::L_IDX, "kill"),
    // day / days — R-mid + R-ring + R-pinky + R-thumb + L-idx + L-mid (days's chord)
    (0b0011, 0b11110, scan::R_THUMB, "day"),
    (0b0011, 0b11110, scan::R_MID, "days"),
    (0b0011, 0b11110, scan::R_RING, "days"),
    (0b0011, 0b11110, scan::R_PINKY, "days"),
    (0b0011, 0b11110, scan::L_IDX, "days"),
    (0b0011, 0b11110, scan::L_MID, "days"),
    // care / careful — R-idx + R-ring + R-pinky + R-thumb + L-ring (careful's chord)
    (0b0100, 0b11101, scan::R_THUMB, "care"),
    (0b0100, 0b11101, scan::R_IDX, "careful"),
    (0b0100, 0b11101, scan::R_RING, "careful"),
    (0b0100, 0b11101, scan::R_PINKY, "careful"),
    (0b0100, 0b11101, scan::L_RING, "careful"),
    // (feel/feeling removed — chord 0b10111 is now the thing-family base
    // shared with something via L-MID modifier; feel and feeling go back
    // to separate auto-assigned briefs.)
    // play / playing — R-ring + R-pinky + R-thumb + L-ring + L-pinky (play's chord; playing more common)
    (0b1100, 0b11100, scan::R_THUMB, "playing"),
    (0b1100, 0b11100, scan::R_RING, "play"),
    (0b1100, 0b11100, scan::R_PINKY, "play"),
    (0b1100, 0b11100, scan::L_RING, "play"),
    (0b1100, 0b11100, scan::L_PINKY, "play"),
    // thank / thanks — R-mid + R-thumb + L-idx (thanks's chord)
    (0b0001, 0b10010, scan::R_THUMB, "thank"),
    (0b0001, 0b10010, scan::R_MID, "thanks"),
    (0b0001, 0b10010, scan::L_IDX, "thanks"),
    // kid / kids — R-idx + R-mid + R-thumb + L-idx + L-pinky (kids's chord; kids more common)
    (0b1001, 0b10011, scan::R_THUMB, "kids"),
    (0b1001, 0b10011, scan::R_IDX, "kid"),
    (0b1001, 0b10011, scan::R_MID, "kid"),
    (0b1001, 0b10011, scan::L_IDX, "kid"),
    (0b1001, 0b10011, scan::L_PINKY, "kid"),


// Total: 424 ORDERED_CLAIMED entries
    // ─── Free morphological ride-alongs (170 sets) ────────
    // Each set reuses an existing auto-assigned brief chord.
    // The brief word keeps the easy finger(s); morphological
    // relatives ride on harder fingers at zero slot cost.
    // call(572,997) / called(264,588)
    (0b0011, 0b10110, scan::R_THUMB, "call"),
    (0b0011, 0b10110, scan::L_IDX, "call"),
    (0b0011, 0b10110, scan::R_MID, "call"),
    (0b0011, 0b10110, scan::L_MID, "call"),
    (0b0011, 0b10110, scan::R_RING, "called"),
    // guys(442,241) / guy(435,832)
    (0b0001, 0b11001, scan::R_THUMB, "guys"),
    (0b0001, 0b11001, scan::R_PINKY, "guys"),
    (0b0001, 0b11001, scan::R_IDX, "guys"),
    (0b0001, 0b11001, scan::L_IDX, "guy"),
    // turn(218,563) / turned(85,279) / turns(45,374) / turning(30,420) / turner(6,951)
    (0b1101, 0b10010, scan::R_THUMB, "turn"),
    (0b1101, 0b10010, scan::L_PINKY, "turned"),
    (0b1101, 0b10010, scan::L_IDX, "turns"),
    (0b1101, 0b10010, scan::R_MID, "turning"),
    (0b1101, 0b10010, scan::L_RING, "turner"),
    // move(283,807) / moving(81,193) / moved(55,096) / moves(20,349)
    (0b0011, 0b10101, scan::R_THUMB, "move"),
    (0b0011, 0b10101, scan::R_IDX, "moving"),
    (0b0011, 0b10101, scan::L_IDX, "moving"),
    (0b0011, 0b10101, scan::L_MID, "moved"),
    (0b0011, 0b10101, scan::R_RING, "moves"),
    // love(830,324) / loved(86,138) / lovely(67,236)
    (0b0011, 0b00110, scan::L_IDX, "love"),
    (0b0011, 0b00110, scan::R_MID, "loved"),
    (0b0011, 0b00110, scan::L_MID, "loved"),
    (0b0011, 0b00110, scan::R_RING, "lovely"),
    // miss(280,930) / missing(77,326) / missed(58,324)
    (0b1100, 0b10111, scan::R_THUMB, "miss"),
    (0b1100, 0b10111, scan::L_PINKY, "miss"),
    (0b1100, 0b10111, scan::R_IDX, "miss"),
    (0b1100, 0b10111, scan::R_MID, "missing"),
    (0b1100, 0b10111, scan::R_RING, "missed"),
    (0b1100, 0b10111, scan::L_RING, "missed"),
    // change(186,962) / changed(86,133) / changes(25,456) / changing(22,102)
    (0b1010, 0b10001, scan::R_THUMB, "change"),
    (0b1010, 0b10001, scan::L_PINKY, "changed"),
    (0b1010, 0b10001, scan::R_IDX, "changes"),
    (0b1010, 0b10001, scan::L_MID, "changing"),
    // like(2,983,027) / liked(52,485) / likes(51,334) / likely(24,527) / liking(5,217)
    (0b0000, 0b11111, scan::R_THUMB, "like"),
    (0b0000, 0b11111, scan::R_PINKY, "liked"),
    (0b0000, 0b11111, scan::R_IDX, "likes"),
    (0b0000, 0b11111, scan::R_MID, "likely"),
    (0b0000, 0b11111, scan::R_RING, "liking"),
    // give(811,919) / giving(82,085) / gives(47,900)
    (0b0100, 0b00101, scan::R_IDX, "give"),
    (0b0100, 0b00101, scan::R_RING, "giving"),
    (0b0100, 0b00101, scan::L_RING, "gives"),
    // sounds(109,140) / sound(105,298)
    (0b0010, 0b11101, scan::R_THUMB, "sounds"),
    (0b0010, 0b11101, scan::R_PINKY, "sounds"),
    (0b0010, 0b11101, scan::R_IDX, "sounds"),
    (0b0010, 0b11101, scan::L_MID, "sounds"),
    (0b0010, 0b11101, scan::R_RING, "sound"),
    // close(166,677) / closed(41,801) / closer(39,022) / closes(21,761)
    (0b1010, 0b10110, scan::R_THUMB, "close"),
    (0b1010, 0b10110, scan::L_PINKY, "closed"),
    (0b1010, 0b10110, scan::R_MID, "closer"),
    (0b1010, 0b10110, scan::L_MID, "closer"),
    (0b1010, 0b10110, scan::R_RING, "closes"),
    // seems(136,699) / seem(98,575)
    (0b1010, 0b11110, scan::R_THUMB, "seems"),
    (0b1010, 0b11110, scan::R_PINKY, "seems"),
    (0b1010, 0b11110, scan::L_PINKY, "seems"),
    (0b1010, 0b11110, scan::R_MID, "seems"),
    (0b1010, 0b11110, scan::L_MID, "seems"),
    (0b1010, 0b11110, scan::R_RING, "seem"),
    // stop(574,016) / stopped(60,200) / stops(23,086) / stopping(14,667)
    (0b1111, 0b00111, scan::L_PINKY, "stop"),
    (0b1111, 0b00111, scan::R_IDX, "stopped"),
    (0b1111, 0b00111, scan::L_IDX, "stopped"),
    (0b1111, 0b00111, scan::R_MID, "stops"),
    (0b1111, 0b00111, scan::L_MID, "stops"),
    (0b1111, 0b00111, scan::R_RING, "stopping"),
    (0b1111, 0b00111, scan::L_RING, "stopping"),
    // other(577,197) / others(96,631)
    (0b0010, 0b01100, scan::R_PINKY, "other"),
    (0b0010, 0b01100, scan::L_MID, "other"),
    (0b0010, 0b01100, scan::R_RING, "others"),
    // watch(207,949) / watching(73,700) / watched(21,542)
    (0b0111, 0b11101, scan::R_THUMB, "watch"),
    (0b0111, 0b11101, scan::R_PINKY, "watch"),
    (0b0111, 0b11101, scan::R_IDX, "watch"),
    (0b0111, 0b11101, scan::L_IDX, "watch"),
    (0b0111, 0b11101, scan::L_MID, "watching"),
    (0b0111, 0b11101, scan::R_RING, "watched"),
    (0b0111, 0b11101, scan::L_RING, "watched"),
    // look(1,348,467) / looked(90,441)
    (0b0110, 0b00100, scan::L_MID, "look"),
    (0b0110, 0b00100, scan::R_RING, "looked"),
    (0b0110, 0b00100, scan::L_RING, "looked"),
    // worry(211,329) / worried(78,083) / worrying(11,246)
    (0b1100, 0b11010, scan::R_THUMB, "worry"),
    (0b1100, 0b11010, scan::R_PINKY, "worry"),
    (0b1100, 0b11010, scan::L_PINKY, "worry"),
    (0b1100, 0b11010, scan::R_MID, "worried"),
    (0b1100, 0b11010, scan::L_RING, "worrying"),
    // name(455,543) / named(44,264) / names(41,772)
    (0b0111, 0b10010, scan::R_THUMB, "name"),
    (0b0111, 0b10010, scan::L_IDX, "name"),
    (0b0111, 0b10010, scan::R_MID, "named"),
    (0b0111, 0b10010, scan::L_MID, "named"),
    (0b0111, 0b10010, scan::L_RING, "names"),
    // open(229,669) / opened(28,868) / opens(28,532) / opening(28,386)
    (0b0101, 0b10110, scan::R_THUMB, "open"),
    (0b0101, 0b10110, scan::L_IDX, "opened"),
    (0b0101, 0b10110, scan::R_MID, "opens"),
    (0b0101, 0b10110, scan::R_RING, "opening"),
    (0b0101, 0b10110, scan::L_RING, "opening"),
    // talk(567,036) / talked(67,666) / talks(17,763)
    (0b0101, 0b01111, scan::R_PINKY, "talk"),
    (0b0101, 0b01111, scan::R_IDX, "talk"),
    (0b0101, 0b01111, scan::L_IDX, "talk"),
    (0b0101, 0b01111, scan::R_MID, "talked"),
    (0b0101, 0b01111, scan::R_RING, "talks"),
    (0b0101, 0b01111, scan::L_RING, "talks"),
    // need(1,040,131) / needed(78,920) / needing(5,238)
    (0b1000, 0b00111, scan::L_PINKY, "need"),
    (0b1000, 0b00111, scan::R_IDX, "need"),
    (0b1000, 0b00111, scan::R_MID, "needed"),
    (0b1000, 0b00111, scan::R_RING, "needing"),
    // help(666,286) / helping(41,832) / helped(41,235)
    (0b0100, 0b00011, scan::R_IDX, "help"),
    (0b0100, 0b00011, scan::R_MID, "helping"),
    (0b0100, 0b00011, scan::L_RING, "helped"),
    // use(256,079) / using(66,503) / useful(15,524)
    (0b1000, 0b11010, scan::R_THUMB, "use"),
    (0b1000, 0b11010, scan::R_PINKY, "using"),
    (0b1000, 0b11010, scan::L_PINKY, "using"),
    (0b1000, 0b11010, scan::R_MID, "useful"),
    // office(141,140) / officer(76,015) / offices(5,138)
    (0b1110, 0b01101, scan::R_PINKY, "office"),
    (0b1110, 0b01101, scan::L_PINKY, "office"),
    (0b1110, 0b01101, scan::R_IDX, "office"),
    (0b1110, 0b01101, scan::L_MID, "officer"),
    (0b1110, 0b01101, scan::R_RING, "offices"),
    (0b1110, 0b01101, scan::L_RING, "offices"),
    // head(266,844) / heads(30,350) / heading(26,098) / headed(21,258)
    (0b1101, 0b00010, scan::L_PINKY, "head"),
    (0b1101, 0b00010, scan::L_IDX, "heads"),
    (0b1101, 0b00010, scan::R_MID, "heading"),
    (0b1101, 0b00010, scan::L_RING, "headed"),
    // clear(125,730) / clearly(35,837) / clears(21,657) / cleared(11,869) / clearing(6,130)
    (0b1011, 0b10110, scan::R_THUMB, "clear"),
    (0b1011, 0b10110, scan::L_PINKY, "clearly"),
    (0b1011, 0b10110, scan::L_IDX, "clears"),
    (0b1011, 0b10110, scan::R_MID, "cleared"),
    (0b1011, 0b10110, scan::L_MID, "cleared"),
    (0b1011, 0b10110, scan::R_RING, "clearing"),
    // married(157,817) / marry(73,938)
    (0b1010, 0b00111, scan::L_PINKY, "married"),
    (0b1010, 0b00111, scan::R_IDX, "married"),
    (0b1010, 0b00111, scan::R_MID, "married"),
    (0b1010, 0b00111, scan::L_MID, "married"),
    (0b1010, 0b00111, scan::R_RING, "marry"),
    // stand(155,391) / standing(57,866) / stands(15,856)
    (0b1000, 0b11110, scan::R_THUMB, "stand"),
    (0b1000, 0b11110, scan::R_PINKY, "stand"),
    (0b1000, 0b11110, scan::L_PINKY, "stand"),
    (0b1000, 0b11110, scan::R_MID, "standing"),
    (0b1000, 0b11110, scan::R_RING, "stands"),
    // supposed(153,117) / suppose(73,465)
    (0b1000, 0b11001, scan::R_THUMB, "supposed"),
    (0b1000, 0b11001, scan::R_PINKY, "supposed"),
    (0b1000, 0b11001, scan::L_PINKY, "supposed"),
    (0b1000, 0b11001, scan::R_IDX, "suppose"),
    // sit(198,149) / sitting(64,521) / sits(7,016)
    (0b1001, 0b11101, scan::R_THUMB, "sit"),
    (0b1001, 0b11101, scan::R_PINKY, "sit"),
    (0b1001, 0b11101, scan::L_PINKY, "sit"),
    (0b1001, 0b11101, scan::R_IDX, "sitting"),
    (0b1001, 0b11101, scan::L_IDX, "sitting"),
    (0b1001, 0b11101, scan::R_RING, "sits"),
    // months(129,732) / month(71,163)
    (0b0100, 0b11010, scan::R_THUMB, "months"),
    (0b0100, 0b11010, scan::R_PINKY, "months"),
    (0b0100, 0b11010, scan::R_MID, "months"),
    (0b0100, 0b11010, scan::L_RING, "month"),
    // order(125,446) / orders(43,786) / ordered(27,127)
    (0b0110, 0b11011, scan::R_THUMB, "order"),
    (0b0110, 0b11011, scan::R_PINKY, "order"),
    (0b0110, 0b11011, scan::R_IDX, "order"),
    (0b0110, 0b11011, scan::R_MID, "orders"),
    (0b0110, 0b11011, scan::L_MID, "orders"),
    (0b0110, 0b11011, scan::L_RING, "ordered"),
    // drink(182,580) / drinking(44,770) / drinks(25,643)
    (0b1001, 0b01100, scan::R_PINKY, "drink"),
    (0b1001, 0b01100, scan::L_PINKY, "drink"),
    (0b1001, 0b01100, scan::L_IDX, "drinking"),
    (0b1001, 0b01100, scan::R_RING, "drinks"),
    // put(552,246) / putting(48,797) / puts(16,846)
    (0b0001, 0b10011, scan::R_THUMB, "put"),
    (0b0001, 0b10011, scan::R_IDX, "putting"),
    (0b0001, 0b10011, scan::L_IDX, "putting"),
    (0b0001, 0b10011, scan::R_MID, "puts"),
    // promise(115,908) / promised(47,559) / promises(9,347) / promising(5,091)
    (0b0010, 0b11110, scan::R_THUMB, "promise"),
    (0b0010, 0b11110, scan::R_PINKY, "promised"),
    (0b0010, 0b11110, scan::R_MID, "promises"),
    (0b0010, 0b11110, scan::L_MID, "promises"),
    (0b0010, 0b11110, scan::R_RING, "promising"),
    // certainly(71,435) / certain(61,050)
    (0b0101, 0b11010, scan::R_THUMB, "certainly"),
    (0b0101, 0b11010, scan::R_PINKY, "certainly"),
    (0b0101, 0b11010, scan::L_IDX, "certainly"),
    (0b0101, 0b11010, scan::R_MID, "certainly"),
    (0b0101, 0b11010, scan::L_RING, "certain"),
    // hope(226,993) / hoping(37,769) / hopes(11,409) / hoped(11,315)
    (0b1101, 0b10100, scan::R_THUMB, "hope"),
    (0b1101, 0b10100, scan::L_PINKY, "hoping"),
    (0b1101, 0b10100, scan::L_IDX, "hopes"),
    (0b1101, 0b10100, scan::R_RING, "hoped"),
    (0b1101, 0b10100, scan::L_RING, "hoped"),
    // hard(242,871) / hardly(34,482) / harder(25,812)
    (0b0110, 0b11001, scan::R_THUMB, "hard"),
    (0b0110, 0b11001, scan::R_PINKY, "hard"),
    (0b0110, 0b11001, scan::R_IDX, "hard"),
    (0b0110, 0b11001, scan::L_MID, "hardly"),
    (0b0110, 0b11001, scan::L_RING, "harder"),
    // bring(247,343) / bringing(33,020) / brings(27,005)
    (0b0010, 0b11001, scan::R_THUMB, "bring"),
    (0b0010, 0b11001, scan::R_PINKY, "bring"),
    (0b0010, 0b11001, scan::R_IDX, "bringing"),
    (0b0010, 0b11001, scan::L_MID, "brings"),
    // end(237,387) / ended(29,775) / ends(28,242)
    (0b1110, 0b01010, scan::R_PINKY, "end"),
    (0b1110, 0b01010, scan::L_PINKY, "end"),
    (0b1110, 0b01010, scan::R_MID, "ended"),
    (0b1110, 0b01010, scan::L_MID, "ended"),
    (0b1110, 0b01010, scan::L_RING, "ends"),
    // hold(262,367) / holding(46,938) / holds(11,004)
    (0b0111, 0b01110, scan::R_PINKY, "hold"),
    (0b0111, 0b01110, scan::L_IDX, "hold"),
    (0b0111, 0b01110, scan::R_MID, "holding"),
    (0b0111, 0b01110, scan::L_MID, "holding"),
    (0b0111, 0b01110, scan::R_RING, "holds"),
    (0b0111, 0b01110, scan::L_RING, "holds"),
    // feel(444,014) / feels(57,388)
    (0b0111, 0b01100, scan::R_PINKY, "feel"),
    (0b0111, 0b01100, scan::L_IDX, "feel"),
    (0b0111, 0b01100, scan::L_MID, "feel"),
    (0b0111, 0b01100, scan::R_RING, "feels"),
    (0b0111, 0b01100, scan::L_RING, "feels"),
    // listen(365,791) / listening(45,564) / listened(11,059)
    (0b1000, 0b00101, scan::L_PINKY, "listen"),
    (0b1000, 0b00101, scan::R_IDX, "listening"),
    (0b1000, 0b00101, scan::R_RING, "listened"),
    // laughing(98,420) / laugh(56,460)
    (0b1011, 0b00011, scan::L_PINKY, "laughing"),
    (0b1011, 0b00011, scan::R_IDX, "laughing"),
    (0b1011, 0b00011, scan::L_IDX, "laughing"),
    (0b1011, 0b00011, scan::R_MID, "laugh"),
    (0b1011, 0b00011, scan::L_MID, "laugh"),
    // sleep(168,103) / sleeping(50,357) / sleeps(6,022)
    (0b1111, 0b01101, scan::R_PINKY, "sleep"),
    (0b1111, 0b01101, scan::L_PINKY, "sleep"),
    (0b1111, 0b01101, scan::R_IDX, "sleep"),
    (0b1111, 0b01101, scan::L_IDX, "sleep"),
    (0b1111, 0b01101, scan::L_MID, "sleeping"),
    (0b1111, 0b01101, scan::R_RING, "sleeps"),
    (0b1111, 0b01101, scan::L_RING, "sleeps"),
    // expect(69,614) / expecting(27,676) / expected(27,140)
    (0b0010, 0b01011, scan::R_PINKY, "expect"),
    (0b0010, 0b01011, scan::R_IDX, "expecting"),
    (0b0010, 0b01011, scan::R_MID, "expected"),
    (0b0010, 0b01011, scan::L_MID, "expected"),
    // find(626,520) / finding(30,196) / finds(23,019)
    (0b0110, 0b00111, scan::R_IDX, "find"),
    (0b0110, 0b00111, scan::R_MID, "finding"),
    (0b0110, 0b00111, scan::L_MID, "finding"),
    (0b0110, 0b00111, scan::R_RING, "finds"),
    (0b0110, 0b00111, scan::L_RING, "finds"),
    // place(444,863) / places(41,041) / placed(12,026)
    (0b0100, 0b01100, scan::R_PINKY, "place"),
    (0b0100, 0b01100, scan::R_RING, "places"),
    (0b0100, 0b01100, scan::L_RING, "placed"),
    // building(74,492) / build(40,738) / buildings(9,504)
    (0b1101, 0b11010, scan::R_THUMB, "building"),
    (0b1101, 0b11010, scan::R_PINKY, "building"),
    (0b1101, 0b11010, scan::L_PINKY, "building"),
    (0b1101, 0b11010, scan::L_IDX, "building"),
    (0b1101, 0b11010, scan::R_MID, "build"),
    (0b1101, 0b11010, scan::L_RING, "buildings"),
    // report(77,824) / reports(17,743) / reporter(17,553) / reported(13,643)
    (0b1010, 0b11010, scan::R_THUMB, "report"),
    (0b1010, 0b11010, scan::R_PINKY, "reports"),
    (0b1010, 0b11010, scan::L_PINKY, "reports"),
    (0b1010, 0b11010, scan::R_MID, "reporter"),
    (0b1010, 0b11010, scan::L_MID, "reported"),
    // second(193,687) / seconds(47,936)
    (0b1111, 0b01100, scan::R_PINKY, "second"),
    (0b1111, 0b01100, scan::L_PINKY, "second"),
    (0b1111, 0b01100, scan::L_IDX, "second"),
    (0b1111, 0b01100, scan::L_MID, "second"),
    (0b1111, 0b01100, scan::R_RING, "seconds"),
    (0b1111, 0b01100, scan::L_RING, "seconds"),
    // stay(378,726) / staying(47,241)
    (0b1010, 0b01000, scan::R_PINKY, "stay"),
    (0b1010, 0b01000, scan::L_PINKY, "stay"),
    (0b1010, 0b01000, scan::L_MID, "staying"),
    // lot(411,660) / lots(46,570)
    (0b0001, 0b10101, scan::R_THUMB, "lot"),
    (0b0001, 0b10101, scan::R_IDX, "lot"),
    (0b0001, 0b10101, scan::L_IDX, "lot"),
    (0b0001, 0b10101, scan::R_RING, "lots"),
    // break(143,107) / breaking(32,321) / breaks(14,045)
    (0b1010, 0b10101, scan::R_THUMB, "break"),
    (0b1010, 0b10101, scan::L_PINKY, "break"),
    (0b1010, 0b10101, scan::R_IDX, "break"),
    (0b1010, 0b10101, scan::L_MID, "breaking"),
    (0b1010, 0b10101, scan::R_RING, "breaks"),
    // return(89,451) / returned(25,860) / returning(11,434) / returns(8,924)
    (0b1011, 0b01011, scan::R_PINKY, "return"),
    (0b1011, 0b01011, scan::L_PINKY, "return"),
    (0b1011, 0b01011, scan::R_IDX, "returned"),
    (0b1011, 0b01011, scan::L_IDX, "returned"),
    (0b1011, 0b01011, scan::R_MID, "returning"),
    (0b1011, 0b01011, scan::L_MID, "returns"),
    // answer(134,088) / answers(22,116) / answering(12,168) / answered(11,431)
    (0b1111, 0b01011, scan::R_PINKY, "answer"),
    (0b1111, 0b01011, scan::L_PINKY, "answer"),
    (0b1111, 0b01011, scan::R_IDX, "answers"),
    (0b1111, 0b01011, scan::L_IDX, "answers"),
    (0b1111, 0b01011, scan::R_MID, "answering"),
    (0b1111, 0b01011, scan::L_MID, "answering"),
    (0b1111, 0b01011, scan::L_RING, "answered"),
    // finally(99,904) / final(45,614)
    (0b0110, 0b11010, scan::R_THUMB, "finally"),
    (0b0110, 0b11010, scan::R_PINKY, "finally"),
    (0b0110, 0b11010, scan::R_MID, "finally"),
    (0b0110, 0b11010, scan::L_MID, "finally"),
    (0b0110, 0b11010, scan::L_RING, "final"),
    // let(1,705,262) / letting(30,057) / lets(14,876)
    (0b0110, 0b00001, scan::R_IDX, "let"),
    (0b0110, 0b00001, scan::L_MID, "letting"),
    (0b0110, 0b00001, scan::L_RING, "lets"),
    // decided(76,683) / decide(44,806)
    (0b1100, 0b11110, scan::R_THUMB, "decided"),
    (0b1100, 0b11110, scan::R_PINKY, "decided"),
    (0b1100, 0b11110, scan::L_PINKY, "decided"),
    (0b1100, 0b11110, scan::R_MID, "decided"),
    (0b1100, 0b11110, scan::R_RING, "decide"),
    (0b1100, 0b11110, scan::L_RING, "decide"),
    // feeling(122,065) / feelings(42,843)
    (0b1111, 0b11010, scan::R_THUMB, "feeling"),
    (0b1111, 0b11010, scan::R_PINKY, "feeling"),
    (0b1111, 0b11010, scan::L_PINKY, "feeling"),
    (0b1111, 0b11010, scan::L_IDX, "feeling"),
    (0b1111, 0b11010, scan::R_MID, "feeling"),
    (0b1111, 0b11010, scan::L_MID, "feeling"),
    (0b1111, 0b11010, scan::L_RING, "feelings"),
    // big(417,218) / bigger(41,873)
    (0b1111, 0b01110, scan::R_PINKY, "big"),
    (0b1111, 0b01110, scan::L_PINKY, "big"),
    (0b1111, 0b01110, scan::L_IDX, "big"),
    (0b1111, 0b01110, scan::R_MID, "big"),
    (0b1111, 0b01110, scan::L_MID, "big"),
    (0b1111, 0b01110, scan::R_RING, "bigger"),
    (0b1111, 0b01110, scan::L_RING, "bigger"),
    // way(1,000,182) / ways(41,675)
    (0b1110, 0b00010, scan::L_PINKY, "way"),
    (0b1110, 0b00010, scan::R_MID, "way"),
    (0b1110, 0b00010, scan::L_MID, "way"),
    (0b1110, 0b00010, scan::L_RING, "ways"),
    // become(128,716) / becomes(21,768) / becoming(19,723)
    (0b0101, 0b01010, scan::R_PINKY, "become"),
    (0b0101, 0b01010, scan::L_IDX, "become"),
    (0b0101, 0b01010, scan::R_MID, "becomes"),
    (0b0101, 0b01010, scan::L_RING, "becoming"),
    // completely(72,777) / complete(40,210)
    (0b0101, 0b10111, scan::R_THUMB, "completely"),
    (0b0101, 0b10111, scan::R_IDX, "completely"),
    (0b0101, 0b10111, scan::L_IDX, "completely"),
    (0b0101, 0b10111, scan::R_MID, "completely"),
    (0b0101, 0b10111, scan::R_RING, "complete"),
    (0b0101, 0b10111, scan::L_RING, "complete"),
    // face(225,482) / faces(21,771) / facing(10,917) / faced(7,028)
    (0b1101, 0b00101, scan::L_PINKY, "face"),
    (0b1101, 0b00101, scan::R_IDX, "faces"),
    (0b1101, 0b00101, scan::L_IDX, "faces"),
    (0b1101, 0b00101, scan::R_RING, "facing"),
    (0b1101, 0b00101, scan::L_RING, "faced"),
    // dangerous(65,974) / danger(39,518)
    (0b1010, 0b01101, scan::R_PINKY, "dangerous"),
    (0b1010, 0b01101, scan::L_PINKY, "dangerous"),
    (0b1010, 0b01101, scan::R_IDX, "dangerous"),
    (0b1010, 0b01101, scan::L_MID, "dangerous"),
    (0b1010, 0b01101, scan::R_RING, "danger"),
    // interesting(65,082) / interest(39,252)
    (0b1110, 0b10111, scan::R_THUMB, "interesting"),
    (0b1110, 0b10111, scan::L_PINKY, "interesting"),
    (0b1110, 0b10111, scan::R_IDX, "interesting"),
    (0b1110, 0b10111, scan::R_MID, "interesting"),
    (0b1110, 0b10111, scan::L_MID, "interesting"),
    (0b1110, 0b10111, scan::R_RING, "interest"),
    (0b1110, 0b10111, scan::L_RING, "interest"),
    // point(174,205) / points(24,659) / pointed(6,390) / pointing(6,222)
    (0b0011, 0b11100, scan::R_THUMB, "point"),
    (0b0011, 0b11100, scan::R_PINKY, "point"),
    (0b0011, 0b11100, scan::L_IDX, "points"),
    (0b0011, 0b11100, scan::L_MID, "pointed"),
    (0b0011, 0b11100, scan::R_RING, "pointing"),
    // will(1,969,807) / willing(36,938)
    (0b0000, 0b11110, scan::R_THUMB, "will"),
    (0b0000, 0b11110, scan::R_PINKY, "will"),
    (0b0000, 0b11110, scan::R_MID, "will"),
    (0b0000, 0b11110, scan::R_RING, "willing"),
    // personal(60,381) / personally(25,058) / personality(11,303)
    (0b1011, 0b11100, scan::R_THUMB, "personal"),
    (0b1011, 0b11100, scan::R_PINKY, "personal"),
    (0b1011, 0b11100, scan::L_PINKY, "personal"),
    (0b1011, 0b11100, scan::L_IDX, "personal"),
    (0b1011, 0b11100, scan::L_MID, "personally"),
    (0b1011, 0b11100, scan::R_RING, "personality"),
    // fuck(260,121) / fucked(27,883) / fucker(8,421)
    (0b1010, 0b10100, scan::R_THUMB, "fuck"),
    (0b1010, 0b10100, scan::L_PINKY, "fuck"),
    (0b1010, 0b10100, scan::L_MID, "fucked"),
    (0b1010, 0b10100, scan::R_RING, "fucker"),
    // reason(144,399) / reasons(24,431) / reasonable(11,835)
    (0b1100, 0b11111, scan::R_THUMB, "reason"),
    (0b1100, 0b11111, scan::R_PINKY, "reason"),
    (0b1100, 0b11111, scan::L_PINKY, "reason"),
    (0b1100, 0b11111, scan::R_IDX, "reason"),
    (0b1100, 0b11111, scan::R_MID, "reasons"),
    (0b1100, 0b11111, scan::R_RING, "reasonable"),
    (0b1100, 0b11111, scan::L_RING, "reasonable"),
    // happy(262,141) / happiness(27,844) / happier(8,150)
    (0b1010, 0b00100, scan::L_PINKY, "happy"),
    (0b1010, 0b00100, scan::L_MID, "happiness"),
    (0b1010, 0b00100, scan::R_RING, "happier"),
    // send(131,999) / sending(26,184) / sends(9,325)
    (0b1101, 0b01010, scan::R_PINKY, "send"),
    (0b1101, 0b01010, scan::L_PINKY, "send"),
    (0b1101, 0b01010, scan::L_IDX, "send"),
    (0b1101, 0b01010, scan::R_MID, "sending"),
    (0b1101, 0b01010, scan::L_RING, "sends"),
    // car(330,261) / cars(35,472)
    (0b0100, 0b11110, scan::R_THUMB, "car"),
    (0b0100, 0b11110, scan::R_PINKY, "car"),
    (0b0100, 0b11110, scan::R_MID, "car"),
    (0b0100, 0b11110, scan::R_RING, "cars"),
    (0b0100, 0b11110, scan::L_RING, "cars"),
    // leave(478,282) / leaves(35,150)
    (0b1001, 0b00011, scan::L_PINKY, "leave"),
    (0b1001, 0b00011, scan::R_IDX, "leave"),
    (0b1001, 0b00011, scan::L_IDX, "leave"),
    (0b1001, 0b00011, scan::R_MID, "leaves"),
    // human(102,595) / humans(22,912) / humanity(10,845)
    (0b1010, 0b11000, scan::R_THUMB, "human"),
    (0b1010, 0b11000, scan::R_PINKY, "humans"),
    (0b1010, 0b11000, scan::L_PINKY, "humans"),
    (0b1010, 0b11000, scan::L_MID, "humanity"),
    // idea(263,097) / ideas(32,182)
    (0b1110, 0b00111, scan::L_PINKY, "idea"),
    (0b1110, 0b00111, scan::R_IDX, "idea"),
    (0b1110, 0b00111, scan::R_MID, "idea"),
    (0b1110, 0b00111, scan::L_MID, "idea"),
    (0b1110, 0b00111, scan::R_RING, "ideas"),
    (0b1110, 0b00111, scan::L_RING, "ideas"),
    // old(431,911) / older(32,130)
    (0b1010, 0b00001, scan::L_PINKY, "old"),
    (0b1010, 0b00001, scan::R_IDX, "old"),
    (0b1010, 0b00001, scan::L_MID, "older"),
    // kind(392,242) / kinds(17,172) / kindness(7,173) / kindly(7,104)
    (0b1001, 0b01111, scan::R_PINKY, "kind"),
    (0b1001, 0b01111, scan::L_PINKY, "kind"),
    (0b1001, 0b01111, scan::R_IDX, "kinds"),
    (0b1001, 0b01111, scan::L_IDX, "kinds"),
    (0b1001, 0b01111, scan::R_MID, "kindness"),
    (0b1001, 0b01111, scan::R_RING, "kindly"),
    // number(163,259) / numbers(31,285)
    (0b1100, 0b01100, scan::R_PINKY, "number"),
    (0b1100, 0b01100, scan::L_PINKY, "number"),
    (0b1100, 0b01100, scan::R_RING, "numbers"),
    (0b1100, 0b01100, scan::L_RING, "numbers"),
    // perfect(112,229) / perfectly(31,201)
    (0b0111, 0b11100, scan::R_THUMB, "perfect"),
    (0b0111, 0b11100, scan::R_PINKY, "perfect"),
    (0b0111, 0b11100, scan::L_IDX, "perfect"),
    (0b0111, 0b11100, scan::L_MID, "perfect"),
    (0b0111, 0b11100, scan::R_RING, "perfectly"),
    (0b0111, 0b11100, scan::L_RING, "perfectly"),
    // mean(821,275) / meaning(30,144)
    (0b0100, 0b01001, scan::R_PINKY, "mean"),
    (0b0100, 0b01001, scan::R_IDX, "mean"),
    (0b0100, 0b01001, scan::L_RING, "meaning"),
    // sure(709,390) / surely(29,565)
    (0b1001, 0b00110, scan::L_PINKY, "sure"),
    (0b1001, 0b00110, scan::L_IDX, "sure"),
    (0b1001, 0b00110, scan::R_MID, "sure"),
    (0b1001, 0b00110, scan::R_RING, "surely"),
    // matter(247,812) / matters(29,539)
    (0b1000, 0b10101, scan::R_THUMB, "matter"),
    (0b1000, 0b10101, scan::L_PINKY, "matter"),
    (0b1000, 0b10101, scan::R_IDX, "matter"),
    (0b1000, 0b10101, scan::R_RING, "matters"),
    // secret(89,236) / secrets(23,070) / secretly(6,398)
    (0b0101, 0b01101, scan::R_PINKY, "secret"),
    (0b0101, 0b01101, scan::R_IDX, "secrets"),
    (0b0101, 0b01101, scan::L_IDX, "secrets"),
    (0b0101, 0b01101, scan::R_RING, "secretly"),
    (0b0101, 0b01101, scan::L_RING, "secretly"),
    // believe(403,874) / believed(29,337)
    (0b1001, 0b01000, scan::R_PINKY, "believe"),
    (0b1001, 0b01000, scan::L_PINKY, "believe"),
    (0b1001, 0b01000, scan::L_IDX, "believed"),
    // totally(72,478) / total(29,264)
    (0b1100, 0b11011, scan::R_THUMB, "totally"),
    (0b1100, 0b11011, scan::R_PINKY, "totally"),
    (0b1100, 0b11011, scan::L_PINKY, "totally"),
    (0b1100, 0b11011, scan::R_IDX, "totally"),
    (0b1100, 0b11011, scan::R_MID, "totally"),
    (0b1100, 0b11011, scan::L_RING, "total"),
    // late(191,989) / lately(28,340)
    (0b1001, 0b01011, scan::R_PINKY, "late"),
    (0b1001, 0b01011, scan::L_PINKY, "late"),
    (0b1001, 0b01011, scan::R_IDX, "late"),
    (0b1001, 0b01011, scan::L_IDX, "late"),
    (0b1001, 0b01011, scan::R_MID, "lately"),
    // street(98,007) / streets(26,067)
    (0b0101, 0b11101, scan::R_THUMB, "street"),
    (0b0101, 0b11101, scan::R_PINKY, "street"),
    (0b0101, 0b11101, scan::R_IDX, "street"),
    (0b0101, 0b11101, scan::L_IDX, "street"),
    (0b0101, 0b11101, scan::R_RING, "streets"),
    (0b0101, 0b11101, scan::L_RING, "streets"),
    // door(261,118) / doors(26,020)
    (0b1010, 0b01001, scan::R_PINKY, "door"),
    (0b1010, 0b01001, scan::L_PINKY, "door"),
    (0b1010, 0b01001, scan::R_IDX, "door"),
    (0b1010, 0b01001, scan::L_MID, "doors"),
    // thought(558,542) / thoughts(25,436)
    (0b1000, 0b11111, scan::R_THUMB, "thought"),
    (0b1000, 0b11111, scan::R_PINKY, "thought"),
    (0b1000, 0b11111, scan::L_PINKY, "thought"),
    (0b1000, 0b11111, scan::R_IDX, "thought"),
    (0b1000, 0b11111, scan::R_MID, "thought"),
    (0b1000, 0b11111, scan::R_RING, "thoughts"),
    // soon(213,279) / sooner(25,236)
    (0b1011, 0b01000, scan::R_PINKY, "soon"),
    (0b1011, 0b01000, scan::L_PINKY, "soon"),
    (0b1011, 0b01000, scan::L_IDX, "soon"),
    (0b1011, 0b01000, scan::L_MID, "sooner"),
    // doctor(198,424) / doctors(24,380)
    (0b1001, 0b10100, scan::R_THUMB, "doctor"),
    (0b1001, 0b10100, scan::L_PINKY, "doctor"),
    (0b1001, 0b10100, scan::L_IDX, "doctor"),
    (0b1001, 0b10100, scan::R_RING, "doctors"),
    // protect(66,508) / protecting(13,916) / protected(10,400)
    (0b1011, 0b00111, scan::L_PINKY, "protect"),
    (0b1011, 0b00111, scan::R_IDX, "protect"),
    (0b1011, 0b00111, scan::L_IDX, "protect"),
    (0b1011, 0b00111, scan::R_MID, "protecting"),
    (0b1011, 0b00111, scan::L_MID, "protecting"),
    (0b1011, 0b00111, scan::R_RING, "protected"),
    // run(244,200) / runs(24,284)
    (0b0111, 0b01101, scan::R_PINKY, "run"),
    (0b0111, 0b01101, scan::R_IDX, "run"),
    (0b0111, 0b01101, scan::L_IDX, "run"),
    (0b0111, 0b01101, scan::L_MID, "run"),
    (0b0111, 0b01101, scan::R_RING, "runs"),
    (0b0111, 0b01101, scan::L_RING, "runs"),
    // obviously(53,527) / obvious(24,230)
    (0b1110, 0b01011, scan::R_PINKY, "obviously"),
    (0b1110, 0b01011, scan::L_PINKY, "obviously"),
    (0b1110, 0b01011, scan::R_IDX, "obviously"),
    (0b1110, 0b01011, scan::R_MID, "obviously"),
    (0b1110, 0b01011, scan::L_MID, "obviously"),
    (0b1110, 0b01011, scan::L_RING, "obvious"),
    // good(1,741,730) / goodness(24,190)
    (0b0001, 0b01000, scan::R_PINKY, "good"),
    (0b0001, 0b01000, scan::L_IDX, "goodness"),
    // part(192,248) / parts(24,186)
    (0b0011, 0b11001, scan::R_THUMB, "part"),
    (0b0011, 0b11001, scan::R_PINKY, "part"),
    (0b0011, 0b11001, scan::R_IDX, "part"),
    (0b0011, 0b11001, scan::L_IDX, "part"),
    (0b0011, 0b11001, scan::L_MID, "parts"),
    // case(215,274) / cases(24,023)
    (0b1101, 0b01100, scan::R_PINKY, "case"),
    (0b1101, 0b01100, scan::L_PINKY, "case"),
    (0b1101, 0b01100, scan::L_IDX, "case"),
    (0b1101, 0b01100, scan::R_RING, "cases"),
    (0b1101, 0b01100, scan::L_RING, "cases"),
    // job(298,911) / jobs(23,268)
    (0b0111, 0b10101, scan::R_THUMB, "job"),
    (0b0111, 0b10101, scan::R_IDX, "job"),
    (0b0111, 0b10101, scan::L_IDX, "job"),
    (0b0111, 0b10101, scan::L_MID, "job"),
    (0b0111, 0b10101, scan::R_RING, "jobs"),
    (0b0111, 0b10101, scan::L_RING, "jobs"),
    // scared(107,906) / scare(23,192)
    (0b1001, 0b11010, scan::R_THUMB, "scared"),
    (0b1001, 0b11010, scan::R_PINKY, "scared"),
    (0b1001, 0b11010, scan::L_PINKY, "scared"),
    (0b1001, 0b11010, scan::L_IDX, "scared"),
    (0b1001, 0b11010, scan::R_MID, "scare"),
    // please(842,120) / pleased(23,158)
    (0b0100, 0b01111, scan::R_PINKY, "please"),
    (0b0100, 0b01111, scan::R_IDX, "please"),
    (0b0100, 0b01111, scan::R_MID, "please"),
    (0b0100, 0b01111, scan::R_RING, "pleased"),
    (0b0100, 0b01111, scan::L_RING, "pleased"),
    // young(194,810) / younger(22,961)
    (0b1011, 0b10001, scan::R_THUMB, "young"),
    (0b1011, 0b10001, scan::L_PINKY, "young"),
    (0b1011, 0b10001, scan::R_IDX, "young"),
    (0b1011, 0b10001, scan::L_IDX, "young"),
    (0b1011, 0b10001, scan::L_MID, "younger"),
    // explain(84,368) / explained(9,346) / explains(7,850) / explaining(5,113)
    (0b0010, 0b11010, scan::R_THUMB, "explain"),
    (0b0010, 0b11010, scan::R_PINKY, "explained"),
    (0b0010, 0b11010, scan::R_MID, "explains"),
    (0b0010, 0b11010, scan::L_MID, "explaining"),
    // remember(358,291) / remembered(16,359) / remembers(5,824)
    (0b1111, 0b00010, scan::L_PINKY, "remember"),
    (0b1111, 0b00010, scan::L_IDX, "remember"),
    (0b1111, 0b00010, scan::R_MID, "remembered"),
    (0b1111, 0b00010, scan::L_MID, "remembered"),
    (0b1111, 0b00010, scan::L_RING, "remembers"),
    // million(94,514) / millions(22,009)
    (0b1010, 0b01110, scan::R_PINKY, "million"),
    (0b1010, 0b01110, scan::L_PINKY, "million"),
    (0b1010, 0b01110, scan::R_MID, "million"),
    (0b1010, 0b01110, scan::L_MID, "million"),
    (0b1010, 0b01110, scan::R_RING, "millions"),
    // bad(369,996) / badly(21,750)
    (0b1010, 0b00010, scan::L_PINKY, "bad"),
    (0b1010, 0b00010, scan::R_MID, "badly"),
    (0b1010, 0b00010, scan::L_MID, "badly"),
    // heart(213,914) / hearts(20,232)
    (0b0111, 0b11001, scan::R_THUMB, "heart"),
    (0b0111, 0b11001, scan::R_PINKY, "heart"),
    (0b0111, 0b11001, scan::R_IDX, "heart"),
    (0b0111, 0b11001, scan::L_IDX, "heart"),
    (0b0111, 0b11001, scan::L_MID, "heart"),
    (0b0111, 0b11001, scan::L_RING, "hearts"),
    // most(283,006) / mostly(19,116)
    (0b1111, 0b11111, scan::R_THUMB, "most"),
    (0b1111, 0b11111, scan::R_PINKY, "most"),
    (0b1111, 0b11111, scan::L_PINKY, "most"),
    (0b1111, 0b11111, scan::R_IDX, "most"),
    (0b1111, 0b11111, scan::L_IDX, "most"),
    (0b1111, 0b11111, scan::R_MID, "most"),
    (0b1111, 0b11111, scan::L_MID, "most"),
    (0b1111, 0b11111, scan::R_RING, "mostly"),
    (0b1111, 0b11111, scan::L_RING, "mostly"),
    // strange(76,863) / stranger(18,791)
    (0b1100, 0b11101, scan::R_THUMB, "strange"),
    (0b1100, 0b11101, scan::R_PINKY, "strange"),
    (0b1100, 0b11101, scan::L_PINKY, "strange"),
    (0b1100, 0b11101, scan::R_IDX, "strange"),
    (0b1100, 0b11101, scan::R_RING, "stranger"),
    (0b1100, 0b11101, scan::L_RING, "stranger"),
    // room(290,206) / rooms(18,562)
    (0b1110, 0b10010, scan::R_THUMB, "room"),
    (0b1110, 0b10010, scan::L_PINKY, "room"),
    (0b1110, 0b10010, scan::R_MID, "room"),
    (0b1110, 0b10010, scan::L_MID, "room"),
    (0b1110, 0b10010, scan::L_RING, "rooms"),
    // present(63,307) / presents(12,544) / presented(5,835)
    (0b1101, 0b11110, scan::R_THUMB, "present"),
    (0b1101, 0b11110, scan::R_PINKY, "present"),
    (0b1101, 0b11110, scan::L_PINKY, "present"),
    (0b1101, 0b11110, scan::L_IDX, "present"),
    (0b1101, 0b11110, scan::R_MID, "presents"),
    (0b1101, 0b11110, scan::R_RING, "presented"),
    (0b1101, 0b11110, scan::L_RING, "presented"),
    // mistake(74,957) / mistakes(18,340)
    (0b1011, 0b11111, scan::R_THUMB, "mistake"),
    (0b1011, 0b11111, scan::R_PINKY, "mistake"),
    (0b1011, 0b11111, scan::L_PINKY, "mistake"),
    (0b1011, 0b11111, scan::R_IDX, "mistake"),
    (0b1011, 0b11111, scan::L_IDX, "mistake"),
    (0b1011, 0b11111, scan::R_MID, "mistake"),
    (0b1011, 0b11111, scan::L_MID, "mistake"),
    (0b1011, 0b11111, scan::R_RING, "mistakes"),
    // house(388,585) / houses(18,338)
    (0b0101, 0b10100, scan::R_THUMB, "house"),
    (0b0101, 0b10100, scan::L_IDX, "house"),
    (0b0101, 0b10100, scan::R_RING, "houses"),
    (0b0101, 0b10100, scan::L_RING, "houses"),
    // exactly(189,513) / exact(18,213)
    (0b0110, 0b10100, scan::R_THUMB, "exactly"),
    (0b0110, 0b10100, scan::L_MID, "exactly"),
    (0b0110, 0b10100, scan::R_RING, "exact"),
    (0b0110, 0b10100, scan::L_RING, "exact"),
    // phone(235,246) / phones(12,424) / phoned(5,393)
    (0b1101, 0b00011, scan::L_PINKY, "phone"),
    (0b1101, 0b00011, scan::R_IDX, "phone"),
    (0b1101, 0b00011, scan::L_IDX, "phone"),
    (0b1101, 0b00011, scan::R_MID, "phones"),
    (0b1101, 0b00011, scan::L_RING, "phoned"),
    // chance(167,981) / chances(17,624)
    (0b1100, 0b11001, scan::R_THUMB, "chance"),
    (0b1100, 0b11001, scan::R_PINKY, "chance"),
    (0b1100, 0b11001, scan::L_PINKY, "chance"),
    (0b1100, 0b11001, scan::R_IDX, "chance"),
    (0b1100, 0b11001, scan::L_RING, "chances"),
    // moment(159,995) / moments(17,531)
    (0b0111, 0b11111, scan::R_THUMB, "moment"),
    (0b0111, 0b11111, scan::R_PINKY, "moment"),
    (0b0111, 0b11111, scan::R_IDX, "moment"),
    (0b0111, 0b11111, scan::L_IDX, "moment"),
    (0b0111, 0b11111, scan::R_MID, "moment"),
    (0b0111, 0b11111, scan::L_MID, "moment"),
    (0b0111, 0b11111, scan::R_RING, "moments"),
    (0b0111, 0b11111, scan::L_RING, "moments"),
    // sometimes(128,779) / sometime(17,142)
    (0b0100, 0b10101, scan::R_THUMB, "sometimes"),
    (0b0100, 0b10101, scan::R_IDX, "sometimes"),
    (0b0100, 0b10101, scan::R_RING, "sometime"),
    (0b0100, 0b10101, scan::L_RING, "sometime"),
    // understand(365,210) / understanding(17,000)
    (0b0100, 0b00010, scan::R_MID, "understand"),
    (0b0100, 0b00010, scan::L_RING, "understanding"),
    // want(1,950,845) / wanting(16,465)
    (0b0000, 0b01110, scan::R_PINKY, "want"),
    (0b0000, 0b01110, scan::R_MID, "want"),
    (0b0000, 0b01110, scan::R_RING, "wanting"),
    // fact(124,995) / facts(16,365)
    (0b1111, 0b11011, scan::R_THUMB, "fact"),
    (0b1111, 0b11011, scan::R_PINKY, "fact"),
    (0b1111, 0b11011, scan::L_PINKY, "fact"),
    (0b1111, 0b11011, scan::R_IDX, "fact"),
    (0b1111, 0b11011, scan::L_IDX, "fact"),
    (0b1111, 0b11011, scan::R_MID, "fact"),
    (0b1111, 0b11011, scan::L_MID, "fact"),
    (0b1111, 0b11011, scan::L_RING, "facts"),
    // trouble(143,235) / troubles(9,940) / troubled(6,282)
    (0b0011, 0b10111, scan::R_THUMB, "trouble"),
    (0b0011, 0b10111, scan::R_IDX, "trouble"),
    (0b0011, 0b10111, scan::L_IDX, "trouble"),
    (0b0011, 0b10111, scan::R_MID, "troubles"),
    (0b0011, 0b10111, scan::L_MID, "troubles"),
    (0b0011, 0b10111, scan::R_RING, "troubled"),
    // actually(279,595) / actual(16,107)
    (0b0110, 0b00011, scan::R_IDX, "actually"),
    (0b0110, 0b00011, scan::R_MID, "actually"),
    (0b0110, 0b00011, scan::L_MID, "actually"),
    (0b0110, 0b00011, scan::L_RING, "actual"),
    // security(71,503) / secure(16,048)
    (0b1100, 0b01110, scan::R_PINKY, "security"),
    (0b1100, 0b01110, scan::L_PINKY, "security"),
    (0b1100, 0b01110, scan::R_MID, "security"),
    (0b1100, 0b01110, scan::R_RING, "secure"),
    (0b1100, 0b01110, scan::L_RING, "secure"),
    // experience(47,824) / experienced(10,323) / experiences(5,392)
    (0b1100, 0b01101, scan::R_PINKY, "experience"),
    (0b1100, 0b01101, scan::L_PINKY, "experience"),
    (0b1100, 0b01101, scan::R_IDX, "experienced"),
    (0b1100, 0b01101, scan::R_RING, "experiences"),
    (0b1100, 0b01101, scan::L_RING, "experiences"),
    // mind(312,927) / minds(15,707)
    (0b0110, 0b11111, scan::R_THUMB, "mind"),
    (0b0110, 0b11111, scan::R_PINKY, "mind"),
    (0b0110, 0b11111, scan::R_IDX, "mind"),
    (0b0110, 0b11111, scan::R_MID, "mind"),
    (0b0110, 0b11111, scan::L_MID, "mind"),
    (0b0110, 0b11111, scan::R_RING, "minds"),
    (0b0110, 0b11111, scan::L_RING, "minds"),
    // guess(250,320) / guessing(9,912) / guessed(5,777)
    (0b1010, 0b01100, scan::R_PINKY, "guess"),
    (0b1010, 0b01100, scan::L_PINKY, "guess"),
    (0b1010, 0b01100, scan::L_MID, "guessing"),
    (0b1010, 0b01100, scan::R_RING, "guessed"),
    // nice(405,497) / nicely(9,761) / nicer(5,618)
    (0b1001, 0b01001, scan::R_PINKY, "nice"),
    (0b1001, 0b01001, scan::L_PINKY, "nice"),
    (0b1001, 0b01001, scan::R_IDX, "nicely"),
    (0b1001, 0b01001, scan::L_IDX, "nicer"),
    // quiet(90,552) / quietly(15,115)
    (0b1110, 0b11011, scan::R_THUMB, "quiet"),
    (0b1110, 0b11011, scan::R_PINKY, "quiet"),
    (0b1110, 0b11011, scan::L_PINKY, "quiet"),
    (0b1110, 0b11011, scan::R_IDX, "quiet"),
    (0b1110, 0b11011, scan::R_MID, "quiet"),
    (0b1110, 0b11011, scan::L_MID, "quiet"),
    (0b1110, 0b11011, scan::L_RING, "quietly"),
    // trust(141,327) / trusted(14,722)
    (0b1101, 0b00100, scan::L_PINKY, "trust"),
    (0b1101, 0b00100, scan::L_IDX, "trust"),
    (0b1101, 0b00100, scan::R_RING, "trusted"),
    (0b1101, 0b00100, scan::L_RING, "trusted"),
    // ask(354,323) / asks(14,655)
    (0b0110, 0b01010, scan::R_PINKY, "ask"),
    (0b0110, 0b01010, scan::R_MID, "ask"),
    (0b0110, 0b01010, scan::L_MID, "ask"),
    (0b0110, 0b01010, scan::L_RING, "asks"),
    // great(545,768) / greater(14,569)
    (0b0101, 0b00100, scan::L_IDX, "great"),
    (0b0101, 0b00100, scan::R_RING, "greater"),
    (0b0101, 0b00100, scan::L_RING, "greater"),
    // control(102,782) / controlled(7,084) / controls(6,520)
    (0b1101, 0b00001, scan::L_PINKY, "control"),
    (0b1101, 0b00001, scan::R_IDX, "controlled"),
    (0b1101, 0b00001, scan::L_IDX, "controlled"),
    (0b1101, 0b00001, scan::L_RING, "controls"),
    // system(73,678) / systems(12,464)
    (0b1011, 0b11001, scan::R_THUMB, "system"),
    (0b1011, 0b11001, scan::R_PINKY, "system"),
    (0b1011, 0b11001, scan::L_PINKY, "system"),
    (0b1011, 0b11001, scan::R_IDX, "system"),
    (0b1011, 0b11001, scan::L_IDX, "system"),
    (0b1011, 0b11001, scan::L_MID, "systems"),
    // time(1,453,708) / timing(12,009)
    (0b0010, 0b00100, scan::L_MID, "time"),
    (0b0010, 0b00100, scan::R_RING, "timing"),
    // home(556,894) / homes(11,826)
    (0b0110, 0b10110, scan::R_THUMB, "home"),
    (0b0110, 0b10110, scan::R_MID, "home"),
    (0b0110, 0b10110, scan::L_MID, "home"),
    (0b0110, 0b10110, scan::R_RING, "homes"),
    (0b0110, 0b10110, scan::L_RING, "homes"),
    // being(386,528) / beings(11,805)
    (0b1100, 0b00010, scan::L_PINKY, "being"),
    (0b1100, 0b00010, scan::R_MID, "being"),
    (0b1100, 0b00010, scan::L_RING, "beings"),
    // absolutely(84,856) / absolute(10,696)
    (0b1111, 0b11001, scan::R_THUMB, "absolutely"),
    (0b1111, 0b11001, scan::R_PINKY, "absolutely"),
    (0b1111, 0b11001, scan::L_PINKY, "absolutely"),
    (0b1111, 0b11001, scan::R_IDX, "absolutely"),
    (0b1111, 0b11001, scan::L_IDX, "absolutely"),
    (0b1111, 0b11001, scan::L_MID, "absolutely"),
    (0b1111, 0b11001, scan::L_RING, "absolute"),
    // school(240,176) / schools(10,470)
    (0b0111, 0b10011, scan::R_THUMB, "school"),
    (0b0111, 0b10011, scan::R_IDX, "school"),
    (0b0111, 0b10011, scan::L_IDX, "school"),
    (0b0111, 0b10011, scan::R_MID, "school"),
    (0b0111, 0b10011, scan::L_MID, "school"),
    (0b0111, 0b10011, scan::L_RING, "schools"),
    // imagine(71,873) / imagined(10,353)
    (0b1101, 0b01011, scan::R_PINKY, "imagine"),
    (0b1101, 0b01011, scan::L_PINKY, "imagine"),
    (0b1101, 0b01011, scan::R_IDX, "imagine"),
    (0b1101, 0b01011, scan::L_IDX, "imagine"),
    (0b1101, 0b01011, scan::R_MID, "imagine"),
    (0b1101, 0b01011, scan::L_RING, "imagined"),
    // parents(110,206) / parent(10,204)
    (0b0010, 0b11100, scan::R_THUMB, "parents"),
    (0b0010, 0b11100, scan::R_PINKY, "parents"),
    (0b0010, 0b11100, scan::L_MID, "parents"),
    (0b0010, 0b11100, scan::R_RING, "parent"),
    // water(193,014) / waters(9,992)
    (0b1111, 0b11100, scan::R_THUMB, "water"),
    (0b1111, 0b11100, scan::R_PINKY, "water"),
    (0b1111, 0b11100, scan::L_PINKY, "water"),
    (0b1111, 0b11100, scan::L_IDX, "water"),
    (0b1111, 0b11100, scan::L_MID, "water"),
    (0b1111, 0b11100, scan::R_RING, "waters"),
    (0b1111, 0b11100, scan::L_RING, "waters"),
    // relationship(55,128) / relationships(9,541)
    (0b1100, 0b10101, scan::R_THUMB, "relationship"),
    (0b1100, 0b10101, scan::L_PINKY, "relationship"),
    (0b1100, 0b10101, scan::R_IDX, "relationship"),
    (0b1100, 0b10101, scan::R_RING, "relationships"),
    (0b1100, 0b10101, scan::L_RING, "relationships"),
    // different(174,657) / differently(9,453)
    (0b1011, 0b10000, scan::R_THUMB, "different"),
    (0b1011, 0b10000, scan::L_PINKY, "different"),
    (0b1011, 0b10000, scan::L_IDX, "different"),
    (0b1011, 0b10000, scan::L_MID, "differently"),
    // dead(317,589) / deadly(9,410)
    (0b0101, 0b01110, scan::R_PINKY, "dead"),
    (0b0101, 0b01110, scan::L_IDX, "dead"),
    (0b0101, 0b01110, scan::R_MID, "dead"),
    (0b0101, 0b01110, scan::R_RING, "deadly"),
    (0b0101, 0b01110, scan::L_RING, "deadly"),
    // music(207,205) / musical(9,325)
    (0b0101, 0b00111, scan::R_IDX, "music"),
    (0b0101, 0b00111, scan::L_IDX, "music"),
    (0b0101, 0b00111, scan::R_MID, "music"),
    (0b0101, 0b00111, scan::R_RING, "musical"),
    (0b0101, 0b00111, scan::L_RING, "musical"),
    // bit(258,929) / bits(9,246)
    (0b1101, 0b00110, scan::L_PINKY, "bit"),
    (0b1101, 0b00110, scan::L_IDX, "bit"),
    (0b1101, 0b00110, scan::R_MID, "bit"),
    (0b1101, 0b00110, scan::R_RING, "bits"),
    (0b1101, 0b00110, scan::L_RING, "bits"),
    // meeting(97,328) / meetings(9,099)
    (0b0001, 0b11011, scan::R_THUMB, "meeting"),
    (0b0001, 0b11011, scan::R_PINKY, "meeting"),
    (0b0001, 0b11011, scan::R_IDX, "meeting"),
    (0b0001, 0b11011, scan::L_IDX, "meeting"),
    (0b0001, 0b11011, scan::R_MID, "meetings"),
    // forget(188,628) / forgetting(8,105)
    (0b1111, 0b10011, scan::R_THUMB, "forget"),
    (0b1111, 0b10011, scan::L_PINKY, "forget"),
    (0b1111, 0b10011, scan::R_IDX, "forget"),
    (0b1111, 0b10011, scan::L_IDX, "forget"),
    (0b1111, 0b10011, scan::R_MID, "forget"),
    (0b1111, 0b10011, scan::L_MID, "forget"),
    (0b1111, 0b10011, scan::L_RING, "forgetting"),
    // immediately(47,938) / immediate(8,061)
    (0b1011, 0b00101, scan::L_PINKY, "immediately"),
    (0b1011, 0b00101, scan::R_IDX, "immediately"),
    (0b1011, 0b00101, scan::L_IDX, "immediately"),
    (0b1011, 0b00101, scan::L_MID, "immediately"),
    (0b1011, 0b00101, scan::R_RING, "immediate"),
    // detective(60,633) / detectives(8,036)
    (0b1111, 0b11101, scan::R_THUMB, "detective"),
    (0b1111, 0b11101, scan::R_PINKY, "detective"),
    (0b1111, 0b11101, scan::L_PINKY, "detective"),
    (0b1111, 0b11101, scan::R_IDX, "detective"),
    (0b1111, 0b11101, scan::L_IDX, "detective"),
    (0b1111, 0b11101, scan::L_MID, "detective"),
    (0b1111, 0b11101, scan::R_RING, "detectives"),
    (0b1111, 0b11101, scan::L_RING, "detectives"),
    // continue(51,279) / continued(7,998)
    (0b1011, 0b01101, scan::R_PINKY, "continue"),
    (0b1011, 0b01101, scan::L_PINKY, "continue"),
    (0b1011, 0b01101, scan::R_IDX, "continue"),
    (0b1011, 0b01101, scan::L_IDX, "continue"),
    (0b1011, 0b01101, scan::L_MID, "continue"),
    (0b1011, 0b01101, scan::R_RING, "continued"),
    // world(370,620) / worlds(7,827)
    (0b0011, 0b10001, scan::R_THUMB, "world"),
    (0b0011, 0b10001, scan::R_IDX, "world"),
    (0b0011, 0b10001, scan::L_IDX, "world"),
    (0b0011, 0b10001, scan::L_MID, "worlds"),
    // death(189,000) / deaths(7,780)
    (0b1011, 0b01100, scan::R_PINKY, "death"),
    (0b1011, 0b01100, scan::L_PINKY, "death"),
    (0b1011, 0b01100, scan::L_IDX, "death"),
    (0b1011, 0b01100, scan::L_MID, "death"),
    (0b1011, 0b01100, scan::R_RING, "deaths"),
    // excuse(219,855) / excuses(7,477)
    (0b0010, 0b11000, scan::R_THUMB, "excuse"),
    (0b0010, 0b11000, scan::R_PINKY, "excuse"),
    (0b0010, 0b11000, scan::L_MID, "excuses"),
    // stuff(190,629) / stuffed(7,114)
    (0b0110, 0b11110, scan::R_THUMB, "stuff"),
    (0b0110, 0b11110, scan::R_PINKY, "stuff"),
    (0b0110, 0b11110, scan::R_MID, "stuff"),
    (0b0110, 0b11110, scan::L_MID, "stuff"),
    (0b0110, 0b11110, scan::R_RING, "stuffed"),
    (0b0110, 0b11110, scan::L_RING, "stuffed"),
    // person(192,652) / persons(6,965)
    (0b1001, 0b11000, scan::R_THUMB, "person"),
    (0b1001, 0b11000, scan::R_PINKY, "person"),
    (0b1001, 0b11000, scan::L_PINKY, "person"),
    (0b1001, 0b11000, scan::L_IDX, "persons"),
    // anyway(166,702) / anyways(6,926)
    (0b1110, 0b01001, scan::R_PINKY, "anyway"),
    (0b1110, 0b01001, scan::L_PINKY, "anyway"),
    (0b1110, 0b01001, scan::R_IDX, "anyway"),
    (0b1110, 0b01001, scan::L_MID, "anyway"),
    (0b1110, 0b01001, scan::L_RING, "anyways"),
    // couple(138,996) / couples(6,872)
    (0b0001, 0b11010, scan::R_THUMB, "couple"),
    (0b0001, 0b11010, scan::R_PINKY, "couple"),
    (0b0001, 0b11010, scan::L_IDX, "couple"),
    (0b0001, 0b11010, scan::R_MID, "couples"),
    // out(2,510,010) / outer(6,677)
    (0b0110, 0b00010, scan::R_MID, "out"),
    (0b0110, 0b00010, scan::L_MID, "out"),
    (0b0110, 0b00010, scan::L_RING, "outer"),
    // back(1,405,024) / backs(6,569)
    (0b1101, 0b10000, scan::R_THUMB, "back"),
    (0b1101, 0b10000, scan::L_PINKY, "back"),
    (0b1101, 0b10000, scan::L_IDX, "back"),
    (0b1101, 0b10000, scan::L_RING, "backs"),
    // general(76,816) / generally(6,401)
    (0b1011, 0b01001, scan::R_PINKY, "general"),
    (0b1011, 0b01001, scan::L_PINKY, "general"),
    (0b1011, 0b01001, scan::R_IDX, "general"),
    (0b1011, 0b01001, scan::L_IDX, "general"),
    (0b1011, 0b01001, scan::L_MID, "generally"),
    // well(2,159,909) / wells(5,955)
    (0b1100, 0b10000, scan::R_THUMB, "well"),
    (0b1100, 0b10000, scan::L_PINKY, "well"),
    (0b1100, 0b10000, scan::L_RING, "wells"),
    // situation(66,029) / situations(5,693)
    (0b1101, 0b10001, scan::R_THUMB, "situation"),
    (0b1101, 0b10001, scan::L_PINKY, "situation"),
    (0b1101, 0b10001, scan::R_IDX, "situation"),
    (0b1101, 0b10001, scan::L_IDX, "situation"),
    (0b1101, 0b10001, scan::L_RING, "situations"),
    // rest(158,976) / resting(5,469)
    (0b1001, 0b11110, scan::R_THUMB, "rest"),
    (0b1001, 0b11110, scan::R_PINKY, "rest"),
    (0b1001, 0b11110, scan::L_PINKY, "rest"),
    (0b1001, 0b11110, scan::L_IDX, "rest"),
    (0b1001, 0b11110, scan::R_MID, "rest"),
    (0b1001, 0b11110, scan::R_RING, "resting"),
    // hospital(98,076) / hospitals(5,457)
    (0b1100, 0b11000, scan::R_THUMB, "hospital"),
    (0b1100, 0b11000, scan::R_PINKY, "hospital"),
    (0b1100, 0b11000, scan::L_PINKY, "hospital"),
    (0b1100, 0b11000, scan::L_RING, "hospitals"),
    // important(164,386) / importantly(5,376)
    (0b0001, 0b01110, scan::R_PINKY, "important"),
    (0b0001, 0b01110, scan::L_IDX, "important"),
    (0b0001, 0b01110, scan::R_MID, "important"),
    (0b0001, 0b01110, scan::R_RING, "importantly"),
    // beautiful(200,811) / beautifully(5,095)
    (0b0010, 0b00101, scan::R_IDX, "beautiful"),
    (0b0010, 0b00101, scan::L_MID, "beautiful"),
    (0b0010, 0b00101, scan::R_RING, "beautifully"),

    // 170 sets, 254 ride-along words


];
