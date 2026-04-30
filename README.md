# Claude Code Statusline (`ccline`)

A small Rust binary (`ccline`) that renders a customizable statusline for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Reads the session JSON from stdin, formats it using a Starship-style `format` string, and writes the result to stdout.

This is the v2 rewrite of the original shell script. The previous Bash implementation lives in the git history if you need it.

## Output Examples

**Basic template** (single line — the baked-in default):

```
🤖 Opus 4.6 | 💰 $1.79 | 📈 17% [0h 31m] | 📅 12% [Wed 9:00PM] | 🧠 █░░░░░░░░░ 17% (34k/200k) | 🌿 main | 📁 my-project
```

**Extended template** (two lines):

```
🤖 Opus 4.6 | 💰 $1.79 | ⏱️ 7m 5s | 📈 17% [0h 31m] | 📅 12% [Wed 9:00PM] | 🧠 Context: 17%
🚀 Claude Code v2.0.76 | ⬇️ Tokens In: 45,000 | ⬆️ Tokens Out: 3,200 | ♻️ Cache: 84% (38,000) | 🌿 main | 📁 my-project
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
make
```

`make` runs `cargo build --release` and stages the binary at `dist/ccline` (~1.2 MB). If you prefer Cargo directly, `cargo build --release` produces the same binary at `target/release/ccline`.

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

| Placeholder      | Description                                       | Example                     |
| ---------------- | ------------------------------------------------- | --------------------------- |
| `$model`         | Current model display name                        | `Opus 4.6`                  |
| `$cost`          | Session cost in USD                               | `$1.79`                     |
| `$duration`      | Session duration                                  | `7m 5s`                     |
| `$session`       | 5-hour utilization (`N/A` if missing)             | `17%`                       |
| `$session_reset` | Countdown to the 5-hour reset                     | `0h 31m`                    |
| `$weekly`        | 7-day utilization (`N/A` if missing)              | `12%`                       |
| `$weekly_reset`  | Weekly reset weekday + time                       | `Wed 9:00PM`                |
| `$context`       | Context window usage percentage                   | `17%`                       |
| `$context_bar`   | 10-cell █/░ bar + percent + used_k/total_k tokens | `█░░░░░░░░░ 17% (34k/200k)` |
| `$tokens_in`     | Total input tokens (comma-separated)              | `45,000`                    |
| `$tokens_out`    | Total output tokens (comma-separated)             | `3,200`                     |
| `$cache`         | Cache hit rate + cache-read count                 | `84% (38,000)`              |
| `$version`       | Claude Code version                               | `v2.0.76`                   |
| `$project`       | Project directory basename                        | `claude-code-statusline`    |
| `$git_branch`    | Raw branch name (empty outside a repo)            | `master`                    |

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

For backward compatibility, the legacy `$git_branch_sep` placeholder is still recognized and expands to `( | 🌿 $git_branch)` at parse time. New configs should use the explicit group form.

## How It Works

1. Reads the Claude Code session JSON from stdin into a typed `Input` struct (all fields optional for forward compat).
2. Loads the TOML config file, falling back to the baked-in default.
3. Parses the `format` string into a sequence of literal and `$module` tokens.
4. Dispatches each `$module` to its renderer and writes the concatenated result to stdout.

Parse errors for either the JSON or the config file are logged to stderr. stdout always receives a valid line.

## Development

The Makefile wraps the common Cargo workflows:

| Target       | What it does                                                                    |
| ------------ | ------------------------------------------------------------------------------- |
| `make`       | Alias for `make dist`                                                           |
| `make dist`  | `cargo build --release`, copy the binary into `dist/ccline`, and print its size |
| `make dev`   | `cargo run` — running without piped stdin prints a usage banner (see below)     |
| `make test`  | `cargo test` — runs the full unit suite                                         |
| `make clean` | `cargo clean` plus `rm -rf dist`                                                |

### Running directly

`ccline` expects the Claude Code session JSON on stdin. If you run it in an interactive terminal (e.g. `make dev` or `./dist/ccline`), it detects the TTY, prints a short usage banner, and exits cleanly instead of blocking on stdin forever.

To smoke-test the render pipeline locally, pipe the bundled fixture:

```bash
cat tests/fixtures/sample_input.json | ./dist/ccline
```

## Migrating from the Bash version

| v1 (shell)                           | v2 (`ccline`)                                   |
| ------------------------------------ | ----------------------------------------------- |
| `statusline.sh` + `jq` at runtime    | Single self-contained binary, zero runtime deps |
| macOS only (BSD `date` flags)        | macOS, Linux, and Windows                       |
| `STATUSLINE_TEMPLATE` env var        | `format` field in TOML config                   |
| Bash template arrays with `---`      | TOML multi-line string with literal `\n`        |
| `{placeholder}` syntax               | `$module` syntax (Starship-style)               |
| `{git_branch_sep}` (baked separator) | `$git_branch_sep` (same behavior, same name)    |

The v1 shell script has been removed; check out an earlier tag if you need to roll back.

## License

MIT
