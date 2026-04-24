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
            state.enabled_item.set_text(if on { "rhe" } else { "keyboard" });
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
    let icon_off = circle_icon(180, 60, 180, 160);
    let icon_on = circle_icon(80, 255, 80, 255);

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

        let icon_off = circle_icon(180, 60, 180, 160);
        let icon_on = circle_icon(80, 255, 80, 255);

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
    };

    event_loop.run_app(&mut app).ok();
}
