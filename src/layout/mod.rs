//! Compile-time user-adjustable mappings.
//!
//! Each language ships its own phoneme/brief/suffix/number tables under a namespaced submodule (`en/` for English, `mri/` for te reo Māori). The active language is selected at compile time by the `lang-en` (default) or `lang-mri` Cargo feature; the re-exports below point `crate::layout::chords` / `briefs` / etc. at the active language so the rest of the codebase doesn't have to know which language is loaded.
//!
//! Language-neutral submodules (`keyboard`, `numbers`) are shared across all builds.

pub mod en;
pub mod keyboard;
pub mod numbers;

pub use en::briefs;
pub use en::chords;
pub use en::number_forms;
pub use en::ordered_briefs;
pub use en::suffixes;
