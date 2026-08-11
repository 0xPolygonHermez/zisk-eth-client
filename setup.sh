#!/usr/bin/env bash
#
# setup.sh — one-shot bootstrap for a fresh clone. Idempotent: safe to re-run.
#
# Does the two things a fresh checkout can't build without:
#   1. Initializes the third_party/ziskethone submodule. It's a Cargo `path`
#      dependency (see Cargo.toml), so the root workspace won't even resolve
#      until its files exist on disk.
#   2. Installs the xPack RISC-V bare-metal GCC toolchain (riscv-none-elf-gcc)
#      used to cross-compile the ziskethone C++ guest. Distro packages are too
#      old. The version is pinned to 14.3.0-1 because the patched GCC that
#      provides -mzisk-dma (step 3) reuses this toolchain's C++ headers and
#      binutils, and the versions have to match exactly.
#
# The zisk Rust toolchain (for the reth/ethrex Rust guests) is separate — install
# it with `cargo-zisk toolchain install`; see the ZisK docs.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# --- 1. Submodules ----------------------------------------------------------
echo "==> Initializing git submodules"
git submodule update --init --recursive

# --- 2. xPack RISC-V toolchain ----------------------------------------------
# Version + install dir MUST match crates/clients/ziskethone/guest/build.rs
# (has_riscv_toolchain / the default ZISK_TOOLCHAIN_PREFIX).
XPACK_VERSION="14.3.0-1"
XPACK_DIR="$HOME/opt/xpack/xpack-riscv-none-elf-gcc-${XPACK_VERSION}"

if [ -x "$XPACK_DIR/bin/riscv-none-elf-gcc" ]; then
  echo "==> xPack RISC-V toolchain already present at $XPACK_DIR"
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)   XPACK_HOST="linux-x64" ;;
    Linux-aarch64)  XPACK_HOST="linux-arm64" ;;
    Darwin-x86_64)  XPACK_HOST="darwin-x64" ;;
    Darwin-arm64)   XPACK_HOST="darwin-arm64" ;;
    *) echo "ERROR: unsupported platform $(uname -s)-$(uname -m); install xPack riscv-none-elf-gcc ${XPACK_VERSION} manually" >&2; exit 1 ;;
  esac
  URL="https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v${XPACK_VERSION}/xpack-riscv-none-elf-gcc-${XPACK_VERSION}-${XPACK_HOST}.tar.gz"
  echo "==> Installing xPack RISC-V toolchain (${XPACK_HOST}) into $HOME/opt/xpack"
  mkdir -p "$HOME/opt/xpack"
  # --no-same-owner / --no-same-permissions: apply the current user's ownership
  # and umask to the extracted files rather than trusting the archive's metadata.
  curl -fsSL "$URL" | tar -xz --no-same-owner --no-same-permissions -C "$HOME/opt/xpack"
  [ -x "$XPACK_DIR/bin/riscv-none-elf-gcc" ] \
    || { echo "ERROR: install completed but $XPACK_DIR/bin/riscv-none-elf-gcc is missing" >&2; exit 1; }
fi

# --- 3. Patched GCC for -mzisk-dma ------------------------------------------
# The guest ELF is always built with -mzisk-dma, which the stock compiler does
# not have. This builds it once (~10 min) from a pristine GCC 14.3.0 tarball
# plus ziskethone's patch; re-runs detect it and return immediately.
echo "==> Building the patched GCC for -mzisk-dma (first run takes ~10 min)"
PATH="$XPACK_DIR/bin:$PATH" ZISK_XPACK_DIR="$XPACK_DIR" \
  third_party/ziskethone/cpp-guest/patches/gcc/build-toolchain.sh

echo
echo "Setup complete. You can now build the guests, e.g.:"
echo "    cd bin/guests/stateless-validator-reth && cargo-zisk build --release"
