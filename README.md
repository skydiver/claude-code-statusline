# Claude Code Statusline

Custom statusline script for Claude Code that displays real-time usage metrics.

## Output

```
🤖 Opus 4.5 | 💰 $1.23 | 📈 Session: 5.2% (Resets in 3 hr 16 min) | 📅 Weekly: 12.5% (Resets Mon 3:59 PM) | 🧠 Context: 2.5%
```

## Metrics

| Icon | Metric                                                           |
| ---- | ---------------------------------------------------------------- |
| 🤖   | Current model name                                               |
| 💰   | Total cost spent in the current session (USD)                    |
| 📈   | 5-hour rolling session utilization with time until reset         |
| 📅   | 7-day rolling weekly utilization with day/time of next reset     |
| 🧠   | Context window usage (total input tokens / context window size)  |

## Requirements

- macOS (uses `security` command for Keychain access)
- `jq` for JSON parsing
- Claude Code with OAuth authentication

> [!WARNING]
> macOS only — uses `security` command for Keychain access. Credential retrieval would need modification for other platforms.

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

## How It Works

The script:
1. Reads JSON input from Claude Code via stdin
2. Retrieves OAuth credentials from macOS Keychain
3. Calls the Anthropic OAuth Usage API for rate limit data
4. Formats and displays all metrics in a single line

## License

MIT
