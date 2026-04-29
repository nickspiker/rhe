#!/bin/bash
# Multi-target release build + sign for rhe.
#
# All three targets cross-built from a single Linux host:
#   - x86_64-unknown-linux-gnu       (native)
#   - x86_64-pc-windows-gnu          (mingw)
#   - aarch64-apple-darwin           (osxcross)
#
# Each binary is signed in place with rhe-signature-signer; the .exe
# also gets a .sha256 sidecar for the PowerShell installer's hash gate.
#
# Outputs (all under ./dist/):
#   rhe-linux-x86_64-release
#   rhe-windows-x86_64-release.exe
#   rhe-windows-x86_64-release.exe.sha256
#   rhe-macos-arm64-release
#
# Prerequisites (all already present on the Octopus dev box):
#   rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu aarch64-apple-darwin
#   sudo apt install mingw-w64    # (or distro equivalent for windows-gnu)
#   osxcross at /mnt/Octopus/Code/osxcross with macOS SDK
#   ed25519 signing key at $RHE_SIGNING_KEY (or one of the searched defaults)
#
# ARM Linux is intentionally out of scope: the Linux backend pulls in
# GTK + xkbcommon, which would need a full arm64 sysroot with those
# headers. If/when there's demand, build natively on an arm64 box (or
# via `cross` with a Docker arm64 image).

set -euo pipefail
cd "$(dirname "$0")"

OSXCROSS_BIN="/mnt/Octopus/Code/osxcross/target/bin"

# Build the signing tool once (native target).
echo "Building rhe-signature-signer (native)..."
cargo build --release --bin rhe-signature-signer
SIGNER="./target/release/rhe-signature-signer"

mkdir -p dist

# 1) Linux x86_64 (native).
echo
echo "── Building rhe for x86_64-unknown-linux-gnu ─────────"
cargo build --release --bin rhe --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/rhe dist/rhe-linux-x86_64-release
"$SIGNER" dist/rhe-linux-x86_64-release

# 2) Windows x86_64 (mingw).
echo
echo "── Building rhe for x86_64-pc-windows-gnu ────────────"
cargo build --release --bin rhe --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/rhe.exe dist/rhe-windows-x86_64-release.exe
"$SIGNER" dist/rhe-windows-x86_64-release.exe

# 3) macOS arm64 via osxcross. CC/CXX needed for cc-rs build deps;
#    CARGO_TARGET_..._LINKER replaces .cargo/config.toml's default
#    "clang" (which would invoke the host's clang and hit a Linux
#    sysroot, not the macOS one).
echo
echo "── Building rhe for aarch64-apple-darwin ─────────────"
CC_aarch64_apple_darwin="$OSXCROSS_BIN/aarch64-apple-darwin-clang-wrapper" \
CXX_aarch64_apple_darwin="$OSXCROSS_BIN/aarch64-apple-darwin-clang-wrapper" \
CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="$OSXCROSS_BIN/aarch64-apple-darwin-clang-wrapper" \
    cargo build --release --bin rhe --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/rhe dist/rhe-macos-arm64-release
"$SIGNER" dist/rhe-macos-arm64-release

echo
echo "── Done ──────────────────────────────────────────────"
echo "Artifacts:"
ls -lh dist/
echo
echo "Windows SHA256:"
cat dist/rhe-windows-x86_64-release.exe.sha256
