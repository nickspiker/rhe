//! Tutor window geometry — every position, size, font, and cell-row
//! dimension lives here.
//!
//! **Vertical layout — proportional sections + gaps** that sum to
//! however-many units, then `unit = bh / total_units`. Adding or
//! removing a section just changes the total; nothing else has to
//! move. Current section weights:
//!
//! ```text
//!   1   — top padding         (TOP_PAD)
//!   0.5 — gap                 (GAP)
//!   1   — paragraph           (top sentence-context line)
//!   0.5 — gap
//!   3   — target word
//!   0.5 — gap
//!   2   — phoneme line        (target word's phoneme strip)
//!   0.5 — gap
//!   0.5 — adaptive cell labels
//!   0.5 — gap
//!   2   — chord row           (10 finger cells)
//!   0.5 — gap
//!   2   — bottom row          (word bar + mod pill)
//!   0.5 — gap
//!   1   — bottom padding      (BOTTOM_PAD)
//! ```
//!
//! Each section's `*_cy` field on [`TutorLayout`] is its vertical
//! centre. Horizontal geometry (row_w, cell_d, hand_gap = cell_d,
//! cell_gap = slack) is span/ru-derived. `cell_d` is also clamped
//! to the chord row's section height so pills never overflow a
//! section and scribble outside the pixel buffer.
//!
//! All knobs (section weights, horizontal pad, font multipliers /
//! caps, zoom range) are `pub const`s at the top of this file so
//! they're easy to tweak without hunting through render code.

use super::span;

// ── Vertical-section weights ─────────────────────────────────────

/// Top padding (above the paragraph line).
pub const TOP_PAD: f32 = 1.0;
/// Gap between any two adjacent sections (incl. between padding
/// and adjacent content).
pub const GAP: f32 = 0.5;
/// Top scrolling sentence-context line.
pub const PARAGRAPH: f32 = 1.0;
/// Big centred drill word.
pub const TARGET: f32 = 3.0;
/// Phoneme strip for the current target word.
pub const PHONEME: f32 = 2.0;
/// Adaptive per-cell labels above the chord row.
pub const LABELS: f32 = 0.5;
/// Chord row (10 finger cells + 1 invisible middle slot).
pub const CHORD_ROW: f32 = 2.0;
/// Bottom row (word bar + 2-cell mod pill).
pub const BOTTOM_ROW: f32 = 2.0;
/// Bottom padding (below the bottom row).
pub const BOTTOM_PAD: f32 = 1.0;

// ── Horizontal layout ────────────────────────────────────────────

/// Outer horizontal padding on each side of the chord row, in the
/// same `unit` as the vertical sections. Mirrors the vertical pad
/// weight so the chord row has visible breathing room from the
/// window edges at any zoom.
pub const H_PAD_UNITS: f32 = 1.0;

// ── Font scaling ────────────────────────────────────────────────

/// Section-fraction at `ru = 1.0`. Every text element starts at
/// `section_units · unit · FONT_MULT` and scales linearly with `ru`.
pub const FONT_MULT: f32 = 0.6;
/// Section-fraction cap. The natural size is clamped at
/// `section_units · unit · FONT_CAP_FRAC` so labels can't bleed
/// out of their assigned band. Saturation point with the default
/// `FONT_MULT` is `FONT_CAP_FRAC / FONT_MULT = 1.5`.
pub const FONT_CAP_FRAC: f32 = 0.9;

/// Min legible pixel sizes per text element — natural × ru is
/// `.max()`'d against these to keep text readable at extreme zoom-
/// outs / tiny windows.
pub const TARGET_MIN_FONT: f32 = 14.0;
pub const PHONEME_MIN_FONT: f32 = 12.0;
pub const SENTENCE_MIN_FONT: f32 = 12.0;
pub const LABEL_MIN_FONT: f32 = 8.0;
pub const HINT_MIN_FONT: f32 = 12.0;

