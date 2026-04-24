//! rhe's GUI surface — vendored subset of photon's UI stack.
//!
//! The tutor window (and any future status UI) renders into a softbuffer
//! pixel buffer. The renderer module below is the platform-agnostic
//! handle; each platform has its own implementation underneath.
//!
//! Only Linux and macOS are supported this pass. Windows/Redox/Android
//! renderers from photon are not carried over.
//!
//! Layout discipline: no pixel-unit dimensions anywhere except 1px
//! hairlines. Every other size derives from `span = 2·w·h/(w+h)` times
//! an `ru` zoom factor. See `scale()`.

#[cfg(target_os = "linux")]
pub mod renderer_linux_softbuffer;

#[cfg(target_os = "macos")]
pub mod renderer_macos_softbuffer;

pub mod text_rasterizing;

#[cfg(target_os = "linux")]
pub use renderer_linux_softbuffer as renderer;

#[cfg(target_os = "macos")]
pub use renderer_macos_softbuffer as renderer;

/// Harmonic-mean scale invariant. Responsive, orientation-agnostic —
/// both landscape and portrait of the same diagonal pick the same span.
#[inline]
pub fn span(width: u32, height: u32) -> f32 {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    2.0 * w * h / (w + h)
}

/// Convert a fractional unit (span divisor) into an integer pixel
/// count. `ru` is the user zoom factor (default 1.0).
#[inline]
pub fn px(span: f32, ru: f32, divisor: f32) -> i32 {
    (span * ru / divisor).round() as i32
}
