//! Cross-platform tray icon + right-click menu + on-demand tutor window.
//!
//! macOS gets its native menu bar icon via `tray-icon` (which wraps
//! `NSStatusItem`); tray events flow naturally through the NSApp
//! event loop that winit already owns on the main thread.
//!
//! Linux gets a StatusNotifierItem registered via DBus — works out of
//! the box on KDE, XFCE, Cinnamon, MATE; on GNOME the user needs
//! `gnome-shell-extension-appindicator` installed and enabled. The
//! icon crate requires `gtk::init` + a running `gtk::main` loop for
//! menu callbacks to fire, and winit doesn't pump gtk. So on Linux we
//! spawn a dedicated gtk thread that owns the tray + menu items and
//! runs `gtk::main()`; the main thread keeps running winit for the
//! tutor window. Menu clicks flow via `tray-icon`'s `MenuEvent`
//! receiver → `EventLoopProxy` → winit `UserEvent`; menu-item labels
//! and icon state are kept in sync by a glib timeout on the gtk
//! thread that polls the shared atomics.

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::chord_map::{BriefTable, PhonemeTable};
use crate::hand::KeyEvent as RheKeyEvent;
use crate::interpreter::FallbackMode;
use crate::tutor::drill::{TutorState, build_practice, cell_label, key_state_to_mask};
use crate::tutor::ui::compositor::{
    HIT_CLOSE_BUTTON, HIT_MAXIMIZE_BUTTON, HIT_MINIMIZE_BUTTON, HIT_NONE, TutorApp,
};
use crate::tutor::ui::renderer::Renderer;
use crate::tutor::ui::text_rasterizing::TextRenderer;
use crate::tutor::wiki::SentenceStream;
use crate::word_lookup::WordLookup;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Rendered tray-icon bitmap dimensions. 44 = 2× the conventional
/// 22-px Linux SNI / 22-pt macOS NSStatusItem size, so retina /
/// HiDPI displays get an exact pixel match and 1× displays
/// downscale cleanly. Tray-icon 0.22 takes a single bitmap; the
/// OS handles further scaling.
const ICON_SIZE: u32 = 44;

/// Bright lime when rhe is capturing input.
const ICON_RING_ON: u32 = 0xFF40_FF40;
/// Dark purple when rhe is passing keys through to the OS.
const ICON_RING_OFF: u32 = 0xFF4A_1A72;

/// Logo embedded at compile time. 1024×1024 RGB PNG.
const LOGO_PNG_BYTES: &[u8] = include_bytes!("../logo.png");

/// Decode and scale-down logo.png to a square RGB buffer of `diameter`×
/// `diameter`. Cached after the first call. Nearest-neighbour scaling —
/// fine for tray-icon sizes where per-pixel sharpness matters more than
/// filter fidelity.
fn scaled_logo_rgb(diameter: usize) -> Vec<u8> {
    use std::sync::OnceLock;
    static DECODED: OnceLock<(Vec<u8>, usize, usize)> = OnceLock::new();
    let (rgb, src_w, src_h) = DECODED.get_or_init(|| {
        let decoder = png::Decoder::new(LOGO_PNG_BYTES);
        let mut reader = decoder.read_info().expect("logo.png invalid");
        let info = reader.info();
        let w = info.width as usize;
        let h = info.height as usize;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).expect("logo.png decode");
        let color = frame.color_type;
        // Normalise to RGB regardless of source colour type.
        let rgb = match color {
            png::ColorType::Rgb => buf,
            png::ColorType::Rgba => buf
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect(),
            png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g]).collect(),
            png::ColorType::GrayscaleAlpha => buf
                .chunks_exact(2)
                .flat_map(|p| [p[0], p[0], p[0]])
                .collect(),
            png::ColorType::Indexed => panic!("indexed PNG not supported"),
        };
        (rgb, w, h)
    });

    let mut dst = vec![0u8; diameter * diameter * 3];
    for dy in 0..diameter {
        let sy = dy * src_h / diameter;
        for dx in 0..diameter {
            let sx = dx * src_w / diameter;
            let src_idx = (sy * src_w + sx) * 3;
            let dst_idx = (dy * diameter + dx) * 3;
            dst[dst_idx] = rgb[src_idx];
            dst[dst_idx + 1] = rgb[src_idx + 1];
            dst[dst_idx + 2] = rgb[src_idx + 2];
        }
    }
    dst
}

/// Per-cell primary highlight colour. Cyan-leaning on the left hand
/// gradients into yellow-leaning on the right so the user can pick
/// out which finger a target belongs to even peripherally. Indexed
/// in display order (L pinky → R pinky); cells 4 and 5 are the
/// inner-index keys.
const KEY_COLORS: [u32; 10] = [
    0xFF60_A8F0, // L pinky
    0xFF70_A8E0, // L ring
    0xFF80_A8D0, // L middle
    0xFF90_A8C0, // L idx-outer
    0xFFA0_A8B0, // L idx-inner
    0xFFB0_A8A0, // R idx-inner
    0xFFC0_A890, // R idx-outer
    0xFFD0_A880, // R middle
    0xFFE0_A870, // R ring
    0xFFF0_A860, // R pinky
];

/// Half-brightness companion to `KEY_COLORS`. Used for ordered-brief
/// secondary targets — cells that are part of the chord but aren't
/// (or aren't yet) the locked-in lead finger. Picking a non-primary
/// cell as the lead would resolve to a different word.
const DOT_COLORS: [u32; 10] = [
    0xFF30_5478,
    0xFF38_5470,
    0xFF40_5468,
    0xFF48_5460,
    0xFF50_5458,
    0xFF58_5450,
    0xFF60_5448,
    0xFF68_5440,
    0xFF70_5438,
    0xFF78_5430,
];

/// Idle / non-target / errored fill for the resting-finger cells
/// (everything except the two inner-index keys). Dark grey: present
/// enough to anchor the row, dim enough to fade behind any active
/// target.
const IDLE_FILL: u32 = 0xFF30_3038;
/// Idle fill for the two inner-index cells (idx 4, 5). Near-black so
/// they visually drop out when not in play — they're never resting-
/// finger keys, only used for number / future symbol modes, so off
/// is the default state and we don't want them competing with the
/// home row for attention.
const INNER_IDLE_FILL: u32 = 0xFF06_0608;

/// Word / mod target highlight colours. Word stays purple, mod stays
/// green — same scheme as the old terminal renderer. Halved variants
/// are used when the target is "secondary" (mod present in a chord
/// alongside fingers, where mod is reachable but not the lead).
const WORD_PRIMARY: u32 = 0xFF80_00FF;
const WORD_SECONDARY: u32 = 0xFF40_0080;
const MOD_PRIMARY: u32 = 0xFF00_FF00;
const MOD_SECONDARY: u32 = 0xFF00_7F00;

