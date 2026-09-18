//! Shared ANSI styling helpers (Phase 3 target: a starship-style style parser).
//!
//! Modules that color their output by threshold share these codes and the
//! `paint` wrapper, so a body is never emitted with a dangling reset.

pub const ANSI_RED: &str = "\x1b[31m";
pub const ANSI_YELLOW: &str = "\x1b[33m";
pub const ANSI_RESET: &str = "\x1b[0m";

/// Wrap `body` in `code`, or return it untouched when `code` is `None`.
pub fn paint(code: Option<&str>, body: String) -> String {
    match code {
        Some(code) => format!("{code}{body}{ANSI_RESET}"),
        None => body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_wraps_body_in_code_and_reset() {
        assert_eq!(paint(Some(ANSI_RED), "75%".into()), "\x1b[31m75%\x1b[0m");
    }

    #[test]
    fn paint_returns_body_untouched_when_no_code() {
        assert_eq!(paint(None, "42%".into()), "42%");
    }
}
