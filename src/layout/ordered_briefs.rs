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
    // minute / minutes — R-idx + R-mid + R-thumb + L-pinky (minutes's chord)
    (0b1000, 0b10011, scan::R_THUMB, "minute"),
    (0b1000, 0b10011, scan::R_IDX, "minutes"),
    (0b1000, 0b10011, scan::R_MID, "minutes"),
    (0b1000, 0b10011, scan::L_PINKY, "minutes"),
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
];
