#!/usr/bin/env python3
"""Render the statusline across a scenario matrix, in real color.

The unit tests assert escape sequences as strings, which proves the bands are
correct but never shows you what they look like. This does the opposite: it
feeds synthetic payloads to the built binary and prints the actual output, so
thresholds can be judged by eye before they ship.

Each section pins its own `format` via CCLINE_CONFIG so the preview stays
focused and your personal config never skews it. The `band` annotation is read
back out of the rendered bytes rather than hardcoded — if a threshold moves,
the annotation moves with it instead of quietly lying.

Scenarios sit just inside their bands rather than exactly on 1.00x / 1.50x:
the binary reads its own clock a few milliseconds after this script reads NOW,
which is enough to tip an exact boundary to the other side. The boundaries
themselves are pinned by the unit tests, which inject `now`.

Usage: make preview   (or: python3 scripts/preview.py)
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "ccline"

DAY = 86_400
WEEK = 7 * DAY
NOW = int(time.time())

ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
BOLD, DIM, RESET = "\x1b[1m", "\x1b[2m", "\x1b[0m"

WEEKLY_FORMAT = "📅 $weekly [$weekly_reset]"
CONTEXT_FORMAT = "🧠 $context_bar"
SESSION_FORMAT = "📈 $session [$session_reset]"
FULL_FORMAT = "🤖 $model | 📈 $session [$session_reset] | 📅 $weekly [$weekly_reset] | 🧠 $context"


def resets_at(days_elapsed):
    """`resets_at` for a window that opened `days_elapsed` days ago."""
    return int(NOW + (7 - days_elapsed) * DAY)


def weekly(used=None, reset=None):
    window = {}
    if used is not None:
        window["used_percentage"] = used
    if reset is not None:
        window["resets_at"] = reset
    return {"rate_limits": {"seven_day": window}}


def session(used, minutes_left=42):
    return {
        "rate_limits": {
            "five_hour": {
                "used_percentage": used,
                "resets_at": NOW + minutes_left * 60,
            }
        }
    }


def context(used, size=200_000):
    return {"context_window": {"used_percentage": used, "context_window_size": size}}


def render(payload, fmt):
    """Pipe `payload` through the binary under an explicit format."""
    with tempfile.NamedTemporaryFile("w", suffix=".toml", delete=False) as f:
        f.write(f"format = {json.dumps(fmt, ensure_ascii=False)}\n")
        config = f.name
    try:
        env = {**os.environ, "CCLINE_CONFIG": config}
        result = subprocess.run(
            [str(BIN)],
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            env=env,
        )
        return result.stdout.rstrip("\n")
    finally:
        os.unlink(config)


def band(rendered):
    """Read the band back out of the output — never assume it."""
    if "\x1b[31m" in rendered:
        return "red"
    if "\x1b[33m" in rendered:
        return "yellow"
    return ""


def visible_len(s):
    """Display width, not character count — the emoji separators are double-width."""
    return sum(
        2 if unicodedata.east_asian_width(c) in ("W", "F") else 1
        for c in ANSI_RE.sub("", s)
    )


def row(label, rendered, width=34):
    marker = band(rendered)
    pad = " " * max(0, width - visible_len(rendered))
    suffix = f"{pad}  {DIM}← {marker}{RESET}" if marker else ""
    print(f"  {label:<22}{rendered}{suffix}")


def section(title):
    print(f"\n{BOLD}{title}{RESET}")


def main():
    if not BIN.exists():
        sys.exit(f"binary not found at {BIN} — run `cargo build --release` first")

    section("WEEKLY PACE  (budget = elapsed share of the 7-day window)")
    for days, used_values in [(1.0, [10, 15, 22]), (3.5, [49, 51, 76]), (6.5, [60, 88])]:
        for used in used_values:
            pace = used / (days / 7 * 100)
            label = f"day {days}  {used}%  {pace:.2f}x"
            row(label, render(weekly(used, resets_at(days)), WEEKLY_FORMAT))

    section("WEEKLY GUARDS")
    row("1h in, 4% (floor)", render(weekly(4, resets_at(1 / 24)), WEEKLY_FORMAT))
    row("1h in, 5% (floor)", render(weekly(5, resets_at(1 / 24)), WEEKLY_FORMAT))
    row("day 6.5, 92% (crit)", render(weekly(92, resets_at(6.5)), WEEKLY_FORMAT))
    row("day 1, 100% (crit)", render(weekly(100, resets_at(1)), WEEKLY_FORMAT))

    section("WEEKLY DEGRADATION")
    row("no resets_at, 50%", render(weekly(50), WEEKLY_FORMAT))
    row("no resets_at, 95%", render(weekly(95), WEEKLY_FORMAT))
    row("clock skew", render(weekly(50, NOW + WEEK + 3600), WEEKLY_FORMAT))
    row("stale (reset passed)", render(weekly(60, NOW - 3600), WEEKLY_FORMAT))
    row("no used_percentage", render(weekly(reset=resets_at(1)), WEEKLY_FORMAT))

    section("SESSION PRESSURE  (absolute thresholds, not pace)")
    for used in [17, 64, 65, 84, 85, 98]:
        row(f"{used}%", render(session(used), SESSION_FORMAT))

    section("CONTEXT PRESSURE  (absolute thresholds)")
    for used in [17, 64, 65, 74, 75, 100]:
        row(f"{used}%", render(context(used), CONTEXT_FORMAT))

    section("EMPTY / PARTIAL PAYLOADS")
    row("empty object", render({}, FULL_FORMAT))
    row("sample fixture", render(json.loads((ROOT / "tests/fixtures/sample_input.json").read_text()), FULL_FORMAT))
    print()


if __name__ == "__main__":
    main()
