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

`ccline` reads an optional TOML file from:

1. `$XDG_CONFIG_HOME/claude-code-statusline/config.toml`, if `XDG_CONFIG_HOME` is set.
2. `$HOME/.config/claude-code-statusline/config.toml` otherwise.

If the file is missing the baked-in default (equivalent to the `basic` template) is used. If the file is present but malformed, `ccline` logs the parse error to stderr **and still renders the default line** — the statusline must never break Claude Code.

### Minimal example

```toml
format = "🤖 $model | 💰 $cost | 🧠 $context_bar$git_branch_sep"
```

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

| Placeholder       | Description                                       | Example                     |
| ----------------- | ------------------------------------------------- | --------------------------- |
| `$model`          | Current model display name                        | `Opus 4.6`                  |
| `$cost`           | Session cost in USD                               | `$1.79`                     |
| `$duration`       | Session duration                                  | `7m 5s`                     |
| `$session`        | 5-hour utilization (`N/A` if missing)             | `17%`                       |
| `$session_reset`  | Countdown to the 5-hour reset                     | `0h 31m`                    |
| `$weekly`         | 7-day utilization (`N/A` if missing)              | `12%`                       |
| `$weekly_reset`   | Weekly reset weekday + time                       | `Wed 9:00PM`                |
| `$context`        | Context window usage percentage                   | `17%`                       |
| `$context_bar`    | 10-cell █/░ bar + percent + used_k/total_k tokens | `█░░░░░░░░░ 17% (34k/200k)` |
| `$tokens_in`      | Total input tokens (comma-separated)              | `45,000`                    |
| `$tokens_out`     | Total output tokens (comma-separated)             | `3,200`                     |
| `$cache`          | Cache hit rate + cache-read count                 | `84% (38,000)`              |
| `$version`        | Claude Code version                               | `v2.0.76`                   |
| `$project`        | Project directory basename                        | `claude-code-statusline`    |
| `$git_branch`     | Raw branch name (empty outside a repo)            | `master`                    |
| `$git_branch_sep` | ` \| 🌿 <branch>` (empty outside a repo)          | ` \| 🌿 master`             |

`$git_branch_sep` exists because it absorbs its own leading separator — so when you're not in a git repo nothing renders and you don't get an awkward trailing ` | 🌿` in the output.

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

The v1 shell script remains in git history if you need to roll back.

## License

MIT
