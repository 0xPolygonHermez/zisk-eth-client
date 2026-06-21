#!/usr/bin/env python3
"""Wrap input .bin files with an outer 8-byte length prefix.

Each file goes from:
  [8B len1][pub bytes][pad][8B len2][wit bytes][pad]
to:
  [8B outer_len][8B len1][pub bytes][pad][8B len2][wit bytes][pad]

where outer_len == original file size. This allows read_input() to return
the full file content as a single record payload. Idempotent.
"""
import struct
import sys
from pathlib import Path


def is_wrapped(data: bytes) -> bool:
    if len(data) < 8:
        return False
    outer_len = struct.unpack_from("<Q", data, 0)[0]
    return outer_len == len(data) - 8


def wrap_file(path: Path) -> None:
    data = path.read_bytes()
    if is_wrapped(data):
        print(f"  already wrapped: {path.name}")
        return
    path.write_bytes(struct.pack("<Q", len(data)) + data)
    print(f"  wrapped: {path.name} ({len(data)} bytes)")


def main() -> None:
    inputs_dir = Path(__file__).parent / "inputs"
    if not inputs_dir.exists():
        print(f"error: directory not found: {inputs_dir}", file=sys.stderr)
        sys.exit(1)

    files = sorted(inputs_dir.glob("*.bin"))
    if not files:
        print(f"error: no .bin files in {inputs_dir}", file=sys.stderr)
        sys.exit(1)

    for f in files:
        wrap_file(f)


if __name__ == "__main__":
    main()
