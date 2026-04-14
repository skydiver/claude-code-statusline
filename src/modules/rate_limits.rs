//! `$session` / `$session_reset` / `$weekly` / `$weekly_reset` — Claude Code
//! usage rate-limit windows.
//!
//! Reads `rate_limits.five_hour` (session/countdown) and
//! `rate_limits.seven_day` (weekly/formatted date). All four render `N/A`
//! when the underlying field is missing, matching the shell script.
//!
//! - `$session` and `$weekly`: `used_percentage` rounded to an integer, suffixed `%`.
//! - `$session_reset`: countdown `Xh Ym` from wall clock to `resets_at`.
//! - `$weekly_reset`: localized date formatted in the system timezone via
//!   `jiff` (`Mon 3:00PM`). No subprocess, no platform-specific `date` flags.

use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::input::{Input, RateLimitWindow};

const NA: &str = "N/A";
const WEEKLY_FMT: &str = "%a %-I:%M%p";

fn five_hour(input: &Input) -> Option<&RateLimitWindow> {
    input.rate_limits.as_ref().and_then(|r| r.five_hour.as_ref())
}

fn seven_day(input: &Input) -> Option<&RateLimitWindow> {
    input.rate_limits.as_ref().and_then(|r| r.seven_day.as_ref())
}

fn percent_or_na(window: Option<&RateLimitWindow>) -> String {
    window
        .and_then(|w| w.used_percentage)
        .map(|p| format!("{}%", p.round() as i64))
        .unwrap_or_else(|| NA.to_string())
}

fn countdown(resets_at: i64, now: i64) -> String {
    let diff = (resets_at - now).max(0);
    let hours = diff / 3600;
    let minutes = (diff % 3600) / 60;
    format!("{hours}h {minutes}m")
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn format_weekly_date(epoch: i64) -> Option<String> {
    format_weekly_date_in_tz(epoch, TimeZone::system())
}

fn format_weekly_date_in_tz(epoch: i64, tz: TimeZone) -> Option<String> {
    let ts = Timestamp::from_second(epoch).ok()?;
    let zoned = ts.to_zoned(tz);
    jiff::fmt::strtime::format(WEEKLY_FMT, &zoned).ok()
}

pub fn render_session(input: &Input) -> String {
    percent_or_na(five_hour(input))
}

pub fn render_weekly(input: &Input) -> String {
    percent_or_na(seven_day(input))
}

pub fn render_session_reset(input: &Input) -> String {
    session_reset_with_now(input, now_epoch())
}

pub fn render_weekly_reset(input: &Input) -> String {
    seven_day(input)
        .and_then(|w| w.resets_at)
        .and_then(format_weekly_date)
        .unwrap_or_else(|| NA.to_string())
}

fn session_reset_with_now(input: &Input, now: i64) -> String {
    five_hour(input)
        .and_then(|w| w.resets_at)
        .map(|resets| countdown(resets, now))
        .unwrap_or_else(|| NA.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{RateLimitWindow, RateLimits};

    fn with_windows(five: Option<(f64, i64)>, seven: Option<(f64, i64)>) -> Input {
        Input {
            rate_limits: Some(RateLimits {
                five_hour: five.map(|(pct, reset)| RateLimitWindow {
                    used_percentage: Some(pct),
                    resets_at: Some(reset),
                }),
                seven_day: seven.map(|(pct, reset)| RateLimitWindow {
                    used_percentage: Some(pct),
                    resets_at: Some(reset),
                }),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn session_renders_rounded_percent() {
        let input = with_windows(Some((17.5, 0)), None);
        assert_eq!(render_session(&input), "18%");
    }

    #[test]
    fn weekly_renders_rounded_percent() {
        let input = with_windows(None, Some((12.3, 0)));
        assert_eq!(render_weekly(&input), "12%");
    }

    #[test]
    fn session_falls_back_to_na_when_missing() {
        assert_eq!(render_session(&Input::default()), "N/A");
    }

    #[test]
    fn weekly_falls_back_to_na_when_missing() {
        assert_eq!(render_weekly(&Input::default()), "N/A");
    }

    #[test]
    fn countdown_renders_hours_and_minutes() {
        // resets_at = 10000, now = 3640 → diff 6360s = 1h 46m
        assert_eq!(countdown(10_000, 3_640), "1h 46m");
    }

    #[test]
    fn countdown_clamps_to_zero_when_already_past() {
        assert_eq!(countdown(100, 500), "0h 0m");
    }

    #[test]
    fn session_reset_uses_injected_now() {
        let input = with_windows(Some((17.0, 11_000)), None);
        assert_eq!(session_reset_with_now(&input, 10_000), "0h 16m");
    }

    #[test]
    fn session_reset_na_when_missing() {
        assert_eq!(session_reset_with_now(&Input::default(), 0), "N/A");
    }

    #[test]
    fn weekly_reset_na_when_missing() {
        assert_eq!(render_weekly_reset(&Input::default()), "N/A");
    }

    #[test]
    fn format_weekly_date_renders_pm_case() {
        // 2023-11-14 22:13:20 UTC → "Tue 10:13PM"
        assert_eq!(
            format_weekly_date_in_tz(1_700_000_000, TimeZone::UTC),
            Some("Tue 10:13PM".into())
        );
    }

    #[test]
    fn format_weekly_date_renders_am_case() {
        // 2023-11-15 08:00:00 UTC → "Wed 8:00AM"
        assert_eq!(
            format_weekly_date_in_tz(1_700_035_200, TimeZone::UTC),
            Some("Wed 8:00AM".into())
        );
    }

    #[test]
    fn format_weekly_date_rejects_invalid_epoch() {
        assert_eq!(format_weekly_date_in_tz(i64::MAX, TimeZone::UTC), None);
    }
}
