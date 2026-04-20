use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::{Icon, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const ICON_SIZE: u32 = 22;

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

/// Run the menu bar tray app on the main thread.
///
/// `enabled` is shared with the engine thread — clicking the icon
/// toggles this flag. The engine checks it to decide whether to
/// intercept keys or pass them through.
pub fn run_tray(enabled: Arc<AtomicBool>) {
    let event_loop = EventLoop::new();

    let icon_off = circle_icon(180, 60, 180, 160);
    let icon_on = circle_icon(80, 255, 80, 255);

    let initial = if enabled.load(Ordering::Relaxed) {
        icon_on.clone()
    } else {
        icon_off.clone()
    };

    let tray = TrayIconBuilder::new()
        .with_icon(initial)
        .with_tooltip("rhe")
        .build()
        .expect("failed to build tray icon");

    #[cfg(target_os = "macos")]
    tray.set_icon_as_template(false);

    let rx = TrayIconEvent::receiver();

    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(50),
        );

        while let Ok(tray_event) = rx.try_recv() {
            if let TrayIconEvent::Click {
                button_state: MouseButtonState::Up,
                ..
            } = tray_event
            {
                let now = !enabled.load(Ordering::Relaxed);
                enabled.store(now, Ordering::Relaxed);

                let icon = if now {
                    icon_on.clone()
                } else {
                    icon_off.clone()
                };
                tray.set_icon(Some(icon)).ok();

                let state = if now { "ON" } else { "OFF" };
                eprintln!("rhe: {}", state);
            }
        }
    });
}