/// Look up the cell fill: primary key colour for primary targets,
/// dot colour for secondary, idle for everything else (including
/// errored frames where the caller zeroes the target out). Inner-
/// index cells (4, 5) idle to near-black; resting fingers idle to
/// dark grey, with `pressed` darkening the grey a touch so the
/// pressed bevel reads against a slightly different surface.
fn finger_cell_fill(cell_idx: usize, is_target: bool, is_primary: bool, pressed: bool) -> u32 {
    if !is_target {
        let inner = cell_idx == 4 || cell_idx == 5;
        let base = if inner { INNER_IDLE_FILL } else { IDLE_FILL };
        if pressed && !inner {
            return (base & 0xFEFE_FEFE).wrapping_sub(0x0010_1018);
        }
        return base;
    }
    if is_primary {
        KEY_COLORS[cell_idx]
    } else {
        DOT_COLORS[cell_idx]
    }
}

/// Pill-shaped cell via the compositor's draw_button. Square: width =
/// height = `cell_d`. `pressed` swaps the bevel highlight/shadow so
/// the cell reads as pushed-in instead of raised — mirrors the
/// physical key state for live feedback.
fn cell(
    pixels: &mut [u32],
    hit: &mut [u8],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    cell_d: i32,
    fill: u32,
    pressed: bool,
) {
    if cx < cell_d / 2 + 1 || cy < cell_d / 2 + 1 {
        return;
    }
    let light = (fill & 0xFEFE_FEFE).wrapping_add(0x0020_2020);
    let shadow = (fill & 0xFEFE_FEFE).wrapping_sub(0x0020_2020);
    let (top_edge, bot_edge) = if pressed { (shadow, light) } else { (light, shadow) };
    crate::tutor::ui::compositor::TutorApp::draw_button(
        pixels,
        hit,
        None,
        w,
        h,
        cx as usize,
        cy as usize,
        cell_d as usize,
        cell_d as usize,
        crate::tutor::ui::compositor::HIT_NONE,
        fill,
        top_edge,
        bot_edge,
    );
}

/// Pill-shaped cell, allowing distinct width/height (used for the
/// thumb / mod cell which is rendered wider). `pressed` flips the
/// bevels same as `cell`.
fn cell_wide(
    pixels: &mut [u32],
    hit: &mut [u8],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    cell_w: i32,
    cell_h: i32,
    fill: u32,
    pressed: bool,
) {
    if cx < cell_w / 2 + 1 || cy < cell_h / 2 + 1 {
        return;
    }
    let light = (fill & 0xFEFE_FEFE).wrapping_add(0x0020_2020);
    let shadow = (fill & 0xFEFE_FEFE).wrapping_sub(0x0020_2020);
    let (top_edge, bot_edge) = if pressed { (shadow, light) } else { (light, shadow) };
    crate::tutor::ui::compositor::TutorApp::draw_button(
        pixels,
        hit,
        None,
        w,
        h,
        cx as usize,
        cy as usize,
        cell_w as usize,
        cell_h as usize,
        crate::tutor::ui::compositor::HIT_NONE,
        fill,
        top_edge,
        bot_edge,
    );
}

/// Render the tray icon: ring around a scaled-down logo.png via the
/// compositor's `draw_avatar` in straight-alpha mode (so the outer AA
/// fringe carries real alpha and the icon fades to transparent
/// outside the ring instead of leaving an opaque half-bright ring).
fn make_tray_icon(size: u32, online: bool) -> Icon {
    use crate::tutor::ui::compositor::TutorApp;

    let w = size as usize;
    let h = size as usize;

    // -2, not -1: with cx = size/2 on an even canvas the solid ring
    // would otherwise extend one pixel past the canvas edge on the
    // bottom-right (parity quirk). Subtracting 2 puts the last solid
    // pixel exactly at the edge and leaves the alpha-0 outer-AA off-
    // canvas where it isn't visible.
    let radius = (size as isize) / 2 - 2;
    let diameter = (radius * 2) as usize;
    let cx = (size / 2) as isize;
    let cy = cx;

    let mut pixels = vec![0u32; w * h];
    let logo = scaled_logo_rgb(diameter);
    let ring = if online { ICON_RING_ON } else { ICON_RING_OFF };
    TutorApp::draw_avatar(
        &mut pixels,
        None,
        w,
        h,
        cx,
        cy,
        radius,
        Some(&logo),
        Some(ring),
        false,
        true, // straight_alpha — tray icons need real edge alpha
    );

    // u32 ARGB → RGBA bytes for tray-icon's Icon::from_rgba.
    let mut rgba = Vec::with_capacity(w * h * 4);
    for px in pixels {
        rgba.push(((px >> 16) & 0xFF) as u8);
        rgba.push(((px >> 8) & 0xFF) as u8);
        rgba.push((px & 0xFF) as u8);
        rgba.push(((px >> 24) & 0xFF) as u8);
    }
    Icon::from_rgba(rgba, size, size).expect("tray icon build failed")
}

/// Events that wake the winit event loop.
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// The engine thread toggled the `enabled` flag (via caps tap).
    /// The tray icon + check item need refreshing. On Linux the gtk
    /// thread's glib timeout notices this via atomics directly, so
    /// this variant is just a no-op wake for the winit thread.
    StateChanged,
    /// A menu item was clicked.
    Menu(MenuId),
    /// The engine thread observed a key event. Forwarded to the tutor
    /// window if it's open so the drill state machine can advance.
    /// Sent unconditionally — the tray drops it on the floor when no
    /// tutor window exists.
    DrillKey(RheKeyEvent),
}

/// Handle a tray can pass to other threads so they can wake the event loop.
pub type TrayProxy = EventLoopProxy<TrayEvent>;

/// Generate a filled circle icon as RGBA bytes.
fn circle_icon(r: u8, g: u8, b: u8, a: u8) -> Icon {
    let mut rgba = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let center = ICON_SIZE as f64 / 2.0;
    let radius = center - 1.0;

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                let offset = ((y * ICON_SIZE + x) * 4) as usize;
                rgba[offset] = r;
                rgba[offset + 1] = g;
                rgba[offset + 2] = b;
                rgba[offset + 3] = a;
            }
        }
    }

    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("failed to create icon")
}

/// Build the event loop + proxy before spawning the engine. The caller
/// gives the proxy to any thread that wants to notify the tray of state
/// changes (primarily the evdev reader on Linux / HID callback on macOS).
pub fn build() -> (EventLoop<TrayEvent>, TrayProxy) {
    let event_loop = EventLoop::<TrayEvent>::with_user_event()
        .build()
        .expect("failed to build winit event loop");
    let proxy = event_loop.create_proxy();
    (event_loop, proxy)
}

/// Menu IDs exposed to the winit thread so it can dispatch clicks back
/// to the right action without holding the `MenuItem`s directly (which
/// would be a problem on Linux, where the items must live on the gtk
/// thread).
#[derive(Clone)]
struct TrayIds {
    tutor: MenuId,
    mode: MenuId,
    enabled: MenuId,
    quit: MenuId,
}

struct TrayApp {
    enabled: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    fallback: Arc<AtomicU8>,
    /// Bitfield mirroring the live Interpreter's sub-mode (number /
    /// future symbol etc.). Cloned from the engine thread so the
    /// tutor's adaptive cell labels match what the next press would
    /// actually emit. See `interpreter::MODE_FLAG_*`.
    mode_flags: Arc<AtomicU8>,
    ids: TrayIds,

