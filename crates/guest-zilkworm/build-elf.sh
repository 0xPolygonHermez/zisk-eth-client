#!/usr/bin/env bash
# Build zilkworm's C++ ZisK guest ELF.
#
# Usage:
#   ./crates/guest-zilkworm/build-elf.sh [--clean] [--zilkworm=PATH]
#                                        [--toolchain-prefix=PATH]
#
# Output: <zilkworm>/prover/guest_zisk/build/z6m_guest.elf

set -euo pipefail

ZILKWORM_DIR="${ZILKWORM_DIR:-$(cd "$(dirname "$0")/../.." && pwd)/third_party/zilkworm}"
TOOLCHAIN_PREFIX="${ZISK_TOOLCHAIN_PREFIX:-$HOME/opt/xpack/xpack-riscv-none-elf-gcc-15.2.0-1/bin}"
CLEAN=0

for arg in "$@"; do
    case "$arg" in
        --clean) CLEAN=1 ;;
        --zilkworm=*) ZILKWORM_DIR="${arg#*=}" ;;
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

[ -d "$ZILKWORM_DIR/prover/guest_zisk" ] \
    || { echo "zilkworm not found at $ZILKWORM_DIR" >&2; exit 1; }
[ -x "$TOOLCHAIN_PREFIX/riscv-none-elf-gcc" ] \
    || { echo "riscv-none-elf-gcc not found in $TOOLCHAIN_PREFIX" >&2; exit 1; }

BUILD_DIR="$ZILKWORM_DIR/prover/guest_zisk/build"
ELF_PATH="$BUILD_DIR/z6m_guest.elf"

if [ "$CLEAN" = 1 ]; then
    echo "==> removing $BUILD_DIR"
    rm -rf "$BUILD_DIR"
fi

# evmone's submodule URL is SSH; fall back to HTTPS if no key is available.
if [ ! -f "$ZILKWORM_DIR/third_party/intx/CMakeLists.txt" ] \
   || [ ! -f "$ZILKWORM_DIR/third_party/evmone/CMakeLists.txt" ]; then
    echo "==> initializing zilkworm submodules"
    pushd "$ZILKWORM_DIR" > /dev/null
    if ! git submodule update --init --recursive third_party/intx third_party/evmone third_party/eest-fixtures 2>&1; then
        echo "==> SSH clone failed, retrying with HTTPS override"
        git config -f .gitmodules submodule.evmone.url https://github.com/RogerTaule/zvm1.git
        git submodule sync third_party/evmone
        git submodule update --init --recursive third_party/intx third_party/evmone third_party/eest-fixtures
        git checkout -- .gitmodules
    fi
    popd > /dev/null
fi

echo "==> configuring cmake"
# Use the default "Unix Makefiles" generator: make ships with essentially
# every toolchain, so the build has no dependency beyond cmake + the
# cross-compiler.
ZISK_TOOLCHAIN_PREFIX="$TOOLCHAIN_PREFIX" cmake \
    -S "$ZILKWORM_DIR/prover/guest_zisk" \
    -B "$BUILD_DIR" \
    -DCMAKE_TOOLCHAIN_FILE="$ZILKWORM_DIR/prover/guest_zisk/cmake/zisk-toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release \
    -G "Unix Makefiles"

echo "==> building z6m_guest.elf"
cmake --build "$BUILD_DIR" --target z6m_guest.elf -j"$(nproc)"

[ -f "$ELF_PATH" ] \
    || { echo "build finished but ELF not at $ELF_PATH" >&2; exit 1; }

echo
echo "ELF: $ELF_PATH"
echo "Run with: ziskemu -e $ELF_PATH -i <input.bin>"
