#!/bin/bash
# Cross-build rhe for Linux x86_64 and Windows x86_64, then sign each
# binary with rhe-signature-signer.
#
# macOS arm64 builds via the Mac-side script (./build-release-macos.sh).
#
# ARM Linux is intentionally not built here. The Linux backend pulls
# in GTK (for the tray), which requires a full arm64 sysroot with
# GTK headers/.so files to cross-compile cleanly. That setup is
# project-sized; if/when there's demand for arm64 Linux, do the build
# natively on an arm64 box (or via `cross` with a Docker arm64 image).
#
# Outputs (all under ./dist/):
#   rhe-linux-x86_64-release
#   rhe-windows-x86_64-release.exe
#   rhe-windows-x86_64-release.exe.sha256
#
# Prerequisites (one-time):
#   rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
#   sudo apt install mingw-w64    # (or distro equivalent)
#   ed25519 signing key at $RHE_SIGNING_KEY (or one of the searched defaults)

set -euo pipefail

cd "$(dirname "$0")"

# Build the signing tool once (native target).
echo "Building rhe-signature-signer (native)..."
cargo build --release --bin rhe-signature-signer
SIGNER="./target/release/rhe-signature-signer"

# Cross-compile targets and their resulting paths/names.
declare -a TARGETS=(
    "x86_64-unknown-linux-gnu:rhe:rhe-linux-x86_64-release"
    "x86_64-pc-windows-gnu:rhe.exe:rhe-windows-x86_64-release.exe"
)

mkdir -p dist

for entry in "${TARGETS[@]}"; do
    IFS=':' read -r target src_name dist_name <<< "$entry"
    echo
    echo "── Building rhe for $target ──────────────────────────"
    cargo build --release --bin rhe --target "$target"

    src_path="target/$target/release/$src_name"
    dist_path="dist/$dist_name"

    cp "$src_path" "$dist_path"
    "$SIGNER" "$dist_path"
done

echo
echo "── Done ──────────────────────────────────────────────"
echo "Artifacts:"
ls -lh dist/
