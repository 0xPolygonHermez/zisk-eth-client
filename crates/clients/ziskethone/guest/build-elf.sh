#!/usr/bin/env bash
# Build ziskethone's C++ ZisK guest ELF (cpp-guest/zisk).
#
# Usage:
#   ./crates/clients/ziskethone/guest/build-elf.sh [--clean] [--ziskethone=PATH]
#                                           [--toolchain-prefix=PATH]
#
# Output: <ziskethone>/cpp-guest/zisk/build/zisk_eth_guest.elf

set -euo pipefail

# Default matches build.rs: the third_party/ziskethone submodule.
# ZISKETHONE_DIR overrides (e.g. a local working checkout of ziskethone).
ZISKETHONE_DIR="${ZISKETHONE_DIR:-$(cd "$(dirname "$0")/../../../.." && pwd)/third_party/ziskethone}"
TOOLCHAIN_PREFIX="${ZISK_TOOLCHAIN_PREFIX:-$HOME/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin}"
CLEAN=0

for arg in "$@"; do
    case "$arg" in
        --clean) CLEAN=1 ;;
        --ziskethone=*) ZISKETHONE_DIR="${arg#*=}" ;;
        --toolchain-prefix=*) TOOLCHAIN_PREFIX="${arg#*=}" ;;
        -h|--help)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            echo "unknown arg: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

GUEST_DIR="$ZISKETHONE_DIR/cpp-guest/zisk"
[ -f "$GUEST_DIR/CMakeLists.txt" ] \
    || { echo "ziskethone cpp-guest/zisk not found at $GUEST_DIR" >&2; exit 1; }
[ -x "$TOOLCHAIN_PREFIX/riscv-none-elf-g++" ] \
    || { echo "riscv-none-elf-g++ not found in $TOOLCHAIN_PREFIX" >&2; exit 1; }

# toolchain.cmake auto-detects riscv-none-elf-* on PATH.
export PATH="$TOOLCHAIN_PREFIX:$PATH"

BUILD_DIR="$GUEST_DIR/build"
ELF_PATH="$BUILD_DIR/zisk_eth_guest.elf"

if [ "$CLEAN" = 1 ]; then
    echo "==> removing $BUILD_DIR"
    rm -rf "$BUILD_DIR"
fi

# The host cmake configure fetches evmone AND applies cpp-guest/patches/* to it.
# Always re-run it: the patch list grows, so a warm _deps from an older revision
# would silently build an ELF missing newer patches. Cheap — FetchContent caches
# the download and the patch loop skips already-applied patches.
#
# Use our own build-stub rather than the developer's cpp-guest/build, and pin the
# guest to it with -DEVMONE_SRC so the tree we patched is the tree we compile.
HOST_DIR="$ZISKETHONE_DIR/cpp-guest"
echo "==> host cmake configure (fetch evmone, apply patches)"
cmake -S "$HOST_DIR" -B "$HOST_DIR/build-stub"

echo "==> configuring cmake"
cmake \
    -S "$GUEST_DIR" \
    -B "$BUILD_DIR" \
    -DCMAKE_TOOLCHAIN_FILE="$GUEST_DIR/toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release \
    -DEVMONE_SRC="$HOST_DIR/build-stub/_deps/evmone-src" \
    -G "Unix Makefiles"

echo "==> building zisk_eth_guest.elf"
cmake --build "$BUILD_DIR" --target zisk_eth_guest.elf -j"$(nproc)"

[ -f "$ELF_PATH" ] \
    || { echo "build finished but ELF not at $ELF_PATH" >&2; exit 1; }

echo
echo "ELF: $ELF_PATH"
echo "Run with: ziskemu -e $ELF_PATH -i <input.bin>"
