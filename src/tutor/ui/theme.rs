//! Single source of truth for every colour the tutor renders. Tweak
//! one constant here and recompile to retheme the whole UI.
//!
//! All colours are u32 in packed ARGB format: 0xAARRGGBB. On Android
//! `fmt()` swaps R↔B at compile time so the same source colour ends
//! up correct on either pipeline.

/// Compile-time colour format conversion for Android (ARGB → ABGR).
#[cfg(target_os = "android")]
const fn fmt(argb: u32) -> u32 {
    let a = (argb >> 24) & 0xFF;
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    (a << 24) | (b << 16) | (g << 8) | r
}

/// Identity on non-Android.
#[cfg(not(target_os = "android"))]
const fn fmt(argb: u32) -> u32 {
    argb
}

// ── Tray icon ────────────────────────────────────────────────────────────

/// Tray icon ring colour while rhe is grabbing input — bright lime.
pub const TRAY_RING_ON: u32 = fmt(0xFF_40_FF_00);
/// Tray icon ring colour while rhe is passing keys through to the OS —
/// dark purple.
pub const TRAY_RING_OFF: u32 = fmt(0xFF_40_00_80);

// ── Tutor canvas ─────────────────────────────────────────────────────────
pub const CANVAS_BG: u32 = fmt(0xFF_20_20_20);

// ── Window chrome ────────────────────────────────────────────────────────

pub const WINDOW_LIGHT_EDGE: u32 = fmt(0xFF_44_41_37);
pub const WINDOW_SHADOW_EDGE: u32 = fmt(0xFF_2B_34_37);
/// Background behind the close / maximise / minimise buttons.
pub const WINDOW_CONTROLS_BG: u32 = fmt(0xFF_1E_1E_1E);
/// Hairline separators between chrome buttons (same hue as the warm
/// window edge).
pub const WINDOW_CONTROLS_HAIRLINE: u32 = fmt(0xFF_44_41_37);

// ── Window-control glyphs ────────────────────────────────────────────────

pub const CLOSE_GLYPH: u32 = fmt(0xFF_80_20_20);
pub const MAXIMIZE_GLYPH: u32 = fmt(0xFF_48_6B_3A);
pub const MAXIMIZE_GLYPH_INTERIOR: u32 = fmt(0xFF_28_2D_2E);
pub const MINIMIZE_GLYPH: u32 = fmt(0xFF_33_30_C7);

// Hover deltas — applied additively on hover, negated on unhover.
// RGB channels wrap; 0xFF alpha absorbs carry from R overflow.
pub const CLOSE_HOVER: u32 = fmt(0xFF_21_FD_F9);
pub const MAXIMIZE_HOVER: u32 = fmt(0xFF_FA_10_FA);
pub const MINIMIZE_HOVER: u32 = fmt(0xFF_F7_FA_25);

// ── Chord-row cells (per-finger gradient + idle / press states) ──────────

/// Per-cell primary highlight. Cyan on the left hand fades to yellow
/// on the right so the user picks out finger position peripherally.
/// Index order matches the row layout (L pinky → R pinky); cells 4
/// and 5 are the inner-index keys.
pub const KEY_COLOURS: [u32; 10] = [
    fmt(0xFF_60_A8_F0), // L pinky
    fmt(0xFF_70_A8_E0), // L ring
    fmt(0xFF_80_A8_D0), // L middle
    fmt(0xFF_90_A8_C0), // L idx-outer
    fmt(0xFF_A0_A8_B0), // L idx-inner
    fmt(0xFF_B0_A8_A0), // R idx-inner
    fmt(0xFF_C0_A8_90), // R idx-outer
    fmt(0xFF_D0_A8_80), // R middle
    fmt(0xFF_E0_A8_70), // R ring
    fmt(0xFF_F0_A8_60), // R pinky
];

/// Half-brightness companion to `KEY_COLOURS`. Used for ordered-brief
/// secondary targets — chord cells that aren't (or aren't yet) the
/// locked-in lead finger.
pub const DOT_COLOURS: [u32; 10] = [
    fmt(0xFF_30_54_78),
    fmt(0xFF_38_54_70),
    fmt(0xFF_40_54_68),
    fmt(0xFF_48_54_60),
    fmt(0xFF_50_54_58),
    fmt(0xFF_58_54_50),
    fmt(0xFF_60_54_48),
    fmt(0xFF_68_54_40),
    fmt(0xFF_70_54_38),
    fmt(0xFF_78_54_30),
];

/// Idle / non-target / errored fill for the eight resting-finger
/// cells. Dark grey: present enough to anchor the row, dim enough to
/// fade behind any active target.
pub const CELL_IDLE: u32 = fmt(0xFF_30_30_38);
/// Idle fill for the two inner-index cells (idx 4, 5). Near-black so
/// they visually drop out when not in play — they're never resting-
/// finger keys, only used for number / symbol modes.
pub const CELL_INNER_IDLE: u32 = fmt(0xFF_06_06_08);

/// Word-mode bar (left thumb): purple when an active target,
/// half-brightness for ordered-brief secondary.
pub const WORD_PRIMARY: u32 = fmt(0xFF_80_00_FF);
pub const WORD_SECONDARY: u32 = fmt(0xFF_40_00_80);
/// Mod-key bar (right thumb): green when an active target,
/// half-brightness for secondary.
pub const MOD_PRIMARY: u32 = fmt(0xFF_00_FF_00);
pub const MOD_SECONDARY: u32 = fmt(0xFF_00_7F_00);

// ── Cell bevel arithmetic ────────────────────────────────────────────────

