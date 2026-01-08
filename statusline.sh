#!/bin/bash

# Read JSON input from stdin
input=$(cat)

# Get Claude Code credentials from keychain
credentials=$(security find-generic-password -a "$USER" -s "Claude Code-credentials" -w 2>/dev/null)

# Extract access token from credentials
access_token=$(echo "$credentials" | jq -r '.claudeAiOauth.accessToken')

# Function to get usage from Anthropic API
get_usage() {
    curl --silent --request GET \
        --url https://api.anthropic.com/api/oauth/usage \
        --header 'anthropic-beta: oauth-2025-04-20' \
        --header "authorization: Bearer $access_token" \
        --header 'content-type: application/json' \
        --header 'user-agent: claude-code/2.0.71'
}

# Get usage data
usage_response=$(get_usage)

# Extract current session utilization (five_hour.utilization)
current_session=$(echo "$usage_response" | jq -r 'if .five_hour.utilization != null then ((.five_hour.utilization | tostring) + "%") else "N/A" end')

# Extract weekly utilization (seven_day.utilization)
weekly=$(echo "$usage_response" | jq -r 'if .seven_day.utilization != null then ((.seven_day.utilization | tostring) + "%") else "N/A" end')

# Calculate weekly reset time
weekly_resets_at=$(echo "$usage_response" | jq -r '.seven_day.resets_at // empty')
if [[ -n "$weekly_resets_at" ]]; then
    weekly_resets_epoch=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%S" "${weekly_resets_at%%.*}" +%s 2>/dev/null)
    weekly_resets_formatted=$(date -j -f "%s" "$weekly_resets_epoch" "+Resets %a %-l:%M %p")
else
    weekly_resets_formatted="N/A"
fi

# Calculate context window usage percentage
context_usage=$(echo "$input" | jq -r '
  ((.context_window.total_input_tokens // 0) / (.context_window.context_window_size // 1) * 100)
  | . * 10 | round / 10 | tostring + "%"
')

# Calculate time until reset (API returns UTC)
resets_at=$(echo "$usage_response" | jq -r '.five_hour.resets_at // empty')
if [[ -n "$resets_at" ]]; then
    resets_epoch=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%S" "${resets_at%%.*}" +%s 2>/dev/null)
    now_epoch=$(date +%s)
    diff_seconds=$((resets_epoch - now_epoch))
    hours=$((diff_seconds / 3600))
    minutes=$(((diff_seconds % 3600) / 60))
    resets_in="Resets in ${hours} hr ${minutes} min"
else
    resets_in="N/A"
fi

# Row 1: Robot + Model + Cost + Session Usage
model_name=$(echo "$input" | jq -r '.model.display_name')
session_cost=$(echo "$input" | jq -r '(.cost.total_cost_usd // 0) | . * 100 | round / 100 | tostring | if contains(".") then (. + "00")[0:index(".")+3] else . + ".00" end')
echo "🤖 $model_name | 💰 \$$session_cost | 📈 Session: $current_session ($resets_in) | 📅 Weekly: $weekly ($weekly_resets_formatted) | 🧠 Context: $context_usage"
