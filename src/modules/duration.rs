//! `$duration` — session active duration as `Xm Ys`.
//!
//! Reads `cost.total_duration_ms` (cumulative API+tool processing time).
//! Falls back to `0m 0s` to match the shell script's `// 0` default.

use crate::input::Input;

pub fn render(input: &Input) -> String {
    let ms = input
        .cost
        .as_ref()
        .and_then(|c| c.total_duration_ms)
        .unwrap_or(0);
    let mins = ms / 60_000;
    let secs = (ms % 60_000) / 1000;
    format!("{mins}m {secs}s")
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
    fn renders_sub_minute_as_zero_minutes() {
        assert_eq!(render(&with_ms(42_000)), "0m 42s");
    }

    #[test]
    fn renders_zero_when_missing() {
        assert_eq!(render(&Input::default()), "0m 0s");
    }

    #[test]
    fn drops_sub_second_remainder() {
        // 60_999 ms → 1m 0s (the extra 999 ms is truncated, not rounded)
        assert_eq!(render(&with_ms(60_999)), "1m 0s");
    }
}