/// LSB strip mask. AND a colour with this before adding/subtracting a
/// delta so the bottom bit of each channel can't carry into the next
/// channel during wrap. Required because `wrapping_add` / `wrapping_sub`
/// treat the u32 as a single integer, not four packed channels.
pub const COLOUR_LSB_MASK: u32 = 0xFEFE_FEFE;
/// Bevel highlight / shadow magnitude. Added to the cell fill for the
/// raised top-left edge, subtracted for the bottom-right shadow. The
/// edges swap when the cell is drawn pressed.
pub const BEVEL_DELTA: u32 = 0x0020_2020;
/// Extra darken applied to a pressed grey idle cell so the inverted
/// bevel reads against a slightly different surface.
pub const BEVEL_PRESS_DARKEN: u32 = 0x0010_1018;

// ── Tutor text ───────────────────────────────────────────────────────────

/// Sentence-context line: current word stays bright white + bold; past
/// words dim to a muted grey-purple, future words sit at medium grey.
pub const SENTENCE_CURRENT: u32 = fmt(0xFF_FF_FF_FF);
pub const SENTENCE_PAST: u32 = fmt(0xFF_60_60_70);
pub const SENTENCE_FUTURE: u32 = fmt(0xFF_B0_B0_C0);

/// Big centred drill word above the chord row.
pub const TARGET_WORD: u32 = fmt(0xFF_E0_E0_F0);
/// Step hint below the target word (IPA / digit / mode glyph).
pub const STEP_HINT: u32 = fmt(0xFF_B0_B0_C0);
/// Adaptive label centred in each chord cell.
pub const CELL_LABEL: u32 = fmt(0xFF_E8_E8_F0);

// ── Generic UI palette (kept for future widgets) ─────────────────────────

pub const LIGHT_EDGE: u32 = fmt(0xFF_60_60_60);
pub const SHADOW_EDGE: u32 = fmt(0xFF_20_20_20);
pub const FILL: u32 = fmt(0xFF_40_40_40);

pub const LABEL_COLOUR: u32 = fmt(0xFF_80_80_80);
pub const TEXT_COLOUR: u32 = fmt(0xFF_D0_D0_D0);
pub const TEXT_SELECTION_COLOUR: u32 = fmt(0xFF_D0_D0_D0);

// Generic button palette.
pub const BUTTON_BASE: u32 = fmt(0xFF_40_40_40);
pub const BUTTON_LIGHT_EDGE: u32 = fmt(0xFF_60_60_60);
pub const BUTTON_SHADOW_EDGE: u32 = fmt(0xFF_20_20_20);
pub const BUTTON_HAIRLINE: u32 = fmt(0xFF_32_32_32);
pub const BUTTON_BLUE: u32 = fmt(0xFF_20_30_50);
pub const BUTTON_GREEN: u32 = fmt(0xFF_20_45_25);
pub const BUTTON_YELLOW: u32 = fmt(0xFF_50_45_20);
pub const BUTTON_TEXT: u32 = fmt(0xFF_D0_D0_D0);

// Hover deltas for generic surfaces (additive; negated on unhover).
pub const TEXTBOX_HOVER: u32 = fmt(0x00_0A_0A_0A);
pub const QUERY_BUTTON_HOVER: u32 = fmt(0x00_0F_0F_0F);
pub const BACK_HEADER_HOVER: u32 = fmt(0x00_0C_0C_0C);

// Textbox surface.
pub const TEXTBOX_LIGHT_EDGE: u32 = fmt(0xFF_44_41_37);
pub const TEXTBOX_SHADOW_EDGE: u32 = fmt(0xFF_2B_34_37);
pub const TEXTBOX_FILL: u32 = fmt(0xFF_06_08_09);

// Textbox glow (0x00RRGGBB — alpha applied separately at blend time).
pub const GLOW_DEFAULT: u32 = fmt(0x00_FF_FF_FF);
pub const GLOW_ATTESTING: u32 = fmt(0x00_FF_FF_40);
pub const GLOW_SUCCESS: u32 = fmt(0x00_40_FF_40);
pub const GLOW_ERROR: u32 = fmt(0x00_FF_60_60);

// Status / counter / hint text states.
pub const COUNTER_TEXT: u32 = fmt(0xFF_FF_FF_FF);
pub const PLACEHOLDER_TEXT: u32 = fmt(0xFF_80_80_80);
pub const STATUS_TEXT_ATTESTING: u32 = fmt(0xFF_FF_FF_00);
pub const STATUS_TEXT_ERROR: u32 = fmt(0xFF_FF_00_00);
pub const ZOOM_HINT_TEXT: u32 = fmt(0xFF_80_80_80);

/// Magenta debug marker. Alpha 0xFE so it never accidentally matches
/// real opaque content during pixel comparisons.
pub const DEBUG_MARKER: u32 = fmt(0xFE_FF_00_FF);

// Background texture (noise generator inputs, if/when we add one).
pub const BG_BASE: u32 = fmt(0xFF_0C_14_0E);
pub const BG_MASK: u32 = fmt(0xFF_0F_07_1F);
pub const BG_ALPHA: u32 = fmt(0xFF_00_00_00);
pub const BG_SPECKLE: u32 = fmt(0x00_3F_1F_7F);

// ── Fonts ────────────────────────────────────────────────────────────────

pub const FONT_LOGO: &str = "Oxanium";
pub const FONT_UI: &str = "Josefin Slab";
pub const FONT_USER_CONTENT: &str = "Open Sans";
pub const FONT_WEIGHT_USER_CONTENT: u16 = 400;
