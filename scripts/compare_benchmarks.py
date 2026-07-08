#!/usr/bin/env python3
# A5
# SPDX-License-Identifier: Apache-2.0
# Copyright (c) A5 contributors
#
# Compares two criterion baselines (saved with `cargo bench -- --save-baseline
# <name>`) and exits non-zero if any benchmark regressed by more than the
# threshold.
#
# Usage:
#   python scripts/compare_benchmarks.py <baseline_name> <current_name> [threshold%] [criterion_dir]
#
# Comparison keys off criterion's MEDIAN point estimate. criterion fits a
# distribution over many samples and reports the median as its most
# outlier-robust central estimate, so it is the stable metric to diff between
# two runs on shared CI hardware (the mean is more perturbed by GC/scheduler
# hiccups). This mirrors the other ports keying off the minimum sample.
#
# Output is GitHub-flavored markdown for $GITHUB_STEP_SUMMARY: regressions and
# gains beyond the threshold are surfaced in their own tables at the top, with
# the full results in a collapsed <details> section below.

import json
import sys
from pathlib import Path


def format_time(ns):
    if ns < 1e3:
        return f"{ns:.1f}ns"
    if ns < 1e6:
        return f"{ns / 1e3:.2f}µs"
    if ns < 1e9:
        return f"{ns / 1e6:.2f}ms"
    return f"{ns / 1e9:.2f}s"


def format_delta(delta):
    return f"{'+' if delta >= 0 else ''}{delta:.1f}%"


def render_table(rows):
    lines = ["| benchmark | baseline | current | change |", "| --- | ---: | ---: | ---: |"]
    for r in rows:
        lines.append(f"| {r['name']} | {r['baseline']} | {r['current']} | {r['change']} |")
    return lines


def load(root, name):
    # target/criterion/<benchmark id.../><name>/estimates.json
    out = {}
    for est in root.glob(f"**/{name}/estimates.json"):
        bench_id = str(est.parent.parent.relative_to(root))
        data = json.loads(est.read_text())
        out[bench_id] = data["median"]["point_estimate"]  # nanoseconds
    return out


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        print(
            "Usage: python scripts/compare_benchmarks.py <baseline_name> <current_name> [threshold%] [criterion_dir]",
            file=sys.stderr,
        )
        sys.exit(2)
    baseline_name, current_name = args[0], args[1]
    threshold = float(args[2]) if len(args) > 2 else 15.0
    root = Path(args[3]) if len(args) > 3 else Path("target/criterion")

    baseline = load(root, baseline_name)
    current = load(root, current_name)

    if not current:
        print(f"No criterion results found for baseline '{current_name}' under {root}", file=sys.stderr)
        sys.exit(2)

    rows = []
    regressions = []
    gains = []
    added = 0

    for name in sorted(current):
        cur = current[name]
        base = baseline.pop(name, None)
        if base is None:
            added += 1
            rows.append({"name": name, "baseline": "—", "current": format_time(cur), "change": "new"})
            continue
        delta = 100.0 * (cur - base) / base
        row = {
            "name": name,
            "baseline": format_time(base),
            "current": format_time(cur),
            "change": format_delta(delta),
            "delta": delta,
        }
        rows.append(row)
        if delta > threshold:
            regressions.append(row)
        elif delta < -threshold:
            gains.append(row)

    removed = sorted(baseline)
    for name in removed:
        rows.append({"name": name, "baseline": format_time(baseline[name]), "current": "—", "change": "removed"})

    regressions.sort(key=lambda r: r["delta"], reverse=True)
    gains.sort(key=lambda r: r["delta"])

    lines = ["## Benchmark comparison", ""]
    lines.append("_Times are criterion's median point estimate per benchmark (most stable metric across runs)._")
    lines.append("")

    if regressions:
        lines.append(f"### ❌ {len(regressions)} regression{'' if len(regressions) == 1 else 's'} above {threshold:g}%")
        lines.append("")
        lines += render_table([{**r, "change": f"**{r['change']}**"} for r in regressions])
        lines.append("")
    else:
        lines.append(f"### ✅ No regressions above {threshold:g}%")
        lines.append("")

    if gains:
        lines.append(f"### \U0001f680 {len(gains)} gain{'' if len(gains) == 1 else 's'} above {threshold:g}%")
        lines.append("")
        lines += render_table([{**r, "change": f"**{r['change']}**"} for r in gains])
        lines.append("")

    if added or removed:
        lines.append(f"_{added} benchmark(s) added, {len(removed)} removed (not compared)._")
        lines.append("")

    lines.append("<details>")
    lines.append(f"<summary>All results ({len(rows)} benchmarks)</summary>")
    lines.append("")
    lines += render_table(rows)
    lines.append("")
    lines.append("</details>")

    print("\n".join(lines))
    sys.exit(1 if regressions else 0)


if __name__ == "__main__":
    main()
