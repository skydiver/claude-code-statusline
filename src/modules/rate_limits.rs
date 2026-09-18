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
//!
//! `$weekly` is additionally wrapped in ANSI color by *burn pace* rather than
//! by an absolute threshold: 100% of the allowance spread evenly over seven
//! days is 14.29%/day, so the linear budget at any instant is the fraction of
//! the window already elapsed. Using faster than that budget means running out
//! before the reset — yellow from 1.0x the budget, red from 1.5x. See
//! `weekly_color` for the guards that keep the ratio honest.

use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::input::{Input, RateLimitWindow};
use crate::style::{ANSI_RED, ANSI_YELLOW, paint};

const NA: &str = "N/A";
const WEEKLY_FMT: &str = "%a %-I:%M%p";

/// Length of the `seven_day` window. The payload only carries `resets_at`,
/// so the window start is derived by subtracting this.
const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

/// Burn-rate multiples of the linear budget at which each band starts.
const PACE_YELLOW: f64 = 1.0;
const PACE_RED: f64 = 1.5;

/// Below this displayed percent the ratio is dominated by how little time has
/// elapsed, not by real consumption — a few percent minutes into a fresh
/// window is a 100x pace and means nothing. Stay silent instead.
const PACE_FLOOR_PCT: i64 = 5;

/// At or above this, the allowance is nearly gone no matter what the pace
/// says — six days in at 92% is "under budget" and still one session from the
/// wall. Red wins, and unlike the pace bands it needs no clock.
const CRITICAL_PCT: i64 = 90;

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
    weekly_with_now(input, now_epoch())
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

fn weekly_with_now(input: &Input, now: i64) -> String {
    let window = seven_day(input);
    let Some(used) = window.and_then(|w| w.used_percentage) else {
        return NA.to_string();
    };
    let rounded = used.round() as i64;
    let color = weekly_color(used, rounded, window.and_then(|w| w.resets_at), now);
    paint(color, format!("{rounded}%"))
}

/// Pick the band for `used`, gated on the *displayed* (`rounded`) value so the
/// number and its color never disagree. Returns `None` — plain text — whenever
/// the pace cannot be computed honestly.
fn weekly_color(used: f64, rounded: i64, resets_at: Option<i64>, now: i64) -> Option<&'static str> {
    if rounded < PACE_FLOOR_PCT {
        return None;
    }
    if rounded >= CRITICAL_PCT {
        return Some(ANSI_RED);
    }

    let elapsed = elapsed_in_window(resets_at?, now)?;
    let budget = elapsed as f64 / WEEK_SECS as f64 * 100.0;
    let ratio = used / budget;

    if ratio >= PACE_RED {
        Some(ANSI_RED)
    } else if ratio >= PACE_YELLOW {
        Some(ANSI_YELLOW)
    } else {
        None
    }
}

