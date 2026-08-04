#!/usr/bin/env bash
# =============================================================================
# NovaiOS — Stage 1: build all Rust userland crates via cargo.
# Outputs:
#   ./build/userland/{bin,lib}/...  — stripped, ready to drop into rootfs.
set -euo pipefail
IFS=$'\n\t'

NOVAI_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$NOVAI_ROOT/build/userland}"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"

cd "$NOVAI_ROOT"
rustup default stable
rustup target add "$TARGET"

echo "::group::Build userland crates (release)"
RUSTFLAGS="-C target-feature=+crt-static -C strip=symbols" \
  cargo build --release --target "$TARGET" --workspace
echo "::endgroup::"

echo "::group::Stage binaries"
mkdir -p "$OUT_DIR/bin" "$OUT_DIR/lib" "$OUT_DIR/share/novai"
for bin in novai-init novai-services novai-shell novai-coreutils novai-pkg \
           novai-comp novai-panel novai-launcher novai-settings novai-lock; do
  src="$NOVAI_ROOT/target/$TARGET/release/$bin"
  [[ -f "$src" ]] && cp "$src" "$OUT_DIR/bin/$bin"
done
cp -r "$NOVAI_ROOT/desktop/novai-theme"/* "$OUT_DIR/share/novai/" 2>/dev/null || true
echo "::endgroup::"

echo "✅ userland built → $OUT_DIR/bin"