    // macOS keeps the tray + menu items on the winit thread; Linux
    // hands them off to the gtk thread. On Linux these stay None.
    #[cfg(target_os = "macos")]
    mac_state: Option<MacTrayState>,

    // Tutor window state. Declared renderer-before-window so they drop
    // in that order (renderer holds `&'static Window` transmuted from
    // the window below, and must be dropped first to avoid dangling).
    tutor_renderer: Option<Renderer>,
    tutor_window: Option<Window>,
    tutor_state: Option<TutorState>,
    text_renderer: Option<TextRenderer>,

    // Cached drill resources, kept alive between word transitions so a
    // wiki wraparound can rebuild Practice without re-parsing cmudict /
    // briefs. Built lazily on first `open_tutor`, dropped on close.
    tutor_wiki_stream: Option<SentenceStream>,
    tutor_word_lookup: Option<WordLookup>,
    tutor_brief_table: Option<BriefTable>,

    /// User zoom multiplier applied on top of span-derived sizes.
    /// Adjusted live by Ctrl+scroll; 1.0 is the default.
    tutor_ru: f32,
    /// Last observed modifier state for the tutor window. Updated on
    /// every ModifiersChanged event so MouseWheel can consult it.
    tutor_mods: ModifiersState,
    /// Last cursor position inside the tutor window, in physical px.
    tutor_cursor: PhysicalPosition<f64>,
    /// Hit-test map, one byte per pixel. Photon's draw_* fns populate
    /// this with HIT_* constants; we consult it on click to dispatch
    /// to the right action.
    tutor_hit_test: Vec<u8>,
    /// Textbox alpha mask (0 = outside textbox, 255 = inside, AA edge
    /// values in between). Populated by draw_textbox, consumed by
    /// apply_textbox_glow. Sized to pixels.len().
    tutor_textbox_mask: Vec<u8>,

    // Debug toggles (photon parity): Ctrl+D counters, Ctrl+H hitmap
    // overlay, Ctrl+T textbox-mask overlay.
    tutor_debug: bool,
    tutor_debug_hit_test: bool,
    tutor_show_textbox_mask: bool,
    tutor_debug_hit_colours: Vec<(u8, u8, u8)>,
    /// Manual Ctrl tracking derived from KeyboardInput events. Used as
    /// a fallback when winit's ModifiersChanged doesn't fire on some
    /// Wayland compositors (or before the window has keyboard focus).
    tutor_ctrl_held: bool,

    // Frame/redraw counters for the debug HUD.
    tutor_frame_counter: u64,
    tutor_redraw_counter: u64,
}

#[cfg(target_os = "macos")]
struct MacTrayState {
    tray: TrayIcon,
    icon_on: Icon,
    icon_off: Icon,
    mode_item: MenuItem,
    enabled_item: MenuItem,
}

impl TrayApp {
    #[cfg(target_os = "macos")]
    fn refresh_enabled_ui(&mut self) {
        if let Some(state) = self.mac_state.as_mut() {
            let on = self.enabled.load(Ordering::Relaxed);
            state
                .enabled_item
                .set_text(if on { "rhe" } else { "keyboard" });
            let icon = if on {
                state.icon_on.clone()
            } else {
                state.icon_off.clone()
            };
            state.tray.set_icon(Some(icon)).ok();
        }
    }

    #[cfg(target_os = "linux")]
    fn refresh_enabled_ui(&mut self) {
        // No-op on Linux: the gtk thread's glib timeout owns the UI.
    }

    fn open_tutor(&mut self, event_loop: &ActiveEventLoop) {
        if self.tutor_window.is_some() {
            if let Some(w) = &self.tutor_window {
                w.focus_window();
            }
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("rhe tutor")
            .with_inner_size(PhysicalSize::new(800u32, 500u32))
            .with_decorations(false)
            .with_transparent(true);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("rhe: failed to create tutor window: {e}");
                return;
            }
        };

        let size = window.inner_size();
        self.tutor_window = Some(window);

        // Safe only because:
        //   1. tutor_renderer is declared before tutor_window in the
        //      struct, so it drops first.
        //   2. We never move tutor_window out of self except to drop it
        //      (close path assigns None; renderer is cleared first).
        if let Some(w_ref) = self.tutor_window.as_ref() {
            self.tutor_renderer = Some(Renderer::new(w_ref, size.width, size.height));
            // Explicit focus request — needed on some compositors
            // (Wayland in particular) to start delivering
            // ModifiersChanged / KeyboardInput events without an
            // intermediate user click.
            w_ref.focus_window();
            w_ref.request_redraw();
        }

