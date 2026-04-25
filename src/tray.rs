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

use crate::interpreter::FallbackMode;
use crate::ui::compositor::{
    HIT_CLOSE_BUTTON, HIT_MAXIMIZE_BUTTON, HIT_MINIMIZE_BUTTON, HIT_NONE, TutorApp,
};
use crate::ui::renderer::Renderer;
use crate::ui::text_rasterizing::TextRenderer;
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

/// Render the tray icon: ring around a scaled-down logo.png via the
/// compositor's `draw_avatar` in straight-alpha mode (so the outer AA
/// fringe carries real alpha and the icon fades to transparent
/// outside the ring instead of leaving an opaque half-bright ring).
fn make_tray_icon(size: u32, online: bool) -> Icon {
    use crate::ui::compositor::TutorApp;

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
    text_renderer: Option<TextRenderer>,

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
    }

    fn close_tutor(&mut self) {
        // Order matters: renderer first (holds &'static Window), window second.
        self.tutor_renderer = None;
        self.tutor_window = None;
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

        // Phase B.4 placeholder: target word in Oxanium Bold, centered
        // below the chrome strip. Phase C replaces this with the real
        // tutor view.
        if let Some(text) = self.text_renderer.as_mut() {
            let span = crate::ui::span(size.width, size.height);
            let font_size = span * self.tutor_ru / 4.0;
            let chrome_h = button_height as f32;
            let cx = width as f32 / 2.0;
            let cy = chrome_h + (height as f32 - chrome_h) / 2.0;
            text.draw_text_center_u32(
                pixels,
                width,
                "rhe",
                cx,
                cy,
                font_size,
                700,
                0xFFE0_E0F0,
                "Oxanium",
            );
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
                let span = crate::ui::span(size.width, size.height);
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
        let border = (crate::ui::span(size.width, size.height) / 32.0).ceil();
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
        ids,
        #[cfg(target_os = "macos")]
        mac_state: None,
        tutor_renderer: None,
        tutor_window: None,
        text_renderer: None,
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
