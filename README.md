# Claude Code Statusline

Custom statusline script for Claude Code that displays real-time usage metrics with fully customizable templates.

> [!WARNING]
> macOS only — uses `security` command for Keychain access. Credential retrieval would need modification for other platforms.

## Output Examples

**Basic template** (single line):

```
🤖 Opus 4.5 | 💰 $1.79 | 📈 Session: 17.0% (Resets in 0h 31m) | 📅 Weekly: 4.0% (Resets Thu 10:59AM) | 🧠 Context: 7.5%
```

**Extended template** (two lines):

```
🤖 Opus 4.5 | 💰 $1.79 | ⏱️ 21m 39s | 📈 Session: 17.0% (Resets in 0h 31m) | 📅 Weekly: 4.0% (Resets Thu 10:59AM) | 🧠 Context: 7.5%
🚀 Claude Code v2.1.1 | ⬇️ Tokens In: 43,439 | ⬆️ Tokens Out: 43,829 | ♻️ Cache: 99% (56,410)
```

## Requirements

- macOS (uses `security` command for Keychain access)
- `jq` for JSON parsing
- Claude Code with OAuth authentication

## Installation

1. Download `statusline.sh` and make it executable:

```bash
chmod +x /path/to/statusline.sh
```

2. Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/path/to/statusline.sh",
    "padding": 0
  }
}
```

3. Restart Claude Code

## Configuration

### Selecting a Template

Set the `STATUSLINE_TEMPLATE` environment variable in your `~/.claude/settings.json`:

```json
{
  "env": {
    "STATUSLINE_TEMPLATE": "extended"
  },
  "statusLine": {
    "type": "command",
    "command": "/path/to/statusline.sh",
    "padding": 0
  }
}
```

Available templates: `basic` (default), `extended`

### Creating Custom Templates

Templates are defined as arrays in the script. Each string is concatenated, and you control separators by including them in the strings:

```bash
TEMPLATE_CUSTOM=(
    "🤖 {model} | "
    "💰 {cost} | "
    "📈 Session: {session}"
    ---
    "🚀 {version}"
)
```

- **Concatenation**: Strings are joined directly (include separators like `|` in your strings)
- **Line breaks**: Use `---` to start a new line
- **Full control**: Add any emojis, labels, or formatting around placeholders

## Available Placeholders

| Placeholder       | Description              | Example        |
| ----------------- | ------------------------ | -------------- |
| `{model}`         | Current model name       | `Opus 4.5`     |
| `{cost}`          | Session cost in USD      | `$1.79`        |
| `{duration}`      | Session duration         | `21m 39s`      |
| `{session}`       | 5-hour utilization       | `17.0%`        |
| `{session_reset}` | Time until session reset | `0h 31m`       |
| `{weekly}`        | 7-day utilization        | `4.0%`         |
| `{weekly_reset}`  | Weekly reset day/time    | `Thu 10:59AM`  |
| `{context}`       | Context window usage     | `7.5%`         |
| `{tokens_in}`     | Total input tokens       | `43,439`       |
| `{tokens_out}`    | Total output tokens      | `43,829`       |
| `{cache}`         | Cache hit rate and count | `99% (56,410)` |
| `{version}`       | Claude Code version      | `v2.1.1`       |

## Template Examples

### Minimal

```bash
TEMPLATE_MINIMAL=(
    "🤖 {model} | "
    "💰 {cost} | "
    "📈 {session}"
)
```

Output: `🤖 Opus 4.5 | 💰 $1.79 | 📈 17.0%`

### With Custom Labels

```bash
TEMPLATE_CUSTOM=(
    "Model: {model} | "
    "Cost: {cost} | "
    "Usage: {session}"
)
```

Output: `Model: Opus 4.5 | Cost: $1.79 | Usage: 17.0%`

### Multi-line with Tokens

```bash
TEMPLATE_DETAILED=(
    "{model} | {cost} | {session}"
    ---
    "Tokens: ▼{tokens_in} ▲{tokens_out} | Cache: {cache}"
)
```

Output:

```
Opus 4.5 | $1.79 | 17.0%
Tokens: ▼43,439 ▲43,829 | Cache: 99% (56,410)
```

## How It Works

1. Reads JSON input from Claude Code via stdin
2. Retrieves OAuth credentials from macOS Keychain
3. Calls the Anthropic OAuth Usage API for rate limit data
4. Renders the selected template with placeholder substitution
5. Outputs formatted statusline (single or multi-line)

## License

MIT
