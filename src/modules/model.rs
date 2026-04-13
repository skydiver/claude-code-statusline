//! `$model` — Claude model display name.

use crate::input::Input;

pub fn render(input: &Input) -> String {
    input
        .model
        .as_ref()
        .and_then(|m| m.display_name.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Model;

    #[test]
    fn renders_display_name() {
        let input = Input {
            model: Some(Model {
                display_name: Some("Opus 4.6".into()),
            }),
            ..Default::default()
        };
        assert_eq!(render(&input), "Opus 4.6");
    }

    #[test]
    fn renders_empty_when_model_missing() {
        let input = Input::default();
        assert_eq!(render(&input), "");
    }

    #[test]
    fn renders_empty_when_display_name_missing() {
        let input = Input {
            model: Some(Model { display_name: None }),
            ..Default::default()
        };
        assert_eq!(render(&input), "");
    }
}
