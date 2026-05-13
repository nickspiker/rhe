//! Compile-time user-adjustable mappings.
//!
//! Each language ships its own phoneme/brief/suffix/number tables under a namespaced submodule (`en/` for English, `mri/` for te reo Māori). The active language is selected at compile time by the `lang-en` (default) or `lang-mri` Cargo feature; the re-exports below point `crate::layout::chords` / `briefs` / etc. at the active language so the rest of the codebase doesn't have to know which language is loaded.
//!
//! Language-neutral submodules (`keyboard`, `numbers`) are shared across all builds.

#[cfg(not(any(feature = "lang-en", feature = "lang-mri")))]
compile_error!("rhe needs exactly one of `lang-en` or `lang-mri` enabled");

#[cfg(all(feature = "lang-en", feature = "lang-mri"))]
compile_error!(
    "rhe must not have both `lang-en` and `lang-mri` enabled — pass --no-default-features when enabling lang-mri"
);

#[cfg(feature = "lang-en")]
pub mod en;

#[cfg(feature = "lang-mri")]
pub mod mri;

pub mod chord_key;
pub mod effort;
pub mod keyboard;
pub mod numbers;

#[cfg(feature = "lang-en")]
pub use en::briefs;
#[cfg(feature = "lang-en")]
pub use en::chords;
#[cfg(feature = "lang-en")]
pub use en::number_forms;
#[cfg(feature = "lang-en")]
pub use en::ordered_briefs;
#[cfg(feature = "lang-en")]
pub use en::suffixes;
#[cfg(feature = "lang-en")]
pub use en::symbols;

#[cfg(feature = "lang-mri")]
pub use mri::briefs;
#[cfg(feature = "lang-mri")]
pub use mri::chords;
#[cfg(feature = "lang-mri")]
pub use mri::number_forms;
#[cfg(feature = "lang-mri")]
pub use mri::ordered_briefs;
#[cfg(feature = "lang-mri")]
pub use mri::suffixes;
#[cfg(feature = "lang-mri")]
pub use mri::symbols;