/// Seconds elapsed since the window opened, clamped to one window so a stale
/// payload reads as "budget fully spent" rather than overshooting. `None` when
/// the start is still in the future (clock skew), which would make the budget
/// zero or negative and the ratio meaningless.
fn elapsed_in_window(resets_at: i64, now: i64) -> Option<i64> {
    let elapsed = now - (resets_at - WEEK_SECS);
    (elapsed > 0).then(|| elapsed.min(WEEK_SECS))
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

    // --- $weekly burn-pace coloring -------------------------------------
    //
    // `NOW` is an arbitrary fixed epoch; each test positions `resets_at`
    // relative to it so `elapsed` — and therefore the linear budget — is
    // exact. Half a window (3.5 days) makes `expected` exactly 50.0, which
    // keeps the ratio boundaries free of float fuzz.

    const NOW: i64 = 1_700_000_000;
    const DAY: i64 = 86_400;

    fn weekly_window(used: Option<f64>, resets_at: Option<i64>) -> Input {
        Input {
            rate_limits: Some(RateLimits {
                five_hour: None,
                seven_day: Some(RateLimitWindow {
                    used_percentage: used,
                    resets_at,
                }),
            }),
            ..Default::default()
        }
    }

    /// Build an input whose weekly window is `elapsed` seconds old at `NOW`.
    fn weekly_at(used: f64, elapsed: i64) -> Input {
        weekly_window(Some(used), Some(NOW - elapsed + WEEK_SECS))
    }

    #[test]
    fn weekly_uncolored_when_under_pace() {
        // Day 1 of 7 → budget 14.29%. 10% used is a 0.70 ratio.
        assert_eq!(weekly_with_now(&weekly_at(10.0, DAY), NOW), "10%");
    }

    #[test]
    fn weekly_yellow_when_over_pace() {
        // Day 1 → 15% used is a 1.05 ratio: on track to run out early.
        assert_eq!(
            weekly_with_now(&weekly_at(15.0, DAY), NOW),
            "\x1b[33m15%\x1b[0m"
        );
    }

    #[test]
    fn weekly_red_when_far_over_pace() {
        // Day 1 → 22% used is a 1.54 ratio: past the red threshold.
        assert_eq!(
            weekly_with_now(&weekly_at(22.0, DAY), NOW),
            "\x1b[31m22%\x1b[0m"
        );
    }

    #[test]
    fn weekly_pace_boundaries_are_exact_at_half_window() {
        // Half the window → budget is exactly 50%, so the ratios land on
        // 0.98 / 1.00 / 1.50 with no floating-point slack.
        let half = WEEK_SECS / 2;
        assert_eq!(weekly_with_now(&weekly_at(49.0, half), NOW), "49%");
        assert_eq!(
            weekly_with_now(&weekly_at(50.0, half), NOW),
            "\x1b[33m50%\x1b[0m"
        );
        assert_eq!(
            weekly_with_now(&weekly_at(75.0, half), NOW),
            "\x1b[31m75%\x1b[0m"
        );
    }

    #[test]
    fn weekly_uncolored_below_floor_despite_wild_ratio() {
        // One hour in, 4% used is a ~168x ratio — noise, not a warning.
        assert_eq!(weekly_with_now(&weekly_at(4.0, 3_600), NOW), "4%");
    }

    #[test]
    fn weekly_colors_from_floor_upward() {
        // 5% is the first displayed value the pace gate acts on.
        assert_eq!(
            weekly_with_now(&weekly_at(5.0, 3_600), NOW),
            "\x1b[31m5%\x1b[0m"
        );
    }

    #[test]
    fn weekly_floor_follows_rounded_value() {
        // 4.5 displays as 5%, so the floor must let it through — display and
        // color stay in sync, same rule $context uses.
        assert_eq!(
            weekly_with_now(&weekly_at(4.5, 3_600), NOW),
            "\x1b[31m5%\x1b[0m"
        );
    }

    #[test]
    fn weekly_red_at_critical_even_when_under_pace() {
        // Day 6.5 → budget 92.86%, so 92% is technically under pace. Being
        // that close to the wall is red regardless of how you got there.
        assert_eq!(
            weekly_with_now(&weekly_at(92.0, WEEK_SECS * 13 / 14), NOW),
            "\x1b[31m92%\x1b[0m"
        );
    }

    #[test]
    fn weekly_uncolored_when_resets_at_missing() {
        // No clock, no pace: fall back to today's plain rendering.
        assert_eq!(
            weekly_with_now(&weekly_window(Some(50.0), None), NOW),
            "50%"
        );
    }

    #[test]
    fn weekly_red_at_critical_without_resets_at() {
        // The critical band needs no clock, so it still warns.
        assert_eq!(
            weekly_with_now(&weekly_window(Some(95.0), None), NOW),
            "\x1b[31m95%\x1b[0m"
        );
    }

    #[test]
    fn weekly_uncolored_when_window_starts_in_the_future() {
        // Clock skew or a reset more than a week out → no elapsed time to
        // divide by. Degrade to plain rather than guess.
        let input = weekly_window(Some(50.0), Some(NOW + WEEK_SECS + 3_600));
        assert_eq!(weekly_with_now(&input, NOW), "50%");
    }

    #[test]
    fn weekly_clamps_elapsed_for_stale_payload() {
        // resets_at already passed → clamp to a full window, budget 100%.
        let input = weekly_window(Some(60.0), Some(NOW - 3_600));
        assert_eq!(weekly_with_now(&input, NOW), "60%");
    }

    #[test]
    fn weekly_na_stays_uncolored() {
        assert_eq!(weekly_with_now(&weekly_window(None, Some(NOW)), NOW), "N/A");
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
