#!/bin/bash

# =============================================================================
# Configuration
# =============================================================================

TEMPLATE="${STATUSLINE_TEMPLATE:-basic}"  # Options: basic, extended (set via env var or change default)

# =============================================================================
# Template syntax
# =============================================================================
#
#   - Each string in the array is concatenated (include separators in the string)
#   - Use "---" to start a new line
#   - Placeholders: {model}, {cost}, {duration}, {session}, {session_reset},
#                   {weekly}, {weekly_reset}, {context}, {tokens_in}, {tokens_out},
#                   {cache}, {version}
#   - Add any literal text, emojis, or formatting around placeholders
#
# Examples:
#   "🤖 {model}"                            -> 🤖 Opus 4.5
#   "💰 {cost}"                             -> 💰 $1.79
#   "📈 Session: {session}"                 -> 📈 Session: 17.0%
#   "{session} (Resets in {session_reset})" -> 17.0% (Resets in 0h 31m)
#
# Multi-line example:
#   TEMPLATE_CUSTOM=(
#       "🤖 {model} | "
#       "💰 {cost}"
#       ---
#       "🚀 {version}"
#   )

# Template: basic (single line)
TEMPLATE_BASIC=(
    "🤖 {model} | "
    "💰 {cost} | "
    "📈 Session: {session} (Resets in {session_reset}) | "
    "📅 Weekly: {weekly} (Resets {weekly_reset}) | "
    "🧠 Context: {context}"
)

# Template: extended (two lines)
TEMPLATE_EXTENDED=(
    "🤖 {model} | "
    "💰 {cost} | "
    "⏱️ {duration} | "
    "📈 Session: {session} (Resets in {session_reset}) | "
    "📅 Weekly: {weekly} (Resets {weekly_reset}) | "
    "🧠 Context: {context}"
    ---
    "🚀 Claude Code {version} | "
    "⬇️ Tokens In: {tokens_in} | "
    "⬆️ Tokens Out: {tokens_out} | "
    "♻️ Cache: {cache}"
)

# =============================================================================
# Data fetching
# =============================================================================

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
cache_creation=$(echo "$input" | jq -r '.context_window.current_usage.cache_creation_input_tokens // 0')
new_input=$(echo "$input" | jq -r '.context_window.current_usage.input_tokens // 0')
cc_version=$(echo "$input" | jq -r '.version // "N/A"')

# From Anthropic API
session_pct=$(echo "$usage_response" | jq -r 'if .five_hour.utilization != null then ((.five_hour.utilization | tostring) + "%") else "N/A" end')
weekly_pct=$(echo "$usage_response" | jq -r 'if .seven_day.utilization != null then ((.seven_day.utilization | tostring) + "%") else "N/A" end')

# Calculate session reset time
resets_at=$(echo "$usage_response" | jq -r '.five_hour.resets_at // empty')
if [[ -n "$resets_at" ]]; then
    resets_epoch=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%S" "${resets_at%%.*}" +%s 2>/dev/null)
    now_epoch=$(date +%s)
    diff_seconds=$((resets_epoch - now_epoch))
    hours=$((diff_seconds / 3600))
    minutes=$(((diff_seconds % 3600) / 60))
    session_reset="${hours}h ${minutes}m"
else
    session_reset="N/A"
fi

# Calculate weekly reset time
weekly_resets_at=$(echo "$usage_response" | jq -r '.seven_day.resets_at // empty')
if [[ -n "$weekly_resets_at" ]]; then
    weekly_resets_epoch=$(TZ=UTC date -j -f "%Y-%m-%dT%H:%M:%S" "${weekly_resets_at%%.*}" +%s 2>/dev/null)
    weekly_reset=$(date -j -f "%s" "$weekly_resets_epoch" "+%a %-l:%M%p")
else
    weekly_reset="N/A"
fi

# Calculate duration
duration_min=$((duration_ms / 60000))
duration_sec=$(((duration_ms % 60000) / 1000))

# =============================================================================
# Placeholder definitions
# =============================================================================

# Each placeholder is: {name} -> value
# Placeholders are replaced in the template string

P_MODEL="$model_name"
P_COST="\$$session_cost"
P_SESSION="$session_pct"
P_SESSION_RESET="$session_reset"
P_WEEKLY="$weekly_pct"
P_WEEKLY_RESET="$weekly_reset"
P_CONTEXT="$context_usage"
P_DURATION="${duration_min}m ${duration_sec}s"
P_TOKENS_IN=$(printf "%'d" "$total_input")
P_TOKENS_OUT=$(printf "%'d" "$total_output")
# Calculate cache hit percentage
cache_total=$((cache_read + cache_creation + new_input))
if [[ $cache_total -gt 0 ]]; then
    cache_pct=$((cache_read * 100 / cache_total))
else
    cache_pct=0
fi
P_CACHE="${cache_pct}% ($(printf "%'d" "$cache_read"))"
P_VERSION="v$cc_version"

# =============================================================================
# Render template
# =============================================================================

render_line() {
    local line="$1"

    # Replace all placeholders
    line="${line//\{model\}/$P_MODEL}"
    line="${line//\{cost\}/$P_COST}"
    line="${line//\{session\}/$P_SESSION}"
    line="${line//\{session_reset\}/$P_SESSION_RESET}"
    line="${line//\{weekly\}/$P_WEEKLY}"
    line="${line//\{weekly_reset\}/$P_WEEKLY_RESET}"
    line="${line//\{context\}/$P_CONTEXT}"
    line="${line//\{duration\}/$P_DURATION}"
    line="${line//\{tokens_in\}/$P_TOKENS_IN}"
    line="${line//\{tokens_out\}/$P_TOKENS_OUT}"
    line="${line//\{cache\}/$P_CACHE}"
    line="${line//\{version\}/$P_VERSION}"

    echo "$line"
}

# Get template based on selected template
template_upper=$(echo "$TEMPLATE" | tr '[:lower:]' '[:upper:]')
template_var="TEMPLATE_${template_upper}[@]"
template_arr=("${!template_var}")

# Process template: split by --- and output each line
current_line=""
for item in "${template_arr[@]}"; do
    if [[ "$item" == "---" ]]; then
        # Output current line and start new one
        if [[ -n "$current_line" ]]; then
            render_line "$current_line"
        fi
        current_line=""
    else
        current_line+="$item"
    fi
done

# Output last line
if [[ -n "$current_line" ]]; then
    render_line "$current_line"
fi
