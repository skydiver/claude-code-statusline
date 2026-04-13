//! `$context` and `$context_bar` — used-percentage and a 10-char progress bar.
//!
//! Both read from `context_window.used_percentage` and (for the bar)
//! `context_window.context_window_size`. Percentages are rounded to the
//! nearest integer to match the shell script (see commit 1a12c71 — jq's
//! `round` avoids `28.000000000000004%` artifacts).
//!
//! Bar format: `██░░░░░░░░ 17% (34k/200k)` — 10 cells, full/empty blocks,
//! then the rounded percent, then used/total tokens in thousands. The
//! `tokens_used_k` figure is derived from `pct * size / 100 / 1000` so it
//! stays consistent with the bar fill rather than the raw token counts.

use crate::input::Input;

const BAR_WIDTH: u64 = 10;
const FILLED: char = '█';
const EMPTY: char = '░';

fn rounded_percent(input: &Input) -> u64 {
    input
        .context_window
        .as_ref()
        .and_then(|c| c.used_percentage)
        .map(|p| p.round() as u64)
        .unwrap_or(0)
}

fn context_size(input: &Input) -> u64 {
    input
        .context_window
        .as_ref()
        .and_then(|c| c.context_window_size)
        .unwrap_or(0)
}

pub fn render_percent(input: &Input) -> String {
    format!("{}%", rounded_percent(input))
}

pub fn render_bar(input: &Input) -> String {
    let pct = rounded_percent(input);
    let size = context_size(input);

    let filled = (pct * BAR_WIDTH / 100).min(BAR_WIDTH);
    let mut bar = String::with_capacity(BAR_WIDTH as usize * 3);
    for i in 0..BAR_WIDTH {
        bar.push(if i < filled { FILLED } else { EMPTY });
    }

    let tokens_used_k = pct * size / 100 / 1000;
    let tokens_total_k = size / 1000;

    format!("{bar} {pct}% ({tokens_used_k}k/{tokens_total_k}k)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ContextWindow;

    fn with_context(pct: f64, size: u64) -> Input {
        Input {
            context_window: Some(ContextWindow {
                used_percentage: Some(pct),
                context_window_size: Some(size),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn percent_rounds_half_up() {
        assert_eq!(render_percent(&with_context(17.3, 200_000)), "17%");
        assert_eq!(render_percent(&with_context(17.5, 200_000)), "18%");
        assert_eq!(render_percent(&with_context(28.000000000000004, 0)), "28%");
    }

    #[test]
    fn percent_falls_back_to_zero() {
        assert_eq!(render_percent(&Input::default()), "0%");
    }

    #[test]
    fn bar_renders_partial_fill() {
        // 17% of 10 cells → 1 filled, 9 empty. tokens_used = 17 * 200000/100/1000 = 34
        let input = with_context(17.3, 200_000);
        assert_eq!(render_bar(&input), "█░░░░░░░░░ 17% (34k/200k)");
    }

    #[test]
    fn bar_renders_empty_when_zero_percent() {
        let input = with_context(0.0, 200_000);
        assert_eq!(render_bar(&input), "░░░░░░░░░░ 0% (0k/200k)");
    }

    #[test]
    fn bar_renders_full_at_hundred_percent() {
        let input = with_context(100.0, 200_000);
        assert_eq!(render_bar(&input), "██████████ 100% (200k/200k)");
    }

    #[test]
    fn bar_clamps_above_hundred_percent() {
        // Defensive: if the server ever reports >100, the bar should stay 10 cells.
        let input = with_context(150.0, 200_000);
        let out = render_bar(&input);
        assert!(out.starts_with("██████████ 150%"));
    }

    #[test]
    fn bar_falls_back_to_zero_when_missing() {
        assert_eq!(render_bar(&Input::default()), "░░░░░░░░░░ 0% (0k/0k)");
    }
}
