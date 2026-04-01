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
#                   {weekly}, {weekly_reset}, {context}, {context_bar},
#                   {tokens_in}, {tokens_out}, {cache}, {version},
#                   {git_branch}
#   - Add any literal text, emojis, or formatting around placeholders
#
# Examples:
#   "🤖 {model}"                            -> 🤖 Opus 4.6
#   "💰 {cost}"                             -> 💰 $1.79
#   "📈 Session: {session}"                 -> 📈 Session: 17%
#   "{session} (Resets in {session_reset})" -> 17% (Resets in 0h 31m)
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
    "📈 Session: {session} [{session_reset}] | "
    "📅 Weekly: {weekly} [{weekly_reset}] | "
    "🧠 {context_bar}"
    "{git_branch_sep}"
)

# Template: extended (two lines)
TEMPLATE_EXTENDED=(
    "🤖 {model} | "
    "💰 {cost} | "
    "⏱️ {duration} | "
    "📈 Session: {session} [{session_reset}] | "
    "📅 Weekly: {weekly} [{weekly_reset}] | "
    "🧠 Context: {context}"
    ---
    "🚀 Claude Code {version} | "
    "⬇️ Tokens In: {tokens_in} | "
    "⬆️ Tokens Out: {tokens_out} | "
    "♻️ Cache: {cache}"
    "{git_branch_sep}"
)

# =============================================================================
# Extract raw data
# =============================================================================

# Read JSON input from stdin
input=$(cat)

# Model, cost, context, and tokens
model_name=$(echo "$input" | jq -r '.model.display_name')
session_cost=$(echo "$input" | jq -r '(.cost.total_cost_usd // 0) | . * 100 | round / 100 | tostring | if contains(".") then (. + "00")[0:index(".")+3] else . + ".00" end')
context_pct=$(echo "$input" | jq -r '.context_window.used_percentage // 0')
context_size=$(echo "$input" | jq -r '.context_window.context_window_size // 0')
duration_ms=$(echo "$input" | jq -r '.cost.total_duration_ms // 0')
total_input=$(echo "$input" | jq -r '.context_window.total_input_tokens // 0')
total_output=$(echo "$input" | jq -r '.context_window.total_output_tokens // 0')
cache_read=$(echo "$input" | jq -r '.context_window.current_usage.cache_read_input_tokens // 0')
cache_creation=$(echo "$input" | jq -r '.context_window.current_usage.cache_creation_input_tokens // 0')
new_input=$(echo "$input" | jq -r '.context_window.current_usage.input_tokens // 0')
cc_version=$(echo "$input" | jq -r '.version // "N/A"')

# Git branch (empty if not in a git repo)
if git rev-parse --is-inside-work-tree &>/dev/null; then
    git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
else
    git_branch=""
fi

# Rate limits (from input JSON)
session_pct=$(echo "$input" | jq -r '.rate_limits.five_hour.used_percentage // empty | tostring + "%"')
weekly_pct=$(echo "$input" | jq -r '.rate_limits.seven_day.used_percentage // empty | tostring + "%"')
session_pct="${session_pct:-N/A}"
weekly_pct="${weekly_pct:-N/A}"

# Calculate session reset time
resets_epoch=$(echo "$input" | jq -r '.rate_limits.five_hour.resets_at // empty')
if [[ -n "$resets_epoch" ]]; then
    now_epoch=$(date +%s)
    diff_seconds=$((resets_epoch - now_epoch))
    hours=$((diff_seconds / 3600))
    minutes=$(((diff_seconds % 3600) / 60))
    session_reset="${hours}h ${minutes}m"
else
    session_reset="N/A"
fi

# Calculate weekly reset time
weekly_resets_epoch=$(echo "$input" | jq -r '.rate_limits.seven_day.resets_at // empty')
if [[ -n "$weekly_resets_epoch" ]]; then
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
P_CONTEXT="${context_pct}%"

# Build context progress bar
bar_width=10
filled=$((context_pct * bar_width / 100))
bar=""
for ((i=0; i<bar_width; i++)); do
    if ((i < filled)); then bar+="█"; else bar+="░"; fi
done
tokens_used_k=$((context_pct * context_size / 100 / 1000))
tokens_total_k=$((context_size / 1000))
P_CONTEXT_BAR="${bar} ${context_pct}% (${tokens_used_k}k/${tokens_total_k}k)"
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
P_GIT_BRANCH="$git_branch"
# Conditional separator: " | 🌿 branch" if in a repo, empty otherwise
if [[ -n "$git_branch" ]]; then
    P_GIT_BRANCH_SEP=" | 🌿 $git_branch"
else
    P_GIT_BRANCH_SEP=""
fi

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
    line="${line//\{context_bar\}/$P_CONTEXT_BAR}"
    line="${line//\{version\}/$P_VERSION}"
    line="${line//\{git_branch\}/$P_GIT_BRANCH}"
    line="${line//\{git_branch_sep\}/$P_GIT_BRANCH_SEP}"

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