        // Build the drill state on first open. Wiki stream blocks for
        // its first article (then prefetches the next in the
        // background); falls back to bundled Alice text when the
        // network isn't reachable.
        if self.tutor_state.is_none() {
            let cmudict = crate::data::load_cmudict();
            let lookup = WordLookup::new(&cmudict);
            let brief_table = crate::briefs::load_briefs();

            let stream = SentenceStream::new();
            let initial = stream.initial();
            let lines: Vec<String> = if initial.is_empty() {
                crate::tutor::drill::ALICE_FALLBACK
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                initial
            };
            let practice = build_practice(&lookup, &brief_table, lines, false);
            self.tutor_state = Some(TutorState::new(practice));
            self.tutor_wiki_stream = Some(stream);
            self.tutor_word_lookup = Some(lookup);
            self.tutor_brief_table = Some(brief_table);
        }
    }

    fn close_tutor(&mut self) {
        // Order matters: renderer first (holds &'static Window), window second.
        self.tutor_renderer = None;
        self.tutor_window = None;
        // Drop drill state too — it'll get rebuilt fresh next open.
        self.tutor_state = None;
        self.tutor_wiki_stream = None;
        self.tutor_word_lookup = None;
        self.tutor_brief_table = None;
    }

    /// On wraparound, swap the drill to the next prefetched wiki batch
    /// if one is ready. Otherwise just clear the flag — the same batch
    /// loops, and the next wrap retries the prefetch.
    fn maybe_swap_practice(&mut self) {
        let Some(state) = self.tutor_state.as_mut() else {
            return;
        };
        if !state.practice.wrapped {
            return;
        }
        if let (Some(stream), Some(lookup), Some(brief_table)) = (
            self.tutor_wiki_stream.as_ref(),
            self.tutor_word_lookup.as_ref(),
            self.tutor_brief_table.as_ref(),
        ) {
            if let Some(new_lines) = stream.try_next() {
                state.practice = build_practice(lookup, brief_table, new_lines, false);
                return;
            }
        }
        state.practice.wrapped = false;
    }

    fn ensure_buffers(&mut self, size_px: usize) {
        if self.tutor_hit_test.len() != size_px {
            self.tutor_hit_test.resize(size_px, HIT_NONE);
        }
        if self.tutor_textbox_mask.len() != size_px {
            self.tutor_textbox_mask.resize(size_px, 0);
        }
    }

    fn redraw_tutor(&mut self) {
        let Some(window) = self.tutor_window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        let width = size.width as usize;
        let height = size.height as usize;
        if width == 0 || height == 0 {
            return;
        }

        if self.text_renderer.is_none() {
            self.text_renderer = Some(TextRenderer::new());
        }

        self.ensure_buffers(width * height);

        let Some(renderer) = self.tutor_renderer.as_mut() else {
            return;
        };

        // CRITICAL: mark the whole frame dirty on the Renderer BEFORE
        // locking the buffer. Photon's Linux softbuffer path only
        // copies rows in the current dirty range to the compositor
        // buffer on present(); after the first frame that range is
        // empty unless we re-mark, so every subsequent redraw would
        // otherwise silently present zero bytes and the screen would
        // freeze at the initial frame. The `buf.mark_all()` method on
        // the SoftbufferBuffer guard is a stub — it has to be called
        // on the Renderer itself.
        renderer.mark_all();

        let mut buf = renderer.lock_buffer();
        let pixels = buf.as_mut();

        // Clear background + hit map every frame. Transparent (0) lets
        // photon's edges-and-mask pass clear the squircle corners to
        // fully transparent — the compositor then shows whatever's
        // behind the window.
        pixels.fill(0xFF10_1016);
        self.tutor_hit_test.fill(HIT_NONE);
        self.tutor_textbox_mask.fill(0);

        // Photon's chrome. window_controls returns the squircle bounds
        // that edges_and_mask + button_hairlines both need.
        let (start, crossings, button_x_start, button_height) = TutorApp::draw_window_controls(
            pixels,
            &mut self.tutor_hit_test,
            size.width,
            size.height,
            self.tutor_ru,
        );
        TutorApp::draw_window_edges_and_mask(
            pixels,
            &mut self.tutor_hit_test,
            size.width,
            size.height,
            start,
            &crossings,
        );
        TutorApp::draw_button_hairlines(
            pixels,
            &mut self.tutor_hit_test,
            size.width,
            size.height,
            button_x_start,
            button_height,
            start,
            &crossings,
        );

        // ── Drill view ─────────────────────────────────────────────
        // Layout: target word (big, centered, Oxanium Bold) at top of
        // the content area, step hint below (small, Josefin Slab,
        // either IPA / Autospell letters or a number-mode glyph),
        // then a row of keyboard cells reflecting the current target
        // and live key state.
        let span = crate::tutor::ui::span(size.width, size.height);
        let ru = self.tutor_ru;
        let chrome_h = button_height as i32;
        let bw = size.width as i32;
        let bh = size.height as i32;

        let word_text = self
            .tutor_state
            .as_ref()
            .and_then(|s| s.practice.current_word())
            .map(|w| w.word.clone())
            .unwrap_or_default();
        let step_hint = self
            .tutor_state
            .as_ref()
            .and_then(|s| s.practice.current_step())
            .map(|step| {
                if let Some(g) = &step.number_glyph {
                    g.clone()
                } else if let Some(p) = step.phoneme {
                    p.to_ipa().to_string()
                } else if step.space_only {
                    "·".to_string()
                } else if step.mod_tap_only {
                    "#".to_string()
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        let errored = self.tutor_state.as_ref().map(|s| s.errored).unwrap_or(false);

        // Sentence context: the full current sentence rendered as a
        // single line, anchored so the current word stays centered.
        // As word_idx advances the line shifts left, sliding completed
        // words off and bringing upcoming words in.
        let sentence_words: Vec<String> = self
            .tutor_state
            .as_ref()
            .and_then(|s| s.practice.sentences.get(s.practice.sentence_idx))
            .map(|s| s.iter().map(|w| w.word.clone()).collect())
            .unwrap_or_default();
        let cur_word_idx = self
            .tutor_state
            .as_ref()
            .map(|s| s.practice.word_idx)
            .unwrap_or(0);

        if let Some(text) = self.text_renderer.as_mut() {
            let cx = bw as f32 / 2.0;
            let word_font = (span * ru / 5.0).max(24.0);
            let cy = chrome_h as f32 + word_font;

            // Sentence line above the big word.
            if !sentence_words.is_empty() {
                let line_font = (span * ru / 18.0).max(14.0);
                let space_w = line_font * 0.4;
                // Pass 1: measure each word by drawing it off-screen and
                // capturing the returned width. Cosmic-text's bounds
                // check skips every pixel write at far-negative x, so
                // this is effectively a measure.
                let widths: Vec<f32> = sentence_words
                    .iter()
                    .map(|w| {
                        text.draw_text_left_u32(
                            pixels,
                            width,
                            w,
                            -1.0e6,
                            -1.0e6,
                            line_font,
                            400,
                            0,
                            "Oxanium",
                        )
                    })
                    .collect();
                let mut left_w = 0.0f32;
                for i in 0..cur_word_idx.min(widths.len()) {
                    left_w += widths[i] + space_w;
                }
                let cur_w = widths.get(cur_word_idx).copied().unwrap_or(0.0);
                let line_x = cx - (left_w + cur_w / 2.0);
                let line_y = chrome_h as f32 + line_font * 1.2;
                let mut x = line_x;
                for (i, w) in sentence_words.iter().enumerate() {
                    let (colour, weight) = if i == cur_word_idx {
                        (0xFFFF_FFFF, 700)
                    } else if i < cur_word_idx {
                        (0xFF60_6070, 400)
                    } else {
                        (0xFFB0_B0C0, 400)
                    };
                    text.draw_text_left_u32(
                        pixels,
                        width,
                        w,
                        x,
                        line_y,
                        line_font,
                        weight,
                        colour,
                        "Oxanium",
                    );
                    x += widths.get(i).copied().unwrap_or(0.0) + space_w;
                }
            }

            // Big centred target word. No errored colour swap — the
            // cue lives entirely in the keyboard row going dark.
            let colour = 0xFFE0_E0F0;
            text.draw_text_center_u32(
                pixels,
                width,
                &word_text,
                cx,
                cy + word_font * 0.6,
                word_font,
                700,
                colour,
                "Oxanium",
            );

            if !step_hint.is_empty() {
                let hint_font = (span * ru / 14.0).max(12.0);
                let hint_cy = cy + word_font * 1.5;
                text.draw_text_center_u32(
                    pixels,
                    width,
                    &step_hint,
                    cx,
                    hint_cy,
                    hint_font,
                    400,
                    0xFFB0B0C0,
                    "Josefin Slab",
                );
            }
        }

        // Keyboard diagram: 10 home/inner cells in a row + thumb cell
        // below the row's centre. Cell colour signals (target,
        // pressed, errored) state.
        //
        // Adaptive label per cell: shows what the cell's key would emit
        // as part of the currently-held chord. Labels are computed
        // ahead of cell rendering so the immutable tutor_state borrow
        // doesn't conflict with the mutable text_renderer borrow that
        // comes after.
        let label_data: Option<(_, [String; 10], _, _, _)> =
            if let Some(state) = self.tutor_state.as_ref() {
                let target = state
                    .practice
                    .current_target()
                    .copied()
                    .unwrap_or_default();
                let key_state = state.key_state.clone();
                let held_mask = key_state_to_mask(&key_state);
                let held_word = key_state.word;
                let first_down = state.tutor_first_down;
                // Live mode bits straight from the engine's
                // Interpreter (single relaxed load on a shared
                // atomic). Always coherent with what the next press
                // would actually emit, regardless of where the drill
                // happens to be. has_number_context flips L-hand
                // brief cells to number-form labels (alt suffix
                // meaning when a pure integer is one slot back).
                let mode_bits = self.mode_flags.load(Ordering::Relaxed);
                let in_number_mode = mode_bits & crate::interpreter::MODE_FLAG_NUMBER != 0;
                let has_number_context =
                    mode_bits & crate::interpreter::MODE_FLAG_HAS_NUMBER != 0;
                let phonemes = PhonemeTable::new();
                let briefs = self.tutor_brief_table.as_ref();
                const CELL_SCANS: [u8; 10] = [
                    crate::scan::L_PINKY,
                    crate::scan::L_RING,
                    crate::scan::L_MID,
                    crate::scan::L_IDX,
                    crate::scan::L_IDX_INNER,
                    crate::scan::R_IDX_INNER,
                    crate::scan::R_IDX,
                    crate::scan::R_MID,
                    crate::scan::R_RING,
                    crate::scan::R_PINKY,
                ];
                let labels: [String; 10] = std::array::from_fn(|i| {
                    if let Some(b) = briefs {
                        cell_label(
                            CELL_SCANS[i],
                            held_mask,
                            held_word,
                            first_down,
                            &phonemes,
                            b,
                            in_number_mode,
                            has_number_context,
                        )
                    } else {
                        String::new()
                    }
                });
                Some((target, labels, key_state, errored, ()))
            } else {
                None
            };

        if let Some((target, labels, key_state, errored, _)) = label_data.as_ref() {
            // Errored frames render as if there were no target — every
            // cell goes idle, the cue being "everything dark, release
            // and try again" rather than a specific red highlight.
            let target = if *errored {
                crate::tutor::drill::Target::default()
            } else {
                *target
            };
            let first_down = self
                .tutor_state
                .as_ref()
                .and_then(|s| s.tutor_first_down);
            let cell_d = ((span * ru / 12.0).round() as i32).max(12);
            let cell_gap = ((span * ru / 96.0).round() as i32).max(2);
            let hand_gap = cell_gap * 4;
            let row_w = 10 * cell_d + 8 * cell_gap + hand_gap;
            let row_x = (bw - row_w) / 2;
            let row_y = bh - cell_d * 3;
            let cy = row_y + cell_d / 2;

            // Cell scancodes in display order; needed both for cell-
            // colour primary/secondary checks and for the label pass.
            const CELL_SCANS: [u8; 10] = [
                crate::scan::L_PINKY,
                crate::scan::L_RING,
                crate::scan::L_MID,
                crate::scan::L_IDX,
                crate::scan::L_IDX_INNER,
                crate::scan::R_IDX_INNER,
                crate::scan::R_IDX,
                crate::scan::R_MID,
                crate::scan::R_RING,
                crate::scan::R_PINKY,
            ];
            // Primary = bright key colour. Every target cell is primary
            // when there's no ordering constraint, OR once the user has
            // committed to a lead (any key down). Otherwise only cells
            // listed in target.accepted_leads are bright; the rest dim
            // to dot_color as a "press one of these first" cue.
            let is_primary = |cell_scan: u8| -> bool {
                if target.accepted_leads.is_empty() {
                    return true;
                }
                if first_down.is_some() {
                    return true;
                }
                target.accepted_leads.test(cell_scan)
            };

            // Cell rectangles in display order (left 0..5, right 5..10),
            // captured during drawing so the label pass can reuse the
            // exact centres without repeating the layout math.
            let mut cell_centres: [(i32, i32); 10] = [(0, 0); 10];

            // Per-cell pressed state in display order. Drives both the
            // press-darken on idle grey and the bevel reversal in
            // `cell()` so any held key reads as pushed-in.
            let pressed: [bool; 10] = [
                key_state.left[0],  // L pinky
                key_state.left[1],  // L ring
                key_state.left[2],  // L mid
                key_state.left[3],  // L idx
                key_state.left[4],  // L idx-inner
                key_state.right[5], // R idx-inner
                key_state.right[0], // R idx
                key_state.right[1], // R mid
                key_state.right[2], // R ring
                key_state.right[3], // R pinky
            ];

            let left_bits = [3usize, 2, 1, 0, 4];
            let right_bits = [5usize, 0, 1, 2, 3];

            let mut x = row_x;
            for i in 0..5 {
                let is_target = (target.left & (1u8 << left_bits[i])) != 0;
                let fill =
                    finger_cell_fill(i, is_target, is_primary(CELL_SCANS[i]), pressed[i]);
                let cx_cell = x + cell_d / 2;
                cell(
                    pixels,
                    &mut self.tutor_hit_test,
                    width,
                    height,
                    cx_cell,
                    cy,
                    cell_d,
                    fill,
                    pressed[i],
                );
                cell_centres[i] = (cx_cell, cy);
                x += cell_d + cell_gap;
            }
            x += hand_gap - cell_gap;
            for i in 0..5 {
                let cell_idx = 5 + i;
                let is_target = (target.right & (1u8 << right_bits[i])) != 0;
                let fill = finger_cell_fill(
                    cell_idx,
                    is_target,
                    is_primary(CELL_SCANS[cell_idx]),
                    pressed[cell_idx],
                );
                let cx_cell = x + cell_d / 2;
                cell(
                    pixels,
                    &mut self.tutor_hit_test,
                    width,
                    height,
                    cx_cell,
                    cy,
                    cell_d,
                    fill,
                    pressed[cell_idx],
                );
                cell_centres[cell_idx] = (cx_cell, cy);
                x += cell_d + cell_gap;
            }

            // Second row below the chord cells: word bar (long, left-
            // aligned under L-pinky) + mod cell (2 cells wide, right-
            // aligned under R-pinky). Word = purple, mod = green —
            // distinct from the finger gradient so the thumb/word
            // roles read at a glance.
            let bottom_cy = cy + cell_d + cell_gap;
            let mod_w = 2 * cell_d + cell_gap;
            let mod_cx = row_x + row_w - mod_w / 2;
            let mod_target = (target.right & (1u8 << 4)) != 0;
            // Mod-tap-only target (thumb alone, no fingers): step
            // advances on key-UP, so cell goes dark once thumb is
            // held — "got it, release to fire". A chord that
            // includes mod alongside fingers stays bright until the
            // chord completes on full key-down.
            let is_mod_tap_only_target = target.right == (1u8 << 4) && target.left == 0;
            let mod_fill = if !mod_target || is_mod_tap_only_target {
                IDLE_FILL
            } else {
                let mod_primary = target.accepted_leads.is_empty()
                    || first_down.is_some()
                    || target.accepted_leads.test(crate::scan::R_THUMB);
                if mod_primary { MOD_PRIMARY } else { MOD_SECONDARY }
            };
            let mod_pressed = key_state.right[4];
            cell_wide(
                pixels,
                &mut self.tutor_hit_test,
                width,
                height,
                mod_cx,
                bottom_cy,
                mod_w,
                cell_d,
                mod_fill,
                mod_pressed,
            );

            let word_sep = cell_gap * 2;
            let word_w = row_w - mod_w - word_sep;
            let word_cx = row_x + word_w / 2;
            let word_fill = if target.word { WORD_PRIMARY } else { IDLE_FILL };
            let word_pressed = key_state.word;
            cell_wide(
                pixels,
                &mut self.tutor_hit_test,
                width,
                height,
                word_cx,
                bottom_cy,
                word_w,
                cell_d,
                word_fill,
                word_pressed,
            );
            let _ = WORD_SECONDARY;

            // Adaptive labels: draw each cell's predicted glyph centred
            // in the cell. Done in its own pass so the text renderer's
            // mutable borrow doesn't collide with tutor_state.
            if let Some(text) = self.text_renderer.as_mut() {
                let label_font = (cell_d as f32 * 0.55).max(10.0);
                for i in 0..10 {
                    if labels[i].is_empty() {
                        continue;
                    }
                    let (cx_cell, cy_cell) = cell_centres[i];
                    text.draw_text_center_u32(
                        pixels,
                        width,
                        &labels[i],
                        cx_cell as f32,
                        cy_cell as f32 + label_font * 0.35,
                        label_font,
                        500,
                        0xFFE8_E8F0,
                        "Josefin Slab",
                    );
                }
            }
        }

        // Debug overlays — photon-parity visualisations toggled with
        // Ctrl+H / Ctrl+T. Drawn LAST so they cover any UI underneath.
        if self.tutor_debug_hit_test {
            for (idx, &id) in self.tutor_hit_test.iter().enumerate() {
                if let Some(&(r, g, b)) = self.tutor_debug_hit_colours.get(id as usize) {
                    pixels[idx] = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            }
        }
        if self.tutor_show_textbox_mask {
            for (idx, &a) in self.tutor_textbox_mask.iter().enumerate() {
                let v = a as u32;
                pixels[idx] = 0xFF00_0000 | (v << 16) | (v << 8) | v;
            }
        }

        // Debug HUD — counters + current zoom. Visible when Ctrl+D
        // toggled the global debug flag. Photon's layout: bottom strip
        // with R/U/F counters + top-left zoom percentage.
        self.tutor_redraw_counter += 1;
        self.tutor_frame_counter += 1;
        if self.tutor_debug {
            if let Some(text) = self.text_renderer.as_mut() {
                let span = crate::tutor::ui::span(size.width, size.height);
                let counter_size = (span / 24.0).max(10.0);
                let strip_h = (counter_size * 2.0) as i32;

                // Dark strip along the bottom for contrast.
                let stride = width;
                let start_row = (height as i32 - strip_h).max(0) as usize;
                for y in start_row..height {
                    let base = y * stride;
                    for x in 0..width {
                        let p = pixels[base + x];
                        pixels[base + x] = (p >> 1 & 0xFF7F_7F7F) | 0xFF00_0000;
                    }
                }

                let y = height as f32 - counter_size;
                let redraw_text = format!("R:{}", self.tutor_redraw_counter);
                let frame_text = format!("F:{}", self.tutor_frame_counter);
                let zoom_text = format!("ru:{:.0}%", self.tutor_ru * 100.0);
                text.draw_text_left_u32(
                    pixels,
                    width,
                    &redraw_text,
                    counter_size,
                    y,
                    counter_size,
                    400,
                    0xFFFF_FFFF,
                    "Josefin Slab",
                );
                text.draw_text_center_u32(
                    pixels,
                    width,
                    &zoom_text,
                    width as f32 / 2.0,
                    y,
                    counter_size,
                    400,
                    0xFFFF_FFFF,
                    "Josefin Slab",
                );
                text.draw_text_right_u32(
                    pixels,
                    width,
                    &frame_text,
                    width as f32 - counter_size,
                    y,
                    counter_size,
                    400,
                    0xFFFF_FFFF,
                    "Josefin Slab",
                );
            }
        }

        let _ = buf.present();
    }

    /// Photon-parity resize-edge detection. Returns Some(direction) if
    /// the cursor is inside the `span/32` border strip on any side or
    /// corner of the window. Mirrors photon's `get_resize_edge`.
    fn resize_edge_at_cursor(&self) -> Option<ResizeDirection> {
        let window = self.tutor_window.as_ref()?;
        let size = window.inner_size();
        let x = self.tutor_cursor.x as f32;
        let y = self.tutor_cursor.y as f32;
        let border = (crate::tutor::ui::span(size.width, size.height) / 32.0).ceil();
        let at_left = x < border;
        let at_right = x > size.width as f32 - border;
        let at_top = y < border;
        let at_bottom = y > size.height as f32 - border;
        let dir = if at_top && at_left {
            ResizeDirection::NorthWest
        } else if at_top && at_right {
            ResizeDirection::NorthEast
        } else if at_bottom && at_left {
            ResizeDirection::SouthWest
        } else if at_bottom && at_right {
            ResizeDirection::SouthEast
        } else if at_top {
            ResizeDirection::North
        } else if at_bottom {
            ResizeDirection::South
        } else if at_left {
            ResizeDirection::West
        } else if at_right {
            ResizeDirection::East
        } else {
            return None;
        };
        Some(dir)
    }

    fn handle_keyboard(&mut self, event: KeyEvent) {
        // Track Ctrl via the key event directly — ModifiersChanged can
        // skip events on some Wayland compositors until after the
        // window has held keyboard focus for a bit.
        if matches!(event.logical_key, Key::Named(NamedKey::Control)) {
            self.tutor_ctrl_held = event.state == ElementState::Pressed;
        }
        if event.state != ElementState::Pressed {
            return;
        }
        let ctrl = self.tutor_mods.control_key() || self.tutor_ctrl_held;
        if !ctrl {
            return;
        }
        let Key::Character(c) = &event.logical_key else {
            return;
        };

        // rhe's evdev grab eats the home-row letters (A/S/D/F/G/H/J/K/L/;)
        // as chord roles, so photon's default Ctrl+D / Ctrl+H never
        // reach this handler while rhe is enabled. We use digit keys
        // for the primary shortcuts so they pass through the grab
        // untouched. Letter aliases are kept as a courtesy for muscle
        // memory from photon — they work whenever rhe is disabled.
        //
        //   Ctrl+1 / Ctrl+D   debug counters HUD
        //   Ctrl+2 / Ctrl+H   hit-map colour overlay
        //   Ctrl+3 / Ctrl+T   textbox mask overlay
        //   Ctrl+= or +       zoom in
        //   Ctrl+-            zoom out
        //   Ctrl+0            reset zoom
        let matched = if c == "1" || c.eq_ignore_ascii_case("d") {
            self.tutor_debug = !self.tutor_debug;
            true
        } else if c == "2" || c.eq_ignore_ascii_case("h") {
            self.tutor_debug_hit_test = !self.tutor_debug_hit_test;
            self.tutor_show_textbox_mask = false;
            if self.tutor_debug_hit_test && self.tutor_debug_hit_colours.is_empty() {
                let mut seed: u32 = 0x9E3779B9;
                for _ in 0..=255u8 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let r = (seed >> 16) as u8;
                    let g = (seed >> 8) as u8;
                    let b = seed as u8;
                    self.tutor_debug_hit_colours.push((r, g, b));
                }
            }
            true
        } else if c == "3" || c.eq_ignore_ascii_case("t") {
            self.tutor_show_textbox_mask = !self.tutor_show_textbox_mask;
            self.tutor_debug_hit_test = false;
            true
        } else if c == "=" || c == "+" {
            self.tutor_ru = (self.tutor_ru * 1.1).clamp(0.3, 5.0);
            true
        } else if c == "-" {
            self.tutor_ru = (self.tutor_ru / 1.1).clamp(0.3, 5.0);
            true
        } else if c == "0" {
            self.tutor_ru = 1.0;
            true
        } else {
            false
        };
        if matched {
            if let Some(w) = self.tutor_window.as_ref() {
                w.request_redraw();
            }
        }
    }

    /// Read the hit-test map at the current cursor position. Returns 0
    /// (HIT_NONE) when cursor is out of bounds.
    fn hit_at_cursor(&self) -> u8 {
        let Some(window) = self.tutor_window.as_ref() else {
            return HIT_NONE;
        };
        let size = window.inner_size();
        let width = size.width as usize;
        let height = size.height as usize;
        let x = self.tutor_cursor.x as i32;
        let y = self.tutor_cursor.y as i32;
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return HIT_NONE;
        }
        let idx = y as usize * width + x as usize;
        self.tutor_hit_test.get(idx).copied().unwrap_or(HIT_NONE)
    }

    fn on_menu_click(&mut self, event_loop: &ActiveEventLoop, id: MenuId) {
        if id == self.ids.tutor {
            self.open_tutor(event_loop);
        } else if id == self.ids.mode {
            let current = FallbackMode::from_u8(self.fallback.load(Ordering::Relaxed));
            let next = match current {
                FallbackMode::Autospell => FallbackMode::Ipa,
                FallbackMode::Ipa => FallbackMode::Autospell,
            };
            self.fallback.store(next.as_u8(), Ordering::Relaxed);
            #[cfg(target_os = "macos")]
            if let Some(state) = self.mac_state.as_mut() {
                state.mode_item.set_text(match next {
                    FallbackMode::Autospell => "Autospell",
                    FallbackMode::Ipa => "IPA",
                });
            }
            // Linux: gtk thread's glib timeout catches the atomic delta.
        } else if id == self.ids.enabled {
            let now = !self.enabled.load(Ordering::Relaxed);
            self.enabled.store(now, Ordering::Relaxed);
            self.refresh_enabled_ui();
        } else if id == self.ids.quit {
            self.quit.store(true, Ordering::Relaxed);
            event_loop.exit();
        }
    }
}

impl ApplicationHandler<TrayEvent> for TrayApp {
    #[cfg(target_os = "macos")]
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // macOS builds the tray on the event-loop thread so NSStatusItem
        // is constructed on the NSApp main thread.
        if self.mac_state.is_some() {
            return;
        }
        self.mac_state = Some(build_mac_tray(&self.ids, &self.enabled, &self.fallback));
    }

    #[cfg(not(target_os = "macos"))]
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Linux: tray built on the gtk thread by spawn_linux_tray_thread.
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.tutor_window.as_ref() else {
            return;
        };
        if window.id() != id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_tutor();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.tutor_renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                if let Some(w) = self.tutor_window.as_ref() {
                    w.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.tutor_mods = mods.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.tutor_cursor = position;
                if let Some(w) = self.tutor_window.as_ref() {
                    // Cursor priority (matches photon):
                    //   chrome buttons → Pointer
                    //   resize edges   → direction arrow
                    //   else           → Default
                    let hit = self.hit_at_cursor();
                    let icon =
                        if hit == HIT_CLOSE_BUTTON
                            || hit == HIT_MAXIMIZE_BUTTON
                            || hit == HIT_MINIMIZE_BUTTON
                        {
                            CursorIcon::Pointer
                        } else {
                            match self.resize_edge_at_cursor() {
                                Some(ResizeDirection::NorthWest)
                                | Some(ResizeDirection::SouthEast) => CursorIcon::NwseResize,
                                Some(ResizeDirection::NorthEast)
                                | Some(ResizeDirection::SouthWest) => CursorIcon::NeswResize,
                                Some(ResizeDirection::North) | Some(ResizeDirection::South) => {
                                    CursorIcon::NsResize
                                }
                                Some(ResizeDirection::East) | Some(ResizeDirection::West) => {
                                    CursorIcon::EwResize
                                }
                                Some(_) | None => CursorIcon::Default,
                            }
                        };
                    w.set_cursor(icon);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let Some(window) = self.tutor_window.as_ref() else {
                        return;
                    };
                    // Priority: chrome buttons → resize edges → drag.
                    // Photon's `handle_mouse_click` uses the same
                    // ordering.
                    let hit = self.hit_at_cursor();
                    if hit == HIT_CLOSE_BUTTON {
                        self.close_tutor();
                        return;
                    }
                    if hit == HIT_MAXIMIZE_BUTTON {
                        let was_max = window.is_maximized();
                        window.set_maximized(!was_max);
                        return;
                    }
                    if hit == HIT_MINIMIZE_BUTTON {
                        window.set_minimized(true);
                        return;
                    }

                    if let Some(dir) = self.resize_edge_at_cursor() {
                        let _ = window.drag_resize_window(dir);
                        return;
                    }

                    // Nothing hit — drag-move the window.
                    let _ = window.drag_window();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(event);
            }
            WindowEvent::Focused(true) => {
                // Nudge: some compositors don't deliver the initial
                // ModifiersChanged until the window has focus. Request
                // a redraw in case anything depends on mods state.
                if let Some(w) = self.tutor_window.as_ref() {
                    w.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.tutor_mods.control_key() || self.tutor_ctrl_held {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 50.0) as f32,
                    };
                    if steps != 0.0 {
                        // Zoom-per-notch matches photon's feel (1.1×
                        // per scroll step). Clamped so nobody can zoom
                        // into the abyss.
                        let factor = 1.1f32.powf(steps);
                        self.tutor_ru = (self.tutor_ru * factor).clamp(0.3, 5.0);
                        if let Some(w) = self.tutor_window.as_ref() {
                            w.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw_tutor();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: TrayEvent) {
        match event {
            TrayEvent::Menu(id) => self.on_menu_click(event_loop, id),
            TrayEvent::StateChanged => self.refresh_enabled_ui(),
            TrayEvent::DrillKey(ev) => {
                if let Some(state) = self.tutor_state.as_mut() {
                    state.tick(ev);
                    self.maybe_swap_practice();
                    if let Some(w) = self.tutor_window.as_ref() {
                        w.request_redraw();
                    }
                }
            }
        }

        if self.quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

/// Forwarder thread: pipe `tray-icon`'s internal menu-click channel
/// into the winit event loop. Runs on any platform — the menu-event
/// receiver itself is thread-agnostic.
fn spawn_menu_forwarder(proxy: TrayProxy) {
    std::thread::spawn(move || {
        let rx = MenuEvent::receiver();
        while let Ok(event) = rx.recv() {
            if proxy.send_event(TrayEvent::Menu(event.id)).is_err() {
                break;
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn build_mac_tray(
    ids: &TrayIds,
    enabled: &Arc<AtomicBool>,
    fallback: &Arc<AtomicU8>,
) -> MacTrayState {
    let icon_off = make_tray_icon(ICON_SIZE, false);
    let icon_on = make_tray_icon(ICON_SIZE, true);

    let initial_enabled = enabled.load(Ordering::Relaxed);
    let initial_fallback = FallbackMode::from_u8(fallback.load(Ordering::Relaxed));
    let is_autospell = initial_fallback == FallbackMode::Autospell;

    // Rebuild MenuItems with the IDs we pre-allocated so that the
    // winit thread can match click events.
    let tutor_item = MenuItem::with_id(ids.tutor.clone(), "Open Tutor", true, None);
    let mode_item = MenuItem::with_id(
        ids.mode.clone(),
        if is_autospell { "Autospell" } else { "IPA" },
        true,
        None,
    );
    let enabled_item = MenuItem::with_id(
        ids.enabled.clone(),
        if initial_enabled { "rhe" } else { "keyboard" },
        true,
        None,
    );
    let quit_item = MenuItem::with_id(ids.quit.clone(), "Exit", true, None);

    let menu = Menu::new();
    menu.append(&tutor_item).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&mode_item).ok();
    menu.append(&enabled_item).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&quit_item).ok();

    let initial_icon = if initial_enabled {
        icon_on.clone()
    } else {
        icon_off.clone()
    };

    let tray = TrayIconBuilder::new()
        .with_icon(initial_icon)
        .with_tooltip("rhe")
        .with_menu(Box::new(menu))
        .build()
        .expect("failed to build tray icon");
    tray.set_icon_as_template(false);

    MacTrayState {
        tray,
        icon_on,
        icon_off,
        mode_item,
        enabled_item,
    }
}

#[cfg(target_os = "linux")]
fn spawn_linux_tray_thread(
    ids: TrayIds,
    enabled: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    fallback: Arc<AtomicU8>,
) {
    std::thread::spawn(move || {
        use gtk::glib;

        gtk::init().expect("gtk::init failed");

        let icon_off = make_tray_icon(ICON_SIZE, false);
        let icon_on = make_tray_icon(ICON_SIZE, true);

        let initial_enabled = enabled.load(Ordering::Relaxed);
        let initial_fallback = FallbackMode::from_u8(fallback.load(Ordering::Relaxed));
        let is_autospell = initial_fallback == FallbackMode::Autospell;

        let tutor_item = MenuItem::with_id(ids.tutor.clone(), "Open Tutor", true, None);
        let mode_item = MenuItem::with_id(
            ids.mode.clone(),
            if is_autospell { "Autospell" } else { "IPA" },
            true,
            None,
        );
        let enabled_item = MenuItem::with_id(
            ids.enabled.clone(),
            if initial_enabled { "rhe" } else { "keyboard" },
            true,
            None,
        );
        let quit_item = MenuItem::with_id(ids.quit.clone(), "Exit", true, None);

        let menu = Menu::new();
        menu.append(&tutor_item).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&mode_item).ok();
        menu.append(&enabled_item).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&quit_item).ok();

        let initial_icon = if initial_enabled {
            icon_on.clone()
        } else {
            icon_off.clone()
        };

        let tray = TrayIconBuilder::new()
            .with_icon(initial_icon)
            .with_tooltip("rhe")
            .with_menu(Box::new(menu))
            .build()
            .expect("failed to build tray icon");

        // Stateful closure that polls the shared atomics and updates the
        // tray icon + menu labels when they diverge. Runs on this (gtk)
        // thread via glib::timeout_add_local — safe for MenuItem/TrayIcon
        // which are !Send on Linux.
        let mut last_enabled = initial_enabled;
        let mut last_fallback = initial_fallback.as_u8();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let e = enabled.load(Ordering::Relaxed);
            if e != last_enabled {
                last_enabled = e;
                enabled_item.set_text(if e { "rhe" } else { "keyboard" });
                let icon = if e { icon_on.clone() } else { icon_off.clone() };
                tray.set_icon(Some(icon)).ok();
            }
            let f = fallback.load(Ordering::Relaxed);
            if f != last_fallback {
                last_fallback = f;
                let fb = FallbackMode::from_u8(f);
                mode_item.set_text(match fb {
                    FallbackMode::Autospell => "Autospell",
                    FallbackMode::Ipa => "IPA",
                });
            }
            if quit.load(Ordering::Relaxed) {
                gtk::main_quit();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });

        gtk::main();
    });
}

/// Run the tray event loop on the main thread. Blocks until `quit` is set
/// or the menu's Quit item fires.
///
/// `fallback` is the shared `AtomicU8` the interpreter reads each time it
/// hits the fallback branch — the Fallback submenu writes to it directly.
pub fn run_tray(
    event_loop: EventLoop<TrayEvent>,
    enabled: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    fallback: Arc<AtomicU8>,
    mode_flags: Arc<AtomicU8>,
) {
    let ids = TrayIds {
        tutor: MenuId::new("rhe.tutor"),
        mode: MenuId::new("rhe.mode"),
        enabled: MenuId::new("rhe.enabled"),
        quit: MenuId::new("rhe.quit"),
    };

    spawn_menu_forwarder(event_loop.create_proxy());

    #[cfg(target_os = "linux")]
    spawn_linux_tray_thread(ids.clone(), enabled.clone(), quit.clone(), fallback.clone());

    let mut app = TrayApp {
        enabled,
        quit,
        fallback,
        mode_flags,
        ids,
        #[cfg(target_os = "macos")]
        mac_state: None,
        tutor_renderer: None,
        tutor_window: None,
        tutor_state: None,
        text_renderer: None,
        tutor_wiki_stream: None,
        tutor_word_lookup: None,
        tutor_brief_table: None,
        tutor_ru: 1.0,
        tutor_mods: ModifiersState::empty(),
        tutor_cursor: PhysicalPosition::new(0.0, 0.0),
        tutor_hit_test: Vec::new(),
        tutor_textbox_mask: Vec::new(),
        tutor_debug: false,
        tutor_debug_hit_test: false,
        tutor_show_textbox_mask: false,
        tutor_debug_hit_colours: Vec::new(),
        tutor_frame_counter: 0,
        tutor_redraw_counter: 0,
        tutor_ctrl_held: false,
    };

    event_loop.run_app(&mut app).ok();
}
