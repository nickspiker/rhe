#!/bin/bash
# Native macOS arm64 release build + sign. Run on a Mac.
#
# Output: ./dist/rhe-macos-arm64-release  (signed in place)
#
# Prerequisites:
#   rustup target add aarch64-apple-darwin
#   ed25519 signing key at $RHE_SIGNING_KEY (or one of the searched defaults
#   — e.g. ~/Library/Application Support/rhe/rhe-signing-key)

set -euo pipefail

cd "$(dirname "$0")"

echo "Building rhe-signature-signer (native)..."
cargo build --release --bin rhe-signature-signer
SIGNER="./target/release/rhe-signature-signer"

echo
echo "── Building rhe for aarch64-apple-darwin ─────────────"
cargo build --release --bin rhe --target aarch64-apple-darwin

mkdir -p dist
cp "target/aarch64-apple-darwin/release/rhe" "dist/rhe-macos-arm64-release"
"$SIGNER" "dist/rhe-macos-arm64-release"

echo
echo "── Done ──────────────────────────────────────────────"
ls -lh dist/rhe-macos-arm64-release
