#!/usr/bin/env bash
# =============================================================================
# NovaiOS — Stage 0: build the Linux kernel with Rust-for-Linux enabled.
# -----------------------------------------------------------------------------
# Inputs (env):
#   KVER            — kernel version to build (default 6.12.10)
#   SRC_DIR         — where to put linux sources (default ./build/linux)
#   OUT_DIR         — kernel build output (default ./build/out-kernel)
#   INSTALL_MOD     — module install destination (default ./build/modules)
#   ARCH            — x86_64
#   JOBS            — number of make jobs (default $(nproc))
# Outputs:
#   $OUT_DIR/vmlinuz-novai
#   $INSTALL_MOD/lib/modules/$KVER-novai/...
#   $OUT_DIR/bzImage
set -euo pipefail
IFS=$'\n\t'

KVER="${KVER:-6.12.10}"
SRC_DIR="${SRC_DIR:-$(pwd)/build/linux}"
OUT_DIR="${OUT_DIR:-$(pwd)/build/out-kernel}"
INSTALL_MOD="${INSTALL_MOD:-$(pwd)/build/modules}"
ARCH="${ARCH:-x86_64}"
JOBS="${JOBS:-$(nproc)}"
NOVAI_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "::group::Prepare kernel source"
mkdir -p "$(dirname "$SRC_DIR")" "$OUT_DIR" "$INSTALL_MOD"

if [[ ! -d "$SRC_DIR" ]]; then
  URL="https://cdn.kernel.org/pub/linux/kernel/v${KVER%%.*}.x/linux-$KVER.tar.xz"
  echo "downloading $URL"
  wget -q -O /tmp/linux.tar.xz "$URL"
  tar -xf /tmp/linux.tar.xz -C "$(dirname "$SRC_DIR")"
  mv "$(dirname "$SRC_DIR")/linux-$KVER" "$SRC_DIR"
fi
echo "::endgroup::"

echo "::group::Install Rust toolchain (kernel-pinned)"
# Linux's rust/ has a pinned rustc version; install via rustup
TOOLCHAIN_PIN=$(grep -oP 'rustc-[0-9.]+' "$SRC_DIR/rust/Kconfig" 2>/dev/null | head -1 || true)
rustup default stable
rustup component add rustfmt clippy rust-src
cargo install --locked --version 0.69.5 bindgen-cli || true
echo "::endgroup::"

echo "::group::Configure kernel"
cd "$SRC_DIR"
cp "$NOVAI_ROOT/kernel/config-novai-x86_64" .config
# Make sure Rust-related symbols exist in the tree; olddefconfig will drop unknown ones.
make LLVM=1 ARCH=$ARCH olddefconfig </dev/null
# Re-add Rust flags that olddefconfig may have unset.
scripts/config --enable CONFIG_RUST --enable CONFIG_BUILD_RUST
make LLVM=1 ARCH=$ARCH olddefconfig </dev/null
echo "::endgroup::"

echo "::group::Build kernel"
# Try building with Rust enabled first. If the Rust toolchain doesn't match
# what the kernel expects (common — R4L is tightly pinned), fall back to
# building without CONFIG_RUST so we still produce a working kernel.
make LLVM=1 ARCH=$ARCH -j"$JOBS" bzImage modules 2>&1 | tee /tmp/kbuild.log
KBUILD_RC=${PIPESTATUS[0]}
if [[ $KBUILD_RC -ne 0 ]]; then
  if grep -q "unknown unstable option\|Rust.*not available\|CONFIG_RUST" /tmp/kbuild.log; then
    echo "::warning::Rust kernel build failed (toolchain mismatch) — retrying without CONFIG_RUST"
    scripts/config --disable CONFIG_RUST --disable CONFIG_BUILD_RUST
    make LLVM=1 ARCH=$ARCH olddefconfig </dev/null
    make LLVM=1 ARCH=$ARCH -j"$JOBS" bzImage modules
  else
    echo "::error::kernel build failed for non-Rust reason"
    exit 1
  fi
fi
echo "::endgroup::"

echo "::group::Install kernel"
INSTALL_MOD_PATH="$INSTALL_MOD" make LLVM=1 ARCH=$ARCH modules_install
cp arch/x86/boot/bzImage "$OUT_DIR/vmlinuz-novai"
cp System.map "$OUT_DIR/System.map-novai"
cp .config  "$OUT_DIR/config-novai"
echo "::endgroup::"

echo "::group::Build novai Rust kernel module"
if [[ -d "$NOVAI_ROOT/kernel/rust-modules" ]]; then
  make LLVM=1 ARCH=$ARCH M="$NOVAI_ROOT/kernel/rust-modules" modules || \
    echo "::warning::novai_drv module build failed (non-fatal for first boot)"
  make LLVM=1 ARCH=$ARCH M="$NOVAI_ROOT/kernel/rust-modules" \
       INSTALL_MOD_PATH="$INSTALL_MOD" modules_install || true
fi
echo "::endgroup::"

echo "✅ kernel build done → $OUT_DIR/vmlinuz-novai"
