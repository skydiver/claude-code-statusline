//! Format string parser and module dispatch.
//!
//! The format string uses Starship-style `$module_name` references. Everything
//! else is literal text emitted verbatim — including whitespace, emoji, and
//! any separators. A `$` followed by a non-identifier character (digit, space,
//! punctuation) stays literal, so `$5` is preserved as the text "$5".

use crate::input::Input;
use crate::modules;

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Literal(String),
    Module(String),
}

fn parse_format(fmt: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut literal = String::new();
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek().is_some_and(|&next| is_ident_start(next)) {
            if !literal.is_empty() {
                tokens.push(Token::Literal(std::mem::take(&mut literal)));
            }
            let mut name = String::new();
            while let Some(&next) = chars.peek() {
                if is_ident_continue(next) {
                    name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Module(name));
        } else {
            literal.push(c);
        }
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    tokens
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn dispatch(name: &str, input: &Input) -> String {
    match name {
        "model" => modules::model::render(input),
        unknown => {
            eprintln!("ccline: unknown module '${unknown}' in format string");
            String::new()
        }
    }
}

pub fn render(fmt: &str, input: &Input) -> String {
    let mut out = String::new();
    for tok in parse_format(fmt) {
        match tok {
            Token::Literal(s) => out.push_str(&s),
            Token::Module(name) => out.push_str(&dispatch(&name, input)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Model;

    #[test]
    fn parses_literal_only() {
        let tokens = parse_format("hello world");
        assert_eq!(tokens, vec![Token::Literal("hello world".into())]);
    }

    #[test]
    fn parses_single_module() {
        let tokens = parse_format("$model");
        assert_eq!(tokens, vec![Token::Module("model".into())]);
    }

    #[test]
    fn parses_mixed_literal_and_module() {
        let tokens = parse_format("🤖 $model | 💰");
        assert_eq!(
            tokens,
            vec![
                Token::Literal("🤖 ".into()),
                Token::Module("model".into()),
                Token::Literal(" | 💰".into()),
            ]
        );
    }

    #[test]
    fn dollar_before_digit_stays_literal() {
        // $5 — digit can't start an identifier → $ stays in the literal.
        let tokens = parse_format("cost $5");
        assert_eq!(tokens, vec![Token::Literal("cost $5".into())]);
    }

    #[test]
    fn renders_end_to_end() {
        let input = Input {
            model: Some(Model {
                display_name: Some("Opus 4.6".into()),
            }),
            ..Default::default()
        };
        let out = render("🤖 $model", &input);
        assert_eq!(out, "🤖 Opus 4.6");
    }

    #[test]
    fn unknown_module_renders_empty() {
        let input = Input::default();
        let out = render("[$nope]", &input);
        assert_eq!(out, "[]");
    }
}
