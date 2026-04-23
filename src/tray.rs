//! Cross-platform tray icon + right-click menu.
//!
//! macOS gets its native menu bar icon via `tray-icon` (which wraps
//! `NSStatusItem`). Linux gets a StatusNotifierItem registered via DBus
//! — works out of the box on KDE, XFCE, Cinnamon, MATE; on GNOME the user
//! needs `gnome-shell-extension-appindicator` installed and enabled.
//!
//! The tray provides an ambient always-visible state indicator (icon
//! color reflects rhe enabled/disabled) and a right-click menu:
//!
//!   ☐ Enabled       (check item — toggles rhe on/off)
//!   ─────────
//!   Fallback
//!     ● Autospell   (radio-style: out-of-dict words → ASCII spelling)
//!     ○ IPA         (raw IPA unicode — GTK/Qt/IBus apps only)
//!   ─────────
//!   Quit            (exits the engine)
//!
//! Event-driven: the tao event loop sleeps indefinitely until either a
//! menu event (bridged from `tray-icon`'s own channel via a forwarder
//! thread) or a state-change notification (sent by the evdev reader via
//! `EventLoopProxy`) fires.

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::interpreter::FallbackMode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const ICON_SIZE: u32 = 22;

/// Events that wake the tao event loop.
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
pub fn build() -> (tao::event_loop::EventLoop<TrayEvent>, TrayProxy) {
    let event_loop = EventLoopBuilder::<TrayEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    (event_loop, proxy)
}

/// Run the tray event loop on the main thread. Blocks until `quit` is set
/// or the menu's Quit item fires.
///
/// `fallback` is the shared `AtomicU8` the interpreter reads each time it
/// hits the fallback branch — the Fallback submenu writes to it directly.
pub fn run_tray(
    event_loop: tao::event_loop::EventLoop<TrayEvent>,
    enabled: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
    fallback: Arc<AtomicU8>,
) {
    let icon_off = circle_icon(180, 60, 180, 160);
    let icon_on = circle_icon(80, 255, 80, 255);

    let initial_enabled = enabled.load(Ordering::Relaxed);
    let initial_icon = if initial_enabled {
        icon_on.clone()
    } else {
        icon_off.clone()
    };

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
    let menu = Menu::new();
    menu.append(&mode_item).ok();
    menu.append(&enabled_item).ok();
    menu.append(&quit_item).ok();

    let tray = TrayIconBuilder::new()
        .with_icon(initial_icon)
        .with_tooltip("rhe")
        .with_menu(Box::new(menu))
        .build()
        .expect("failed to build tray icon");

    #[cfg(target_os = "macos")]
    tray.set_icon_as_template(false);

    // Bridge tray-icon's menu channel into the tao event loop so menu
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

    let mode_id = mode_item.id().clone();
    let enabled_id = enabled_item.id().clone();
    let quit_id = quit_item.id().clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::UserEvent(user_event) = event {
            match user_event {
                TrayEvent::Menu(id) if id == mode_id => {
                    // Toggle between Autospell and IPA
                    let current = FallbackMode::from_u8(fallback.load(Ordering::Relaxed));
                    let next = match current {
                        FallbackMode::Autospell => FallbackMode::Ipa,
                        FallbackMode::Ipa => FallbackMode::Autospell,
                    };
                    fallback.store(next.as_u8(), Ordering::Relaxed);
                    mode_item.set_text(match next {
                        FallbackMode::Autospell => "Autospell",
                        FallbackMode::Ipa => "IPA",
                    });
                    eprintln!("rhe: mode → {:?} (tray)", next);
                }
                TrayEvent::Menu(id) if id == enabled_id => {
                    let now = !enabled.load(Ordering::Relaxed);
                    enabled.store(now, Ordering::Relaxed);
                    enabled_item.set_text(if now { "rhe" } else { "keyboard" });
                    let icon = if now { icon_on.clone() } else { icon_off.clone() };
                    tray.set_icon(Some(icon)).ok();
                    eprintln!("rhe: {} (tray)", if now { "rhe" } else { "keyboard" });
                }
                TrayEvent::Menu(id) if id == quit_id => {
                    eprintln!("rhe: exit from tray");
                    quit.store(true, Ordering::Relaxed);
                    *control_flow = ControlFlow::Exit;
                }
                TrayEvent::StateChanged => {
                    let current = enabled.load(Ordering::Relaxed);
                    enabled_item.set_text(if current { "rhe" } else { "keyboard" });
                    let icon = if current {
                        icon_on.clone()
                    } else {
                        icon_off.clone()
                    };
                    tray.set_icon(Some(icon)).ok();
                }
                _ => {}
            }
        }

        if quit.load(Ordering::Relaxed) {
            *control_flow = ControlFlow::Exit;
        }
    });
}
