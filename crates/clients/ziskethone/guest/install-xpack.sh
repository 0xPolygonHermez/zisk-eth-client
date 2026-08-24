#!/usr/bin/env bash
#
# Install the xPack riscv-none-elf-gcc cross-toolchain used to build ziskethone's
# C++ ZisK guest ELF.
#
# The single source of truth for the version and the install prefix: build-elf.sh
# calls it on demand and CI's cpp-guest-toolchain action calls it to warm the
# runner, so the pin lives in exactly one place.
#
# Idempotent: an existing toolchain is detected and the script returns in about a
# second, so it is safe to call from a build script or a CI step.
#
# Usage:
#     install-xpack.sh [--print-prefix]
#
#   --print-prefix   Print the install prefix and exit without installing.
#
# The prefix is echoed on success either way, so a caller can capture it:
#     XPACK_DIR="$(install-xpack.sh --print-prefix)"
#
# Env:
#   ZISK_XPACK_DIR   Override the install prefix (the xPack root, NOT its bin/).

set -euo pipefail

# Pinned, and matched in three places that must agree:
#   - build.rs (has_riscv_toolchain / the default ZISK_TOOLCHAIN_PREFIX)
#   - build-elf.sh (TOOLCHAIN_PREFIX default)
#   - ziskethone's cpp-guest/patches/gcc/build-toolchain.sh, which symlinks this
#     toolchain's C++ headers and binutils into the patched GCC. That is the
#     reason the version is exact rather than a floor: the patched compiler is
#     GCC 14.3.0, so its target side must come from a 14.3.0 xPack.
XPACK_VERSION="14.3.0-1"
PREFIX="${ZISK_XPACK_DIR:-$HOME/opt/xpack/xpack-riscv-none-elf-gcc-${XPACK_VERSION}}"

if [ "${1:-}" = "--print-prefix" ]; then
    printf '%s\n' "$PREFIX"
    exit 0
fi
if [ -n "${1:-}" ]; then
    echo "unknown arg: $1 (try --print-prefix)" >&2
    exit 2
fi

if [ -x "$PREFIX/bin/riscv-none-elf-gcc" ]; then
    echo "==> xPack RISC-V toolchain already present at $PREFIX" >&2
    printf '%s\n' "$PREFIX"
    exit 0
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)   XPACK_HOST="linux-x64" ;;
    Linux-aarch64)  XPACK_HOST="linux-arm64" ;;
    Darwin-x86_64)  XPACK_HOST="darwin-x64" ;;
    Darwin-arm64)   XPACK_HOST="darwin-arm64" ;;
    *)
        echo "ERROR: unsupported platform $(uname -s)-$(uname -m)." >&2
        echo "       Install xPack riscv-none-elf-gcc ${XPACK_VERSION} manually into" >&2
        echo "       $PREFIX, or point ZISK_TOOLCHAIN_PREFIX at its bin/ dir." >&2
        exit 1
        ;;
esac

URL="https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v${XPACK_VERSION}/xpack-riscv-none-elf-gcc-${XPACK_VERSION}-${XPACK_HOST}.tar.gz"

# The archive's top-level directory is xpack-riscv-none-elf-gcc-<version>, so it
# extracts into the parent and lands on PREFIX. A ZISK_XPACK_DIR whose basename
# differs would therefore not receive the files — refuse rather than extract to a
# path the caller is not expecting.
PARENT="$(dirname "$PREFIX")"
EXPECTED_BASENAME="xpack-riscv-none-elf-gcc-${XPACK_VERSION}"
if [ "$(basename "$PREFIX")" != "$EXPECTED_BASENAME" ]; then
    echo "ERROR: ZISK_XPACK_DIR must end in $EXPECTED_BASENAME (the archive's own" >&2
    echo "       top-level directory); got $(basename "$PREFIX")." >&2
    exit 1
fi

echo "==> Installing xPack RISC-V toolchain (${XPACK_HOST}) into $PARENT" >&2
for tool in curl tar; do
    command -v "$tool" >/dev/null || { echo "ERROR: missing required tool: $tool" >&2; exit 1; }
done
mkdir -p "$PARENT"
# --no-same-owner / --no-same-permissions: apply the current user's ownership and
# umask to the extracted files rather than trusting the archive's metadata.
curl -fsSL --retry 3 "$URL" \
    | tar -xz --no-same-owner --no-same-permissions -C "$PARENT"

[ -x "$PREFIX/bin/riscv-none-elf-gcc" ] \
    || { echo "ERROR: install completed but $PREFIX/bin/riscv-none-elf-gcc is missing" >&2; exit 1; }

"$PREFIX/bin/riscv-none-elf-g++" --version | head -1 >&2
printf '%s\n' "$PREFIX"