/// Cell-label fonts intentionally render at 2× the section-derived
/// size (multiplier and cap both doubled). The 0.5-unit LABELS
/// section is a visual anchor, not a hard ceiling — labels overflow
/// into the half-unit gaps above and below, which are both empty,
/// so nothing visually collides. Width-overflow on too-long labels
/// is handled at draw time by a measure-then-trim pass.
pub const LABEL_FONT_BOOST: f32 = 2.0;

/// In-hand-gap step-hint glyph height as a fraction of `cell_d`.
/// Sized to fit inside the slot-5 (= cell_d wide) hand_gap with a
/// touch of margin so the glyph never spills into a neighbouring
/// cell.
pub const HINT_FONT_OF_CELL: f32 = 0.65;

// ── Chord row internals ─────────────────────────────────────────

/// Number of slots in the chord row — 10 visible cells plus one
/// invisible "between hands" slot at index 5 so spacing reads
/// uniform end-to-end and the step-hint glyph has a slot to centre
/// on.
pub const SLOT_COUNT: i32 = 11;
/// Slots between adjacent cells (10) — `SLOT_COUNT - 1`.
pub const GAP_COUNT: i32 = SLOT_COUNT - 1;
/// Minimum cell pill size (px). Floors the natural ru-scaled
/// computation.
pub const MIN_CELL_D: i32 = 12;
/// Minimum gap between adjacent cells (px). Used to compute the
/// horizontal cap on `cell_d_max`.
pub const MIN_CELL_GAP: i32 = 2;

/// At `ru = 1.0` each button takes 2/3 of its slot (cell_d:cell_gap
/// = 2:1), giving `row_w = 11·cell_d + 10·(cell_d/2) = 16·cell_d`,
/// so `cell_d = row_w / 16`. This constant is the divisor.
pub const CELL_D_AT_RU1_DIVISOR: f32 = 16.0;

// ── Inter-text spacing ──────────────────────────────────────────

/// Width of a space character, as a fraction of the sentence-line
/// font size.
pub const SENTENCE_SPACE_FRAC: f32 = 0.4;
/// Width of a separator between phoneme glyphs, as a fraction of
/// the phoneme-line font size.
pub const PHONEME_SPACE_FRAC: f32 = 0.5;

// ── User-zoom range ─────────────────────────────────────────────

/// Lower bound for `ru` from Ctrl+− / Ctrl+0 / Ctrl+scroll. Below
/// this the min-font floors take over and ru just stops mattering.
pub const ZOOM_MIN: f32 = 0.3;
/// Upper bound for `ru`. Lined up with the design saturation point
/// (`FONT_CAP_FRAC / FONT_MULT = 1.5`) so Ctrl+= never silently
/// does nothing.
pub const ZOOM_MAX: f32 = 1.5;
/// Multiplicative step per Ctrl+= / Ctrl+− press.
pub const ZOOM_STEP: f32 = 1.1;

// ── Fixed window-fraction sizing ────────────────────────────────

/// Fraction of the primary monitor's width / height the tutor
/// window opens to on first show.
pub const WINDOW_W_FRAC: u32 = 2;
pub const WINDOW_H_FRAC: u32 = 2;

// ── Layout output ───────────────────────────────────────────────

/// Geometry for every element the tutor draws. Computed once per
/// redraw via [`TutorLayout::compute`] from the live `(window_w,
/// window_h, chrome_h, ru)`.
pub struct TutorLayout {
    // Window
    pub bw: i32,
    pub bh: i32,
    pub chrome_h: i32,

    // Top sentence-context line.
    pub sentence_font: f32,
    pub sentence_cy: f32,
    pub sentence_space_w: f32,

    // Big centred drill word.
    pub target_font: f32,
    pub target_cx: f32,
    pub target_cy: f32,

    // Phoneme line for the current target word.
    pub phoneme_font: f32,
    pub phoneme_cy: f32,
    pub phoneme_space_w: f32,

