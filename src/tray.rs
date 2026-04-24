//! Cross-platform tray icon + right-click menu + on-demand tutor window.
//!
//! macOS gets its native menu bar icon via `tray-icon` (which wraps
//! `NSStatusItem`). Linux gets a StatusNotifierItem registered via DBus
//! — works out of the box on KDE, XFCE, Cinnamon, MATE; on GNOME the user
//! needs `gnome-shell-extension-appindicator` installed and enabled.
//!
//! The tray provides an ambient always-visible state indicator (icon
//! color reflects rhe enabled/disabled) and a right-click menu. The
//! winit event loop owns the main thread — this is the same event loop
//! that hosts the tutor GUI window, so the tray and tutor share one
//! runtime instead of fighting over main-thread ownership.
//!
//! Event-driven: the event loop sleeps indefinitely until either a menu
//! event (bridged from `tray-icon`'s own channel via a forwarder
//! thread) or a state-change notification (sent by the evdev reader via
//! `EventLoopProxy`) fires.

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::interpreter::FallbackMode;
use crate::ui::primitives;
use crate::ui::renderer::Renderer;
use crate::ui::text_rasterizing::TextRenderer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const ICON_SIZE: u32 = 22;

/// Events that wake the winit event loop.
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// The engine thread toggled the `enabled` flag (via caps tap).
    /// The tray icon + check item need refreshing.
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

struct TrayApp {
    enabled: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    fallback: Arc<AtomicU8>,
    icon_on: Icon,
    icon_off: Icon,
    tray: Option<TrayIcon>,

    // Tutor window state. Declared renderer-before-window so they drop
    // in that order (renderer holds `&'static Window` transmuted from
    // the window below, and must be dropped first to avoid dangling).
    tutor_renderer: Option<Renderer>,
    tutor_window: Option<Window>,
    text_renderer: Option<TextRenderer>,

    mode_item: MenuItem,
    enabled_item: MenuItem,
    tutor_item: MenuItem,
    quit_item: MenuItem,
    mode_id: MenuId,
    enabled_id: MenuId,
    tutor_id: MenuId,
    quit_id: MenuId,
}

impl TrayApp {
    fn refresh_enabled_ui(&mut self) {
        let on = self.enabled.load(Ordering::Relaxed);
        self.enabled_item.set_text(if on { "rhe" } else { "keyboard" });
        if let Some(tray) = &self.tray {
            let icon = if on {
                self.icon_on.clone()
            } else {
                self.icon_off.clone()
            };
            tray.set_icon(Some(icon)).ok();
        }
    }

    fn open_tutor(&mut self, event_loop: &ActiveEventLoop) {
        if self.tutor_window.is_some() {
            // Already open — just focus.
            if let Some(w) = &self.tutor_window {
                w.focus_window();
            }
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("rhe tutor")
            .with_inner_size(PhysicalSize::new(800u32, 500u32));
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
            w_ref.request_redraw();
        }
    }

    fn close_tutor(&mut self) {
        // Order matters: renderer first (holds &'static Window), window second.
        self.tutor_renderer = None;
        self.tutor_window = None;
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

        // Lazily construct the text renderer on first tutor open. It
        // parses 20 TTFs (~1.2MB), so we don't want this on startup.
        if self.text_renderer.is_none() {
            self.text_renderer = Some(TextRenderer::new());
        }

        let Some(renderer) = self.tutor_renderer.as_mut() else {
            return;
        };

        let mut buf = renderer.lock_buffer();
        primitives::fill(buf.as_mut(), primitives::rgb(0x1a, 0x1a, 0x22));

        // Phase B.4 placeholder: draw "rhe" centered to prove the full
        // font stack works end-to-end. Phase C will render the real
        // tutor view (target word + phoneme hint + keyboard diagram).
        if let Some(text) = self.text_renderer.as_mut() {
            let span = crate::ui::span(size.width, size.height);
            let font_size = span / 4.0;
            let cx = width as f32 / 2.0;
            let cy = height as f32 / 2.0;
            text.draw_text_center_u32(
                buf.as_mut(),
                width,
                "rhe",
                cx,
                cy,
                font_size,
                700,
                0x00E0E0F0,
                "Oxanium",
            );
        }

        buf.mark_all();
        let _ = buf.present();
    }
}

