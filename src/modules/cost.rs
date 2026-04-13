//! `$cost` — session cost formatted as `$X.XX`.
//!
//! Matches the shell script's behavior of always showing a dollar amount
//! even when `cost.total_cost_usd` is missing — falls back to `$0.00`.

use crate::input::Input;

pub fn render(input: &Input) -> String {
    let usd = input
        .cost
        .as_ref()
        .and_then(|c| c.total_cost_usd)
        .unwrap_or(0.0);
    format!("${usd:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Cost;

    fn with_usd(usd: f64) -> Input {
        Input {
            cost: Some(Cost {
                total_cost_usd: Some(usd),
                total_duration_ms: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn renders_two_decimal_places() {
        assert_eq!(render(&with_usd(1.79)), "$1.79");
    }

    #[test]
    fn rounds_to_two_decimals() {
        assert_eq!(render(&with_usd(1.7955)), "$1.80");
    }

    #[test]
    fn pads_single_decimal() {
        assert_eq!(render(&with_usd(0.5)), "$0.50");
    }

    #[test]
    fn renders_zero_when_cost_missing() {
        assert_eq!(render(&Input::default()), "$0.00");
    }
}