    // Adaptive cell labels — strip above the chord row.
    pub label_font: f32,
    pub label_cy: f32,

    // Chord row (10 finger cells, hand_gap = cell_d wide between).
    pub cell_d: i32,
    pub cell_gap: i32,
    pub hand_gap: i32,
    pub row_x: i32,
    pub row_w: i32,
    pub row_cy: i32,

    // Step-hint glyph rendered in the hand-gap centre between the
    // two finger groups (same Y as the chord row).
    pub hint_font: f32,
    pub hint_cx: f32,
    pub hint_cy: f32,

    // Bottom row: long word bar + 2-cell mod pill on the right.
    pub bottom_cy: i32,
    pub word_cx: i32,
    pub word_w: i32,
    pub mod_cx: i32,
    pub mod_w: i32,
}

impl TutorLayout {
    pub fn compute(window_w: u32, window_h: u32, chrome_h: i32, ru: f32) -> Self {
        let bw = window_w as i32;
        let bh = window_h as i32;
        let span = span(window_w, window_h);
        let cx = bw as f32 / 2.0;

        // ── Vertical sections ────────────────────────────────────
        let total_units = TOP_PAD
            + GAP
            + PARAGRAPH
            + GAP
            + TARGET
            + GAP
            + PHONEME
            + GAP
            + LABELS
            + GAP
            + CHORD_ROW
            + GAP
            + BOTTOM_ROW
            + GAP
            + BOTTOM_PAD;
        let unit = (bh as f32) / total_units;

        // Walk the sections in order, recording each band's centre.
        let mut y = TOP_PAD * unit;
        let advance = |y: &mut f32, w: f32| {
            let top = *y;
            *y += w * unit;
            (top + *y) * 0.5
        };
        let skip_gap = |y: &mut f32| {
            *y += GAP * unit;
        };

        skip_gap(&mut y);
        let sentence_cy = advance(&mut y, PARAGRAPH);
        skip_gap(&mut y);
        let target_cy = advance(&mut y, TARGET);
        skip_gap(&mut y);
        let phoneme_cy = advance(&mut y, PHONEME);
        skip_gap(&mut y);
        // Advance past LABELS to keep row_cy math correct. The actual
        // label_cy is computed below from chord-row top so labels
        // float against the buttons with a tiny gap.
        let _ = advance(&mut y, LABELS);
        skip_gap(&mut y);
        let row_cy_f = advance(&mut y, CHORD_ROW);
        skip_gap(&mut y);
        let bottom_cy_f = advance(&mut y, BOTTOM_ROW);

        // ── Horizontal geometry ──────────────────────────────────
        let h_pad = (H_PAD_UNITS * unit).round() as i32;
        let row_w = (bw - 2 * h_pad).max(48);
        let row_x = h_pad;

        // ── Cell sizing: ru-scaled, clamped both ways ────────────
        // 11 uniform slots (slot 5 is the invisible "between-hands"
        // button so spacing reads identical end-to-end). At ru=1.0
        // the button takes 2/3 of its slot and the gap takes 1/3:
        //   slot = cell_d + cell_gap = (3/2)·cell_d
        //   row_w = SLOT_COUNT·cell_d + GAP_COUNT·cell_gap
        //         = 11·cell_d + 10·(cell_d/2) = 16·cell_d
        // Below ru=1 the buttons shrink and the gaps absorb the
        // slack; above ru=1 we cap at the slot fit / vertical fit.
        let cell_d_natural = ((row_w as f32 / CELL_D_AT_RU1_DIVISOR) * ru).round() as i32;
        let cell_d_h_max = ((row_w - GAP_COUNT * MIN_CELL_GAP) / SLOT_COUNT).max(MIN_CELL_D);
        let cell_d_v_max = ((CHORD_ROW * unit).round() as i32).max(MIN_CELL_D);
        let cell_d_max = cell_d_h_max.min(cell_d_v_max);
        let cell_d = cell_d_natural.clamp(MIN_CELL_D, cell_d_max);
        let cell_gap = ((row_w - SLOT_COUNT * cell_d) / GAP_COUNT).max(MIN_CELL_GAP);
        // hand_gap kept as a name but it's just slot 5's width now,
        // not a separate "extra gap" between hand groups.
        let hand_gap = cell_d;

        let row_cy = row_cy_f.round() as i32;
        let bottom_cy = bottom_cy_f.round() as i32;

        // Bottom-row pills (mod right-aligned, word left-aligned and
        // ending at the right edge of slot 6 — the right-hand inner-
        // index, the rightmost non-finger key. That keeps the word
        // bar visually anchored to "the row except the right-hand
        // fingers" rather than running into the middle finger.
        // Slot k's right edge = row_x + k·(cell_d+cell_gap) + cell_d.
        let mod_w = 2 * cell_d + cell_gap;
        let mod_cx = row_x + row_w - mod_w / 2;
        let word_w = 7 * cell_d + 6 * cell_gap;
        let word_cx = row_x + word_w / 2;

        // ── Fonts ────────────────────────────────────────────────
        let font_for = |section_units: f32, min: f32| -> f32 {
            let natural = section_units * unit * FONT_MULT * ru;
            let cap = section_units * unit * FONT_CAP_FRAC;
            natural.min(cap).max(min)
        };
        let target_font = font_for(TARGET, TARGET_MIN_FONT);
        let phoneme_font = font_for(PHONEME, PHONEME_MIN_FONT);
        let sentence_font = font_for(PARAGRAPH, SENTENCE_MIN_FONT);
        let label_font_natural = LABELS * unit * FONT_MULT * LABEL_FONT_BOOST * ru;
        let label_font_cap = LABELS * unit * FONT_CAP_FRAC * LABEL_FONT_BOOST;
        let label_font = label_font_natural.min(label_font_cap).max(LABEL_MIN_FONT);
        let hint_font = (cell_d as f32 * HINT_FONT_OF_CELL).max(HINT_MIN_FONT);

        let sentence_space_w = sentence_font * SENTENCE_SPACE_FRAC;
        let phoneme_space_w = phoneme_font * PHONEME_SPACE_FRAC;

        // Labels anchor a hairline above the chord-row top — eighth-
        // of-a-GAP of breathing room (so glyphs don't touch the pill
        // edge) but otherwise tight. The LABELS section weight still
        // reserves vertical space in the grid; this just overrides
        // the centre to float against the buttons.
        //
        // text.draw_text_center_u32 anchors at the vertical centre
        // of the line-height box (font · 1.2). Setting
        //   label_cy = row_top − tiny_gap − line_height/2
        // puts the bottom edge of that box `tiny_gap` above the
        // cell pill top.
        let label_tiny_gap = GAP / 8.0 * unit;
        let label_cy =
            (row_cy as f32) - (cell_d as f32) / 2.0 - label_tiny_gap - label_font * 0.6;

        // Hint glyph centres on slot 5 (the invisible 11th button).
        // Slot k's centre = row_x + k·(cell_d + cell_gap) + cell_d/2.
        let hint_cx = (row_x as f32) + 5.0 * (cell_d + cell_gap) as f32 + (cell_d as f32) / 2.0;
        let hint_cy = row_cy_f;

        Self {
            bw,
            bh,
            chrome_h,
            sentence_font,
            sentence_cy,
            sentence_space_w,
            target_font,
            target_cx: cx,
            target_cy,
            phoneme_font,
            phoneme_cy,
            phoneme_space_w,
            label_font,
            label_cy,
            cell_d,
            cell_gap,
            hand_gap,
            row_x,
            row_w,
            row_cy,
            hint_font,
            hint_cx,
            hint_cy,
            bottom_cy,
            word_cx,
            word_w,
            mod_cx,
            mod_w,
        }
    }
}
