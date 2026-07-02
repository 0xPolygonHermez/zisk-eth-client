#!/usr/bin/env python3
"""Render a markdown diff of `ziskemu -X` reports between two branches.

Reads per-(client, block) report files produced by zec_bench.sh from a base dir
and a PR dir, parses the STEPS + COST DISTRIBUTION numbers, and prints a markdown
comment: a summary table (headline Steps + Total Cost per guest+block) followed
by a collapsible full cost breakdown for each.

Usage: zec_cycle_diff.py <BASE_DIR> <PR_DIR>
"""
import os
import re
import sys

# Report label -> friendly row name, in display order. Keys must match the
# leading label of each line emitted by the emulator's stats report().
ROWS = [
    ("STEPS", "Total Steps"),
    ("MAIN", "Main Cost"),
    ("OPCODES", "Opcodes Cost"),
    ("PRECOMPILES", "Precompiles Cost"),
    ("MEMORY", "Memory Cost"),
    ("VARIABLE", "Variable Cost"),
    ("BASE", "Base Cost"),
    ("TOTAL", "Total Cost"),
    ("FROPS", "Frops Cost"),
]

LINE_RE = re.compile(r"^\s*([A-Z]+)\s+([\d,]+)")


def parse_report(path):
    """Parse a ziskemu -X report into {LABEL: int}. Missing file -> {}."""
    out = {}
    if not os.path.isfile(path):
        return out
    wanted = {label for label, _ in ROWS}
    with open(path) as f:
        for line in f:
            m = LINE_RE.match(line)
            if m and m.group(1) in wanted:
                # First occurrence wins (the summary section comes first).
                out.setdefault(m.group(1), int(m.group(2).replace(",", "")))
    return out


def fmt(n):
    return f"{n:,}" if n is not None else "N/A"


def delta(b, p):
    """Return a signed percentage with a color indicator, or N/A.

    🔴 increase (regression — cost went up), 🟢 decrease (improvement),
    ➖ no change. Lower is better, so a positive diff is a regression.
    """
    if b is None or p is None:
        return "N/A"
    d = p - b
    if b == 0:
        if d == 0:
            return "➖ 0.00%"
        return "🔴 new" if d > 0 else "🟢 —"
    pct = d / b * 100
    # A diff that rounds to 0.00% is reported as no-change.
    if round(pct, 2) == 0:
        return "➖ 0.00%"
    dot = "🔴" if d > 0 else "🟢"
    return f"{dot} {pct:+.2f}%"


def split_prog(prog):
    """Split a report stem into (guest, input): 'reth_24647140' -> ('reth', '24647140')."""
    client, _, block = prog.partition("_")
    return client, block


BASELINE_CLIENT = "reth"


def vs_baseline(baseline, value):
    """Cross-client comparison: PR `value` relative to the baseline client's PR
    value on the same block. 'baseline' for the reference client itself, a signed
    percentage otherwise, or N/A when either side is missing."""
    if baseline is None or value is None:
        return "N/A"
    if baseline == value:
        return "0.00%"
    if baseline == 0:
        return "N/A"
    return f"{(value - baseline) / baseline * 100:+.2f}%"


def summary(rows):
    """Headline table: per guest + input, the PR Steps/Total Cost, the Δ vs the
    base branch (regression check), and a cross-client comparison vs the baseline
    client (reth) on the same block."""
    # PR STEPS/TOTAL for the baseline client, keyed by block.
    base_steps = {
        block: pr.get("STEPS")
        for client, block, _b, pr in rows
        if client == BASELINE_CLIENT
    }
    base_total = {
        block: pr.get("TOTAL")
        for client, block, _b, pr in rows
        if client == BASELINE_CLIENT
    }

    out = [
        f"| Guest | Input | Steps | Δ Steps | Steps vs {BASELINE_CLIENT} "
        f"| Total Cost | Δ Total Cost | Cost vs {BASELINE_CLIENT} |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for client, block, base, pr in rows:
        if client == BASELINE_CLIENT:
            steps_vs = cost_vs = "baseline"
        else:
            steps_vs = vs_baseline(base_steps.get(block), pr.get("STEPS"))
            cost_vs = vs_baseline(base_total.get(block), pr.get("TOTAL"))
        out.append(
            f"| {client} | {block} "
            f"| {fmt(pr.get('STEPS'))} | {delta(base.get('STEPS'), pr.get('STEPS'))} | {steps_vs} "
            f"| {fmt(pr.get('TOTAL'))} | {delta(base.get('TOTAL'), pr.get('TOTAL'))} | {cost_vs} |"
        )
    return "\n".join(out)


def breakdown(client, block, base, pr):
    """Full per-row table for one guest + input, wrapped in a collapsible section."""
    lines = [
        "| Metric | Base Branch | Current PR | Diff | Diff (%) |",
        "| --- | --- | --- | --- | --- |",
    ]
    for label, name in ROWS:
        b = base.get(label)
        p = pr.get(label)
        if b is None and p is None:
            continue
        if b is None or p is None:
            diff = pct = "N/A"
        else:
            d = p - b
            diff = f"{d:,}"
            pct = f"{(d / b * 100):.2f}%" if b != 0 else "N/A"
        lines.append(f"| {name} | {fmt(b)} | {fmt(p)} | {diff} | {pct} |")
    table = "\n".join(lines)
    return (
        f"<details>\n<summary><b>{client} ({block})</b></summary>\n\n"
        f"{table}\n\n"
        "</details>"
    )


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    base_dir, pr_dir = sys.argv[1], sys.argv[2]

    # Programs = the .txt reports present in the PR dir (fall back to base dir).
    src = pr_dir if os.path.isdir(pr_dir) else base_dir
    programs = [f[:-4] for f in os.listdir(src) if f.endswith(".txt")]

    rows = [
        (
            *split_prog(prog),
            parse_report(os.path.join(base_dir, f"{prog}.txt")),
            parse_report(os.path.join(pr_dir, f"{prog}.txt")),
        )
        for prog in programs
    ]
    # Group by block, with the baseline client first in each group, so the
    # cross-client comparison reads top-to-bottom per block.
    rows.sort(key=lambda r: (r[1], r[0] != BASELINE_CLIENT, r[0]))

    out = ["## 🔄 zisk-eth-client Cycle Tracking", ""]
    out.append(
        "Emulator cost report (`ziskemu -X`) per guest client and block input. "
        f"`Δ` columns compare this PR against the base branch (regression check); "
        f"the `vs {BASELINE_CLIENT}` columns compare each client against "
        f"`{BASELINE_CLIENT}` on the same block (lower is cheaper)."
    )
    out.append("")

    if not rows:
        out.append("> ⚠️ No benchmark reports were produced.")
        print("\n".join(out))
        return

    out.append("### Summary")
    out.append("")
    out.append(summary(rows))
    out.append("")
    out.append("### Per-Guest Breakdown")
    out.append("")
    for client, block, base, pr in rows:
        out.append(breakdown(client, block, base, pr))
        out.append("")

    out.append("---")
    out.append(
        "<sub>🔴 increase (regression) · 🟢 decrease (improvement) · ➖ no change. "
        "The emulator is held fixed (pinned ZisK ref), so `STEPS` and all `COSTS` "
        "are deterministic functions of (guest ELF, input).</sub>"
    )

    print("\n".join(out))


if __name__ == "__main__":
    main()