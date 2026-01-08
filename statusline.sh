#!/bin/bash

# =============================================================================
# Configuration
# =============================================================================

TEMPLATE="extended"  # Options: basic, extended

# Template: basic (single line)
TEMPLATE_BASIC_LINE1="model | cost | session | weekly | context"

# Template: extended (two lines)
TEMPLATE_EXTENDED_LINE1="model | cost duration | session | weekly | context"
TEMPLATE_EXTENDED_LINE2="version | tokens | cache"

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
cc_version=$(echo "$input" | jq -r '.version // "N/A"')

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
# Widget definitions
# =============================================================================

get_widget() {
    case "$1" in
        model)    echo "🤖 $model_name" ;;
        cost)     echo "💰 \$$session_cost" ;;
        session)  echo "📈 Session: $current_session ($resets_in)" ;;
        weekly)   echo "📅 Weekly: $weekly ($weekly_resets_formatted)" ;;
        context)  echo "🧠 Context: $context_usage" ;;
        duration) echo "⏱️ ${duration_min}m ${duration_sec}s" ;;
        tokens)   echo "🔤 In: $total_input Out: $total_output" ;;
        cache)    echo "💾 Cache: $cache_read" ;;
        version)  echo "🚀 Claude Code v$cc_version" ;;
        *)        echo "" ;;
    esac
}

# =============================================================================
# Render template
# =============================================================================

render_line() {
    local template="$1"
    local output=""
    local group_output=""
    local widget_value

    # Split by | to get groups
    IFS='|' read -ra groups <<< "$template"

    for group in "${groups[@]}"; do
        group_output=""
        # Trim whitespace and process widgets in group
        for widget in $group; do
            widget_value=$(get_widget "$widget")
            if [[ -n "$widget_value" ]]; then
                if [[ -n "$group_output" ]]; then
                    group_output="$group_output $widget_value"
                else
                    group_output="$widget_value"
                fi
            fi
        done
        # Add group to output with | separator
        if [[ -n "$group_output" ]]; then
            if [[ -n "$output" ]]; then
                output="$output | $group_output"
            else
                output="$group_output"
            fi
        fi
    done
    echo "$output"
}

# Get template config based on selected template
template_upper=$(echo "$TEMPLATE" | tr '[:lower:]' '[:upper:]')
line1_var="TEMPLATE_${template_upper}_LINE1"
line2_var="TEMPLATE_${template_upper}_LINE2"

# Output lines
if [[ -n "${!line1_var}" ]]; then
    render_line "${!line1_var}"
fi

if [[ -n "${!line2_var}" ]]; then
    render_line "${!line2_var}"
fi
