# Claude Code Statusline (`ccline`)

A small Rust binary (`ccline`) that renders a customizable statusline for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Reads the session JSON from stdin, formats it using a Starship-style `format` string, and writes the result to stdout.

This is the v2 rewrite of the original shell script. The previous Bash implementation lives in the git history if you need it.

## Output Examples

**Basic template** (single line — the baked-in default):

```
🤖 Opus 4.6 | 💰 $1.79 | 📈 Session: 17% [0h 31m] | 📅 Weekly: 12% [Wed 9:00PM] | 🧠 █░░░░░░░░░ 17% (34k/200k) | 🌿 main
```

**Extended template** (two lines):

```
🤖 Opus 4.6 | 💰 $1.79 | ⏱️ 7m 5s | 📈 Session: 17% [0h 31m] | 📅 Weekly: 12% [Wed 9:00PM] | 🧠 Context: 17%
🚀 Claude Code v2.0.76 | ⬇️ Tokens In: 45,000 | ⬆️ Tokens Out: 3,200 | ♻️ Cache: 84% (38,000) | 🌿 main
```

## Requirements

- macOS (only supported platform for v2)
- Rust stable (for building)
- Claude Code v2.x or later

The binary depends only on `serde`, `serde_json`, `toml`, and `anyhow` at build time. At runtime it shells out to the system `git` and `date` binaries — both shipped with macOS.

## Installation

Clone and build:

```bash
git clone https://github.com/skydiver/claude-code-statusline.git
cd claude-code-statusline
cargo build --release
```

The binary lands at `target/release/ccline` (~855K).

### Wire into Claude Code

Add the binary path to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/ccline",
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

## Migrating from the Bash version

| v1 (shell)                           | v2 (`ccline`)                                                 |
| ------------------------------------ | ------------------------------------------------------------- |
| `statusline.sh` + `jq` at runtime    | Single binary, no runtime deps beyond system `git` and `date` |
| `STATUSLINE_TEMPLATE` env var        | `format` field in TOML config                                 |
| Bash template arrays with `---`      | TOML multi-line string with literal `\n`                      |
| `{placeholder}` syntax               | `$module` syntax (Starship-style)                             |
| `{git_branch_sep}` (baked separator) | `$git_branch_sep` (same behavior, same name)                  |

The v1 shell script remains in git history if you need to roll back.

## License

MIT
