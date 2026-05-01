//! Binary self-verification using Ed25519 cryptographic signatures.
//!
//! Signed rhe binaries carry a trailer at the very end of the executable: a 64-byte Ed25519 signature followed by the 8-byte magic `RHESIGV1`. The verifier checks for the magic; if present it verifies the preceding 64 bytes against the BLAKE3 hash of everything before the trailer. If the magic is absent, the binary is treated as unsigned (cargo install or local dev build) and verification silently passes.
//!
//! # For end users
//!
//! Use the official installer — don't build from source unless you know what you're doing:
//!
//! - **Linux/macOS**: `curl -sSfL https://brobdingnagian.holdmyoscilloscope.com/rhe/install-release.sh | sh`
//! - **Windows**: `iwr -useb https://brobdingnagian.holdmyoscilloscope.com/rhe/install-release.ps1 | iex`
//!
//! These download pre-built, pre-signed binaries that carry the trailer above.
//!
//! # For `cargo install rhe` users
//!
//! Cargo builds locally with no signing infrastructure (no private key, no `sign-after-build.sh`), so the resulting binary has no trailer. Verification recognises the missing trailer and returns `Ok(None)` instead of failing — `rhe verify` reports "unsigned" and startup continues normally. You're trusting your own build at that point, the same way you would for any other `cargo install`-ed crate.
//!
//! To produce a signed binary as a contributor:
//!
//! 1. Generate your own keypair: `cargo run --bin rhe-keygen`
//! 2. Replace `AUTHOR_PUBKEY` below with your generated public key
//! 3. Build, then sign: `cargo run --bin rhe-signature-signer -- target/release/rhe`
//!
//! Public key (hex): 68519ed076f87fde60510912fb2ce7cba20c0258ac683626c825f377eac25adb

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Trailer magic appended after the 64-byte signature so the verifier can distinguish a signed binary (cargo build → sign-after-build.sh) from an unsigned cargo-install build whose last 64 bytes are arbitrary section data. Versioned so the format can evolve without ambiguity.
pub const SIGNATURE_TRAILER: &[u8; 8] = b"RHESIGV1";
const TRAILER_LEN: usize = 64 + 8;

/// Embedded Ed25519 public key for the official rhe distribution.
///
/// Binaries bearing a signature that verifies against this key are guaranteed to have been built and released by the original author. Public key (hex): 68519ed076f87fde60510912fb2ce7cba20c0258ac683626c825f377eac25adb
pub const AUTHOR_PUBKEY: [u8; 32] = [
    0x68, 0x51, 0x9e, 0xd0, 0x76, 0xf8, 0x7f, 0xde, 0x60, 0x51, 0x09, 0x12, 0xfb, 0x2c, 0xe7, 0xcb,
    0xa2, 0x0c, 0x02, 0x58, 0xac, 0x68, 0x36, 0x26, 0xc8, 0x25, 0xf3, 0x77, 0xea, 0xc2, 0x5a, 0xdb,
];

/// Three-way result of self-verification:
/// - `Ok(Some(sig_hex))` — binary carries the trailer and the signature verifies against `AUTHOR_PUBKEY`. Hex-encoded signature returned for logging.
/// - `Ok(None)` — binary has no trailer (cargo install or local unsigned build). Caller should continue without complaint.
/// - `Err(msg)` — trailer present but signature is invalid: tampered, corrupted, or signed by a different key. Caller should refuse to launch.
pub fn verify_binary_hash() -> Result<Option<String>, String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

    let exe_data =
        std::fs::read(&exe_path).map_err(|e| format!("Failed to read executable: {}", e))?;

    // No trailer → unsigned binary. Common case for `cargo install rhe` and local
    // `cargo build` runs that didn't go through sign-after-build.sh.
    if exe_data.len() < TRAILER_LEN
        || &exe_data[exe_data.len() - SIGNATURE_TRAILER.len()..] != SIGNATURE_TRAILER
    {
        return Ok(None);
    }

    let body_end = exe_data.len() - TRAILER_LEN;
    let signature_bytes = &exe_data[body_end..exe_data.len() - SIGNATURE_TRAILER.len()];
    let body = &exe_data[..body_end];

    let signature = Signature::from_bytes(
        signature_bytes
            .try_into()
            .map_err(|_| "Invalid signature length".to_string())?,
    );

    let hash = blake3::hash(body);

    let verifying_key = VerifyingKey::from_bytes(&AUTHOR_PUBKEY)
        .map_err(|e| format!("Invalid embedded public key: {}", e))?;

    verifying_key
        .verify(hash.as_bytes(), &signature)
        .map_err(|_| "Signature verification failed — binary corrupted or modified".to_string())?;

    Ok(Some(hex::encode(signature.to_bytes()).to_uppercase()))
}
