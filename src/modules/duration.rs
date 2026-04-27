//! `$duration` — session wall-clock time.
//!
//! Reads `cost.total_duration_ms` — per Claude Code's statusline schema, this
//! is total wall-clock time since the session started, not API time (which is
//! `total_api_duration_ms`, not currently exposed as a variable).
//!
//! Formatted as the two most significant non-zero units so output stays
//! compact at every scale: `42s`, `5m 29s`, `7h 5m`, `2d 3h`. Falls back to
//! `0s` when the field is missing. Sub-second remainders are truncated.

use crate::input::Input;

pub fn render(input: &Input) -> String {
    let ms = input
        .cost
        .as_ref()
        .and_then(|c| c.total_duration_ms)
        .unwrap_or(0);

    let total_secs = ms / 1000;
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Cost;

    fn with_ms(ms: u64) -> Input {
        Input {
            cost: Some(Cost {
                total_cost_usd: None,
                total_duration_ms: Some(ms),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn renders_minutes_and_seconds() {
        assert_eq!(render(&with_ms(425_000)), "7m 5s");
    }

    #[test]
    fn renders_sub_minute_as_seconds_only() {
        assert_eq!(render(&with_ms(42_000)), "42s");
    }

    #[test]
    fn renders_zero_when_missing() {
        assert_eq!(render(&Input::default()), "0s");
    }

    #[test]
    fn drops_sub_second_remainder() {
        // 60_999 ms → 1m 0s (the extra 999 ms is truncated, not rounded)
        assert_eq!(render(&with_ms(60_999)), "1m 0s");
    }

    #[test]
    fn renders_hours_and_minutes_past_one_hour() {
        // 7h 5m 29s → seconds are dropped once hours are present.
        assert_eq!(render(&with_ms(25_529_000)), "7h 5m");
    }

    #[test]
    fn renders_exact_hour_boundary() {
        // Exactly 1h — minutes field is zero but still present.
        assert_eq!(render(&with_ms(3_600_000)), "1h 0m");
    }

    #[test]
    fn renders_days_and_hours_past_one_day() {
        // 2d 3h → minutes are dropped once days are present.
        assert_eq!(render(&with_ms(183_600_000)), "2d 3h");
    }
}
