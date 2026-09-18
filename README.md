# Claude Code Statusline (`ccline`)

A small Rust binary (`ccline`) that renders a customizable statusline for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Reads the session JSON from stdin, formats it using a Starship-style `format` string, and writes the result to stdout.

## Output Examples

**Basic template** (single line — the baked-in default):

```
🤖 Opus 5 (1M context) | 💰 $6.62 | 📈 17% [4h 21m] | 📅 22% [Tue 7:00PM] | 🧠 █░░░░░░░░░ 13% (130k/1000k) | 🌿 main | 📁 my-project
```

**Extended template** (two lines):

```
🤖 Opus 5 (1M context) | 💰 $6.62 | ⏱️ 26m 51s | 📈 17% [4h 21m] | 📅 22% [Tue 7:00PM] | 🧠 Context: 13%
🚀 Claude Code v2.1.276 | ⬇️ Tokens In: 128,000 | ⬆️ Tokens Out: 9,400 | ♻️ Cache: 85% (118,000) | 🌿 main | 📁 my-project
```

## Requirements

- macOS, Linux, or Windows
- Rust stable (for building)
- Claude Code v2.x or later

Build dependencies: `serde`, `serde_json`, `toml`, `anyhow`, `jiff` (cross-platform timezone-aware date formatting), and `dirs` (cross-platform home directory resolution). No runtime dependencies — the binary reads `.git/HEAD` directly and formats dates in-process, so it does not shell out to `git` or `date`.

## Installation

Clone and build:

```bash
git clone https://github.com/skydiver/claude-code-statusline.git
cd claude-code-statusline
make dist
```

`make dist` runs `cargo build --release` and stages the binary at `dist/ccline` (~1.2 MB). Running `make` on its own lists the available targets instead of building. If you prefer Cargo directly, `cargo build --release` produces the same binary at `target/release/ccline`.

### Wire into Claude Code