impl ApplicationHandler<TrayEvent> for TrayApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Construct the tray icon lazily here rather than in run_tray so
        // that on Linux the `gtk` init happens on the event-loop thread.
        if self.tray.is_some() {
            return;
        }

        let menu = Menu::new();
        menu.append(&self.tutor_item).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&self.mode_item).ok();
        menu.append(&self.enabled_item).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&self.quit_item).ok();

        let initial_on = self.enabled.load(Ordering::Relaxed);
        let initial_icon = if initial_on {
            self.icon_on.clone()
        } else {
            self.icon_off.clone()
        };

        let tray = TrayIconBuilder::new()
            .with_icon(initial_icon)
            .with_tooltip("rhe")
            .with_menu(Box::new(menu))
            .build()
            .expect("failed to build tray icon");

        #[cfg(target_os = "macos")]
        tray.set_icon_as_template(false);

        self.tray = Some(tray);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
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
            WindowEvent::RedrawRequested => {
                self.redraw_tutor();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: TrayEvent) {
        match event {
            TrayEvent::Menu(id) if id == self.tutor_id => {
                self.open_tutor(event_loop);
            }
            TrayEvent::Menu(id) if id == self.mode_id => {
                let current = FallbackMode::from_u8(self.fallback.load(Ordering::Relaxed));
                let next = match current {
                    FallbackMode::Autospell => FallbackMode::Ipa,
                    FallbackMode::Ipa => FallbackMode::Autospell,
                };
                self.fallback.store(next.as_u8(), Ordering::Relaxed);
                self.mode_item.set_text(match next {
                    FallbackMode::Autospell => "Autospell",
                    FallbackMode::Ipa => "IPA",
                });
            }
            TrayEvent::Menu(id) if id == self.enabled_id => {
                let now = !self.enabled.load(Ordering::Relaxed);
                self.enabled.store(now, Ordering::Relaxed);
                self.refresh_enabled_ui();
            }
            TrayEvent::Menu(id) if id == self.quit_id => {
                self.quit.store(true, Ordering::Relaxed);
                event_loop.exit();
            }
            TrayEvent::StateChanged => {
                self.refresh_enabled_ui();
            }
            _ => {}
        }

        if self.quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Engine thread signals quit through the shared flag. We poll on
        // every loop iteration before going back to sleep so a quit set
        // without a UserEvent nudge still lands.
        if self.quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }
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
    let icon_off = circle_icon(180, 60, 180, 160);
    let icon_on = circle_icon(80, 255, 80, 255);

    let initial_enabled = enabled.load(Ordering::Relaxed);
    let initial_fallback = FallbackMode::from_u8(fallback.load(Ordering::Relaxed));
    let is_autospell = initial_fallback == FallbackMode::Autospell;

    let tutor_item = MenuItem::new("Open Tutor", true, None);
    let mode_item = MenuItem::new(
        if is_autospell { "Autospell" } else { "IPA" },
        true,
        None,
    );
    let enabled_item = MenuItem::new(
        if initial_enabled { "rhe" } else { "keyboard" },
        true,
        None,
    );
    let quit_item = MenuItem::new("Exit", true, None);

    let tutor_id = tutor_item.id().clone();
    let mode_id = mode_item.id().clone();
    let enabled_id = enabled_item.id().clone();
    let quit_id = quit_item.id().clone();

    // Bridge tray-icon's menu channel into the winit event loop so menu
    // clicks trigger a UserEvent without us polling.
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        let rx = MenuEvent::receiver();
        while let Ok(event) = rx.recv() {
            if proxy.send_event(TrayEvent::Menu(event.id)).is_err() {
                break; // event loop closed
            }
        }
    });

    let mut app = TrayApp {
        enabled,
        quit,
        fallback,
        icon_on,
        icon_off,
        tray: None,
        tutor_renderer: None,
        tutor_window: None,
        text_renderer: None,
        tutor_item,
        mode_item,
        enabled_item,
        quit_item,
        tutor_id,
        mode_id,
        enabled_id,
        quit_id,
    };

    event_loop.run_app(&mut app).ok();
}
