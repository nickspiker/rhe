//! Cross-platform tray icon + right-click menu.
//!
//! macOS gets its native menu bar icon via `tray-icon` (which wraps
//! `NSStatusItem`). Linux gets a StatusNotifierItem registered via DBus
//! — works out of the box on KDE, XFCE, Cinnamon, MATE; on GNOME the user
//! needs `gnome-shell-extension-appindicator` installed and enabled.
//!
//! The tray provides an ambient always-visible state indicator (icon
//! color reflects rhe enabled/disabled) and a right-click menu. The
//! winit event loop owns the main thread — this is the same event loop
//! that will host the tutor GUI window in a later phase, so the tray
//! and tutor share one runtime instead of fighting over main-thread
//! ownership.
//!
//! Event-driven: the event loop sleeps indefinitely until either a menu
//! event (bridged from `tray-icon`'s own channel via a forwarder
//! thread) or a state-change notification (sent by the evdev reader via
//! `EventLoopProxy`) fires.

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::interpreter::FallbackMode;
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
    mode_item: MenuItem,
    enabled_item: MenuItem,
    quit_item: MenuItem,
    mode_id: MenuId,
    enabled_id: MenuId,
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
}

impl ApplicationHandler<TrayEvent> for TrayApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Construct the tray icon lazily here rather than in run_tray so
        // that on Linux the `gtk` init happens on the event-loop thread.
        if self.tray.is_some() {
            return;
        }

        let menu = Menu::new();
        menu.append(&self.mode_item).ok();
        menu.append(&self.enabled_item).ok();
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
        _id: WindowId,
        _event: WindowEvent,
    ) {
        // No windows yet in Phase A — tutor GUI window will land here
        // once Phase C lifts photon's renderer.
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: TrayEvent) {
        match event {
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
        mode_item,
        enabled_item,
        quit_item,
        mode_id,
        enabled_id,
        quit_id,
    };

    event_loop.run_app(&mut app).ok();
}
