//! `$version` — Claude Code version prefixed with `v`.

use crate::input::Input;

pub fn render(input: &Input) -> String {
    match input.version.as_deref() {
        Some(v) => format!("v{v}"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_prefixed_version() {
        let input = Input {
            version: Some("2.0.76".into()),
            ..Default::default()
        };
        assert_eq!(render(&input), "v2.0.76");
    }

    #[test]
    fn renders_empty_when_missing() {
        let input = Input::default();
        assert_eq!(render(&input), "");
    }
}
