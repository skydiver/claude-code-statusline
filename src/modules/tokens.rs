//! `$tokens_in` and `$tokens_out` — total input/output token counts with
//! comma thousands separators. Reads from `context_window.total_input_tokens`
//! and `context_window.total_output_tokens`.

use super::format_with_commas;
use crate::input::Input;

pub fn render_in(input: &Input) -> String {
    format_with_commas(
        input
            .context_window
            .as_ref()
            .and_then(|c| c.total_input_tokens)
            .unwrap_or(0),
    )
}

pub fn render_out(input: &Input) -> String {
    format_with_commas(
        input
            .context_window
            .as_ref()
            .and_then(|c| c.total_output_tokens)
            .unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ContextWindow;

    fn with_tokens(input_tokens: u64, output_tokens: u64) -> Input {
        Input {
            context_window: Some(ContextWindow {
                total_input_tokens: Some(input_tokens),
                total_output_tokens: Some(output_tokens),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn renders_with_commas() {
        let input = with_tokens(1_234_567, 89_012);
        assert_eq!(render_in(&input), "1,234,567");
        assert_eq!(render_out(&input), "89,012");
    }

    #[test]
    fn renders_zero_when_missing() {
        assert_eq!(render_in(&Input::default()), "0");
        assert_eq!(render_out(&Input::default()), "0");
    }
}
