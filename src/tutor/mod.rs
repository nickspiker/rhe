//! Tutor: drill window + drill state machine + adaptive cell labels.
//!
//! All GUI in rhe currently exists for the tutor — the compositor,
//! softbuffer renderer, and text rasterizer under `ui` are vendored
//! photon code that has no other consumer yet, so they live under
//! this module. `drill` is the renderer-agnostic state machine;
//! `wiki` is the practice-text stream. `view` glues them to a winit
//! window for the on-demand drill UI.

pub mod drill;
pub mod ui;
pub mod wiki;
