//! Binary self-verification using Ed25519 cryptographic signatures.
//!
//! All official rhe binaries are signed by Nick Spiker
//! <fractaldecoder@proton.me>. The signature lives in the last 64
//! bytes of the executable and is verified at startup against the
//! embedded public key below.
//!
//! # For end users
//!
//! Use the official installer — don't build from source unless you
//! know what you're doing:
//!
//! - **Linux/macOS**: `curl -sSfL https://brobdingnagian.holdmyoscilloscope.com/rhe/install-release.sh | sh`
//! - **Windows**: `iwr -useb https://brobdingnagian.holdmyoscilloscope.com/rhe/install-release.ps1 | iex`
//!
//! These download pre-built, pre-signed binaries.
//!
//! # For contributors building from source
//!
//! `cargo install rhe` from crates.io will produce an UNSIGNED binary
//! (the signing scripts and keys aren't published with the crate).
//! That's fine for personal use — the binary still runs, the verify
//! check is only invoked explicitly via `rhe verify`.
//!
//! To produce a signed binary you'd need:
//!
//! 1. Generate your own keypair: `cargo run --bin rhe-keygen`
//! 2. Replace `AUTHOR_PUBKEY` below with your generated public key
//! 3. Build, then sign: `cargo run --bin rhe-signature-signer -- target/release/rhe`
//!
//! Public key (hex): 68519ed076f87fde60510912fb2ce7cba20c0258ac683626c825f377eac25adb

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Embedded Ed25519 public key for the official rhe distribution.
///
/// Binaries bearing a signature that verifies against this key are
/// guaranteed to have been built and released by the original author.
/// Public key (hex): 68519ed076f87fde60510912fb2ce7cba20c0258ac683626c825f377eac25adb
pub const AUTHOR_PUBKEY: [u8; 32] = [
    0x68, 0x51, 0x9e, 0xd0, 0x76, 0xf8, 0x7f, 0xde, 0x60, 0x51, 0x09, 0x12, 0xfb, 0x2c, 0xe7, 0xcb,
    0xa2, 0x0c, 0x02, 0x58, 0xac, 0x68, 0x36, 0x26, 0xc8, 0x25, 0xf3, 0x77, 0xea, 0xc2, 0x5a, 0xdb,
];

/// Verify that this binary has a valid Ed25519 signature appended to
/// the end. Returns the hex-encoded signature on success.
///
/// Errors if the signature is missing (all zeros), the binary is too
/// small to fit a signature, or the signature doesn't verify against
/// `AUTHOR_PUBKEY`.
pub fn verify_binary_hash() -> Result<String, String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

    let mut exe_data =
        std::fs::read(&exe_path).map_err(|e| format!("Failed to read executable: {}", e))?;

    if exe_data.len() < 64 {
        return Err("Binary too small — signature verification failed".to_string());
    }

    // Signature lives in the last 64 bytes.
    let signature_bytes = exe_data.split_off(exe_data.len() - 64);

    if signature_bytes.iter().all(|&b| b == 0) {
        return Err("Binary signature missing — executable was not signed".to_string());
    }

    let signature = Signature::from_bytes(
        signature_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid signature length".to_string())?,
    );

    // Hash the binary minus its signature.
    let hash = blake3::hash(&exe_data);

    let verifying_key = VerifyingKey::from_bytes(&AUTHOR_PUBKEY)
        .map_err(|e| format!("Invalid embedded public key: {}", e))?;

    verifying_key
        .verify(hash.as_bytes(), &signature)
        .map_err(|_| "Signature verification failed — binary corrupted or modified".to_string())?;

    Ok(hex::encode(signature.to_bytes()).to_uppercase())
}