Add the binary path to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/dist/ccline",
    "padding": 0
  }
}
```

Restart Claude Code. No environment variable is needed to pick a template — use the config file instead.

## Configuration

`ccline` reads an optional TOML file from (in order of precedence):

1. `$CCLINE_CONFIG`, if set to a non-empty path. Lets you point `ccline` at any file — handy for switching between layouts without moving files around.
2. `$XDG_CONFIG_HOME/claude-code-statusline/config.toml`, if `XDG_CONFIG_HOME` is set.
3. `$HOME/.config/claude-code-statusline/config.toml` otherwise.

If the file is missing the baked-in default (equivalent to the `basic` template) is used. If the file is present but malformed, `ccline` logs the parse error to stderr **and still renders the default line** — the statusline must never break Claude Code. When `CCLINE_CONFIG` points at a missing or unreadable file, the read error is also logged to stderr so a typo'd path doesn't fail silently.

`CCLINE_CONFIG` is handy for keeping multiple layouts side-by-side (e.g. one for work, one for personal) and pointing `ccline` at whichever one you want via your shell environment or launcher config.

### Mirroring the input to disk

Set `CCLINE_INPUT_DUMP` to a writable path and `ccline` will save a pretty-printed JSON snapshot of the stdin payload to that path on every render. Useful when another tool (a menubar app, a dashboard, a watcher script) wants the current Claude Code session state without re-implementing a statusline reader.

```bash
export CCLINE_INPUT_DUMP="$HOME/.cache/ccline/input.json"
```

Writes are atomic — the JSON goes to `<path>.tmp` first and is then renamed onto the target. A reader either sees the previous snapshot or the new one, never a half-written file. The dump preserves every field Claude Code sent (including ones `ccline` itself doesn't render), so it stays forward-compatible with new payload fields. An empty value is treated as unset; a write failure is logged to stderr but never breaks the statusline.

### Minimal example

```toml
format = "🤖 $model | 💰 $cost | 🧠 $context_bar ( | 🌿 $git_branch )"
```

The `(...)` wrapping around ` | 🌿 $git_branch` is a **conditional group**: if any `$module` inside renders empty, the entire group disappears — separator, emoji, and all. Outside a git repo you get just `🤖 $model | 💰 $cost | 🧠 $context_bar` with no awkward trailing ` | 🌿`. See [Conditional Groups](#conditional-groups) below.

### Multi-line example

```toml
format = """
🤖 $model | 💰 $cost | ⏱️ $duration
🚀 Claude Code $version | 🌿 $git_branch\
"""
```

The trailing `\` before the closing `"""` strips the final newline so you don't get an empty line under the statusline. Two ready-to-copy presets live in [`examples/basic.toml`](examples/basic.toml) and [`examples/extended.toml`](examples/extended.toml).

## Available Placeholders

Each placeholder is a Starship-style `$module_name` reference. Everything else in the `format` string is literal text (including emojis, separators, and newlines).

| Placeholder      | Description                                             | Example                       |
| ---------------- | ------------------------------------------------------- | ----------------------------- |
| `$model`         | Current model display name                              | `Opus 5 (1M context)`         |
| `$cost`          | Session cost in USD                                     | `$6.62`                       |
| `$duration`      | Session duration                                        | `26m 51s`                     |
| `$session`       | 5-hour utilization, colored by usage (`N/A` if missing) | `17%`                         |
| `$session_reset` | Countdown to the 5-hour reset                           | `4h 21m`                      |
| `$weekly`        | 7-day utilization, colored by pace (`N/A` if missing)   | `22%`                         |
| `$weekly_reset`  | Weekly reset weekday + time                             | `Tue 7:00PM`                  |
| `$context`       | Context window usage percentage                         | `13%`                         |
| `$context_bar`   | 10-cell █/░ bar + percent + used_k/total_k tokens       | `█░░░░░░░░░ 13% (130k/1000k)` |
| `$tokens_in`     | Total input tokens (comma-separated)                    | `128,000`                     |
| `$tokens_out`    | Total output tokens (comma-separated)                   | `9,400`                       |
| `$cache`         | Cache hit rate + cache-read count                       | `85% (118,000)`               |
| `$version`       | Claude Code version                                     | `v2.1.276`                    |
| `$project`       | Project directory basename                              | `my-project`                  |
| `$git_branch`    | Raw branch name (empty outside a repo)                  | `main`                        |

### Colors

Four placeholders wrap themselves in ANSI color when they cross a threshold. Everything else renders plain, so the `format` string stays in charge of the rest of the styling.

`$context` and `$context_bar` color on **absolute context pressure**:

| Context used | Color   |
| ------------ | ------- |
| 0–64%        | default |
| 65–74%       | yellow  |
| 75%+         | red     |

`$session` also colors on absolute usage, on its own thresholds:

| 5-hour used | Color   |
| ----------- | ------- |
| 0–64%       | default |
| 65–84%      | yellow  |
| 85%+        | red     |

It is deliberately *not* pace-based. A five-hour window is meant to be spent, so burning it near-linearly is ordinary work, not a warning — pace coloring would sit yellow through most of any focused session and train you to ignore it. Only proximity to the cutoff is worth flagging, and `$session_reset` already tells you when that cutoff lands.

`$weekly` colors on **burn pace** instead, because a raw weekly percentage says nothing on its own — 40% is comfortable on day 5 and alarming on day 1. Spending the full allowance evenly across the seven days works out to 14.29%/day, so the budget at any moment is simply the share of the window already elapsed:

```
budget = elapsed_fraction_of_the_7_day_window × 100
pace   = weekly_used_percentage ÷ budget
```

| Pace           | Color   | Meaning                                      |
| -------------- | ------- | -------------------------------------------- |
| below 1.0x     | default | on track to reach the reset with room to spare |
| 1.0x – 1.49x   | yellow  | on track to run out before the reset          |
| 1.5x and above | red     | on track to run out well before the reset     |

Two guards keep that ratio honest:

- **Below 5% used, never colored.** Minutes into a fresh window the elapsed fraction is near zero, so any usage at all computes to a wild pace. That's arithmetic, not a warning.
- **At 90% used, always red.** Late in the window a near-exhausted allowance can still be technically "under budget" — 92% on day 6.5 is a 0.99x pace and one session from the wall. This band needs no reset timestamp, so it warns even on an older payload that omits one.

When `resets_at` is missing or the window start lands in the future (clock skew), the pace can't be computed and `$weekly` renders plain.

### Conditional Groups

Anything you wrap in `(...)` is a **conditional group**: the group renders only if every `$module` inside it produces a non-empty value. If any one module is missing, the whole group — including its literal separators, emoji, and spaces — disappears.

```toml
format = "🧠 $context_bar ( | 🌿 $git_branch ) | 📁 $project"
```

- Inside a git repo → `🧠 █░░░░░░░░░ 17% | 🌿 master | 📁 my-project`
- Outside a git repo → `🧠 █░░░░░░░░░ 17% | 📁 my-project` (no dangling ` | 🌿`)

Whitespace directly adjacent to `(` or `)` is treated as cosmetic padding — a space before `(` is absorbed into the group (so it disappears along with the group when suppressed) and any whitespace immediately inside `(...)` is stripped. That means all three of these parse to the same thing and render identically:

```toml
format = "$context_bar ( | 🌿 $git_branch )"  # symmetric, recommended
format = "$context_bar (| 🌿 $git_branch)"    # tight
format = "$context_bar( | 🌿 $git_branch)"    # legacy, no outer space
```

Groups can nest. An inner group's emptiness does **not** bubble up to its parent — each group decides independently. Unmatched `(` extends silently to end-of-string; a stray `)` is treated as literal text.

## How It Works

1. Reads the Claude Code session JSON from stdin into a typed `Input` struct (all fields optional for forward compat).
2. Loads the TOML config file, falling back to the baked-in default.
3. Parses the `format` string into a sequence of literal and `$module` tokens.
4. Dispatches each `$module` to its renderer and writes the concatenated result to stdout.

Parse errors for either the JSON or the config file are logged to stderr. stdout always receives a valid line.

## Development

The Makefile wraps the common Cargo workflows. Running `make` with no target lists them all:

```
ccline — available targets:

  all      Alias for `make dist`
  clean    Remove build artifacts (cargo clean plus dist/)
  dev      Run via cargo (prints a usage banner without piped stdin)
  dist     Build the release binary and copy it into dist/
  help     List the available targets
  preview  Render the statusline across a scenario matrix, in real color
  test     Run the full unit suite
```

That listing is generated from the `##` comments on the targets themselves, so it can't drift out of sync.

| Target       | What it does                                                                    |
| ------------ | ------------------------------------------------------------------------------- |
| `make`       | Alias for `make help` — lists the targets without building                      |
| `make all`   | Alias for `make dist`                                                           |
| `make dist`  | `cargo build --release`, copy the binary into `dist/ccline`, and print its size |
| `make dev`   | `cargo run` — running without piped stdin prints a usage banner (see below)     |
| `make test`  | `cargo test` — runs the full unit suite                                         |
| `make preview` | Build, then render a scenario matrix in real color (see below)                |
| `make clean` | `cargo clean` plus `rm -rf dist`                                               |

### Previewing scenarios

The unit suite asserts escape sequences as strings, which proves the color bands are correct but never shows you what they look like. `make preview` does the opposite — it feeds synthetic payloads to the built binary and prints the real output, grouped by scenario:

```
WEEKLY PACE  (budget = elapsed share of the 7-day window)
  day 1.0  10%  0.70x   📅 10% [Thu 9:25AM]
  day 1.0  15%  1.05x   📅 15% [Thu 9:25AM]      ← yellow
  day 1.0  22%  1.54x   📅 22% [Thu 9:25AM]      ← red
  day 6.5  88%  0.95x   📅 88% [Fri 9:25PM]
```

It covers the weekly pace bands, both guards, every degradation path, the context pressure thresholds, and empty/partial payloads. The `← yellow` / `← red` annotations are read back out of the rendered bytes rather than hardcoded, so moving a threshold moves the annotation with it. Each section pins its own `format`, so your personal config never skews the output.

Scenario values sit just inside their bands rather than exactly on `1.00x` / `1.50x`: the binary reads its own clock a few milliseconds after the script does, which is enough to tip an exact boundary. The boundaries themselves are pinned by unit tests, which inject `now`.

### Running directly

`ccline` expects the Claude Code session JSON on stdin. If you run it in an interactive terminal (e.g. `make dev` or `./dist/ccline`), it detects the TTY, prints a short usage banner, and exits cleanly instead of blocking on stdin forever.

To smoke-test the render pipeline locally, pipe the bundled fixture:

```bash
cat tests/fixtures/sample_input.json | ./dist/ccline
```

The fixture carries fixed timestamps, so `$session_reset` counts down against your current clock and reads `0h 0m` once that moment has passed. Everything else renders exactly as shown in [Output Examples](#output-examples), give or take your own git branch.

## License

MIT
