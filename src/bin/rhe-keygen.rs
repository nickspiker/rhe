//! Generate Ed25519 keypair for signing rhe release binaries.
//!
//! Stores the private key unencrypted (filesystem encryption assumed). After running this, copy the printed public key into `src/crypto/self_verify.rs` so the resulting binary self-verifies.
//!
//! Usage: `cargo run --bin rhe-keygen [--out-dir <path>]` (default out-dir: $HOME/.rhe-keys)

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rhe key generator");
    println!("=================");
    println!();

    // CLI: --out-dir <path>, falling back to $HOME/.rhe-keys.
    let args: Vec<String> = env::args().collect();
    let mut out_dir: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--out-dir" && i + 1 < args.len() {
            out_dir = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else {
            eprintln!("Unknown argument: {}", args[i]);
            std::process::exit(2);
        }
    }
    let keys_dir = out_dir.unwrap_or_else(|| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".rhe-keys")
    });

    if !keys_dir.exists() {
        fs::create_dir_all(&keys_dir)?;
        println!("Created keys directory: {}", keys_dir.display());
    }

    let private_key_path = keys_dir.join("rhe-signing-key");
    let public_key_path = keys_dir.join("rhe-signing-key.pub");

    if private_key_path.exists() || public_key_path.exists() {
        eprintln!("ERROR: Keys already exist:");
        eprintln!("  Private: {}", private_key_path.display());
        eprintln!("  Public:  {}", public_key_path.display());
        eprintln!("Delete them first if you really want to regenerate.");
        std::process::exit(1);
    }

    println!("Generating Ed25519 keypair...");
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    fs::write(&private_key_path, signing_key.to_bytes())?;
    println!("✓ Private key saved: {}", private_key_path.display());

    fs::write(&public_key_path, verifying_key.to_bytes())?;
    println!("✓ Public key saved:  {}", public_key_path.display());

    let pubkey_hex = hex::encode(verifying_key.to_bytes());
    println!();
    println!("Public key (hex): {}", pubkey_hex);
    println!();
    println!("Next steps:");
    println!("  1. Copy this pubkey into src/crypto/self_verify.rs as AUTHOR_PUBKEY");
    println!(
        "  2. Set RHE_SIGNING_KEY={} when running rhe-signature-signer",
        private_key_path.display()
    );
    println!("  3. KEEP THE PRIVATE KEY SECURE.");

    Ok(())
}
