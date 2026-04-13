//! `$cache` — cache hit percentage and cache-read token count.
//!
//! Mirrors the shell script: percentage of `cache_read / (cache_read +
//! cache_creation + input)`, followed by the comma-formatted cache_read
//! count in parentheses. Falls back to `0% (0)` when the relevant fields
//! are missing or the denominator is zero.

use super::format_with_commas;
use crate::input::Input;

pub fn render(input: &Input) -> String {
    let usage = input
        .context_window
        .as_ref()
        .and_then(|c| c.current_usage.as_ref());

    let cache_read = usage.and_then(|u| u.cache_read_input_tokens).unwrap_or(0);
    let cache_creation = usage.and_then(|u| u.cache_creation_input_tokens).unwrap_or(0);
    let new_input = usage.and_then(|u| u.input_tokens).unwrap_or(0);

    let total = cache_read + cache_creation + new_input;
    let pct = if total > 0 {
        cache_read * 100 / total
    } else {
        0
    };

    format!("{pct}% ({})", format_with_commas(cache_read))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{ContextWindow, CurrentUsage};

    fn with_usage(read: u64, creation: u64, new_input: u64) -> Input {
        Input {
            context_window: Some(ContextWindow {
                current_usage: Some(CurrentUsage {
                    input_tokens: Some(new_input),
                    cache_read_input_tokens: Some(read),
                    cache_creation_input_tokens: Some(creation),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn renders_typical_hit_rate() {
        // 38000 read / (38000 + 5800 + 1200) = 38000/45000 ≈ 84%
        let input = with_usage(38_000, 5_800, 1_200);
        assert_eq!(render(&input), "84% (38,000)");
    }

    #[test]
    fn renders_zero_when_everything_missing() {
        assert_eq!(render(&Input::default()), "0% (0)");
    }

    #[test]
    fn renders_zero_pct_when_denominator_is_zero() {
        let input = with_usage(0, 0, 0);
        assert_eq!(render(&input), "0% (0)");
    }

    #[test]
    fn renders_hundred_percent_when_only_cache_read() {
        let input = with_usage(1_000, 0, 0);
        assert_eq!(render(&input), "100% (1,000)");
    }
}
