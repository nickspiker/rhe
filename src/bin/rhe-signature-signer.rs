//! Sign a rhe binary with an Ed25519 signature appended to its tail.
//!
//! Usage: `rhe-signature-signer <binary-path>`
//!
//! Looks for the private key at `$RHE_SIGNING_KEY`, falling back to a
//! short list of common locations. Idempotent: re-running on a binary
//! already signed by the same key is a no-op.
//!
//! For Windows binaries (`.exe`), also writes a `<binary>.sha256` file
//! containing the SHA-256 of the signed binary, used by the PowerShell
//! installer to validate the download (Defender prefers a hash check
//! over running an unknown binary just to verify itself).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <binary-path>", args[0]);
        eprintln!();
        eprintln!("Set RHE_SIGNING_KEY=/path/to/rhe-signing-key, or place");
        eprintln!("the key at one of the searched default locations.");
        std::process::exit(1);
    }

    let binary_path = &args[1];
    println!("Signing binary: {}", binary_path);

    let mut binary_data = fs::read(binary_path)?;
    println!("  Binary size: {} bytes", binary_data.len());

    // Find the private key — env var first, then common locations.
    let private_key_path = if let Ok(env_path) = env::var("RHE_SIGNING_KEY") {
        let p = PathBuf::from(&env_path);
        if !p.exists() {
            eprintln!();
            eprintln!("ERROR: RHE_SIGNING_KEY set but file not found: {}", env_path);
            std::process::exit(1);
        }
        p
    } else {
        let home = env::var("HOME").unwrap_or_default();
        let candidates = [
            format!("{}/.rhe-keys/rhe-signing-key", home),
            format!("{}/MEGA/Code/keys/rhe-signing-key", home),
            format!("{}/Code/keys/rhe-signing-key", home),
            "/mnt/Octopus/Code/keys/rhe-signing-key".to_string(),
            "/mnt/Chiton/MEGA/Code/keys/rhe-signing-key".to_string(),
            format!(
                "{}/Library/Application Support/rhe/rhe-signing-key",
                home
            ),
        ];
        match candidates.iter().map(PathBuf::from).find(|p| p.exists()) {
            Some(p) => p,
            None => {
                eprintln!();
                eprintln!("ERROR: Private key not found. Searched:");
                for c in &candidates {
                    eprintln!("  {}", c);
                }
                eprintln!();
                eprintln!("Set RHE_SIGNING_KEY=/path/to/key or place a key in one of the above.");
                std::process::exit(1);
            }
        }
    };

    let private_key_bytes = fs::read(&private_key_path)?;
    let signing_key = SigningKey::from_bytes(
        private_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid private key length")?,
    );
    let verifying_key = signing_key.verifying_key();

    // If the binary already carries a valid signature from this key,
    // skip — re-signing would just replace it with an equivalent one.
    if binary_data.len() >= 64 {
        let sig_bytes = binary_data.split_off(binary_data.len() - 64);
        let sig = Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap_or(&[0u8; 64]));
        let hash = blake3::hash(&binary_data);
        if verifying_key.verify(hash.as_bytes(), &sig).is_ok() {
            println!();
            println!("⚠ Binary is already signed with this key. Skipping.");
            // Still write the .sha256 sidecar for Windows installers.
            if binary_path.ends_with(".exe") {
                binary_data.extend_from_slice(&sig_bytes);
                write_sha256_sidecar(binary_path, &binary_data)?;
            }
            return Ok(());
        }
        // Not our signature — restore so we hash the full binary
        // (signatures from other keys are part of the bytes-to-sign).
        binary_data.extend_from_slice(&sig_bytes);
    }

    let hash = blake3::hash(&binary_data);
    println!(
        "  BLAKE3 hash: {}",
        hex::encode(hash.as_bytes()).to_uppercase()
    );

    let signature: Signature = signing_key.sign(hash.as_bytes());
    println!(
        "  Ed25519 signature: {}",
        hex::encode(signature.to_bytes()).to_uppercase()
    );

    let mut signed_binary = binary_data;
    signed_binary.extend_from_slice(&signature.to_bytes());
    fs::write(binary_path, &signed_binary)?;

    println!();
    println!(
        "✓ Signature appended (+{} bytes → {} bytes total)",
        64,
        signed_binary.len()
    );

    if binary_path.ends_with(".exe") {
        write_sha256_sidecar(binary_path, &signed_binary)?;
    }

    Ok(())
}

fn write_sha256_sidecar(binary_path: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let sha256_hex = hex::encode(hasher.finalize()).to_uppercase();
    let sha_path = format!("{}.sha256", binary_path);
    fs::write(&sha_path, &sha256_hex)?;
    println!("  SHA256: {}", sha256_hex);
    println!("  Written to: {}", sha_path);
    Ok(())
}
