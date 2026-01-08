#!/bin/bash

# Configuration
EXTENDED_INFO=false  # Set to true to show extended info (second line)

# Read JSON input from stdin
input=$(cat)

# Get Claude Code credentials from keychain
credentials=$(security find-generic-password -a "$USER" -s "Claude Code-credentials" -w 2>/dev/null)
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

# =============================================================================
# Extract raw data
# =============================================================================

# From Claude Code input
model_name=$(echo "$input" | jq -r '.model.display_name')
session_cost=$(echo "$input" | jq -r '(.cost.total_cost_usd // 0) | . * 100 | round / 100 | tostring | if contains(".") then (. + "00")[0:index(".")+3] else . + ".00" end')
context_usage=$(echo "$input" | jq -r '((.context_window.total_input_tokens // 0) / (.context_window.context_window_size // 1) * 100) | . * 10 | round / 10 | tostring + "%"')
duration_ms=$(echo "$input" | jq -r '.cost.total_duration_ms // 0')
total_input=$(echo "$input" | jq -r '.context_window.total_input_tokens // 0')
total_output=$(echo "$input" | jq -r '.context_window.total_output_tokens // 0')
cache_read=$(echo "$input" | jq -r '.context_window.current_usage.cache_read_input_tokens // 0')
version=$(echo "$input" | jq -r '.version // "N/A"')

# From Anthropic API
current_session=$(echo "$usage_response" | jq -r 'if .five_hour.utilization != null then ((.five_hour.utilization | tostring) + "%") else "N/A" end')
weekly=$(echo "$usage_response" | jq -r 'if .seven_day.utilization != null then ((.seven_day.utilization | tostring) + "%") else "N/A" end')

# Calculate session reset time
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

# Calculate weekly reset time
weekly_resets_at=$(echo "$usage_response" | jq -r '.seven_day.resets_at // empty')
if [[ -n "$weekly_resets_at" ]]; then
    weekly_resets_epoch=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%S" "${weekly_resets_at%%.*}" +%s 2>/dev/null)
    weekly_resets_formatted=$(date -j -f "%s" "$weekly_resets_epoch" "+Resets %a %-l:%M %p")
else
    weekly_resets_formatted="N/A"
fi

# Calculate duration
duration_min=$((duration_ms / 60000))
duration_sec=$(((duration_ms % 60000) / 1000))

# =============================================================================
# Define widgets
# =============================================================================

W_MODEL="🤖 $model_name"
W_COST="💰 \$$session_cost"
W_SESSION="📈 Session: $current_session ($resets_in)"
W_WEEKLY="📅 Weekly: $weekly ($weekly_resets_formatted)"
W_CONTEXT="🧠 Context: $context_usage"
W_DURATION="⏱️ ${duration_min}m ${duration_sec}s"
W_TOKENS="🔤 In: $total_input Out: $total_output"
W_CACHE="💾 Cache: $cache_read"
W_VERSION="🚀 Claude Code v$version"

# =============================================================================
# Compose and output lines
# =============================================================================

LINE1="$W_MODEL | $W_COST | $W_SESSION | $W_WEEKLY | $W_CONTEXT"
echo "$LINE1"

if [[ "$EXTENDED_INFO" == "true" ]]; then
    LINE2="$W_DURATION | $W_TOKENS | $W_CACHE | $W_VERSION"
    echo "$LINE2"
fi
