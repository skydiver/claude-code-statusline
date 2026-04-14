//! Format string parser and module dispatch.
//!
//! The format string uses Starship-style `$module_name` references. Everything
//! else is literal text emitted verbatim — including whitespace, emoji, and
//! any separators. A `$` followed by a non-identifier character (digit, space,
//! punctuation) stays literal, so `$5` is preserved as the text "$5".
//!
//! `(...)` groups are conditional: if any direct-child `$module` inside a
//! group renders empty, the entire group is suppressed. Groups are how you
//! express "show this only when the value is present" without polluting
//! modules with hidden separators. Nested groups are supported — an inner
//! group's emptiness does not bubble up to its parent. Unmatched `(` silently
//! extends to end-of-string; unmatched `)` is treated as literal.
//!
//! `$git_branch_sep` is kept as a back-compat parser alias that expands to
//! `( | 🌿 $git_branch)` at parse time, so existing user configs keep
//! working. New configs should use the explicit group form.

use std::iter::Peekable;
use std::str::Chars;

use crate::input::Input;
use crate::modules;

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Literal(String),
    Module(String),
    Group(Vec<Token>),
}

fn parse_format(fmt: &str) -> Vec<Token> {
    let mut chars = fmt.chars().peekable();
    parse_tokens(&mut chars, false)
}

fn parse_tokens(chars: &mut Peekable<Chars>, in_group: bool) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut literal = String::new();

    while let Some(&c) = chars.peek() {
        if in_group && c == ')' {
            chars.next();
            flush_literal(&mut tokens, &mut literal);
            return tokens;
        }

        if c == '(' {
            chars.next();
            // A space typed right before `(` reads as "part of the
            // conditional region" — so absorb trailing whitespace from the
            // pending literal into the group. To keep old `$context_bar( |
            // 🌿 ...)` source and new `$context_bar ( | 🌿 ...)` source
            // rendering identically, we also strip leading whitespace from
            // the group's first literal child so the two don't double up.
            let absorbed = take_trailing_whitespace(&mut literal);
            flush_literal(&mut tokens, &mut literal);
            let mut children = parse_tokens(chars, true);
            if !absorbed.is_empty() {
                match children.first_mut() {
                    Some(Token::Literal(first)) => {
                        let merged = format!("{}{}", absorbed, first.trim_start());
                        if merged.is_empty() {
                            children.remove(0);
                        } else {
                            *first = merged;
                        }
                    }
                    _ => children.insert(0, Token::Literal(absorbed)),
                }
            }
            tokens.push(Token::Group(children));
            continue;
        }

        if c == '$' {
            chars.next();
            if chars.peek().is_some_and(|&next| is_ident_start(next)) {
                flush_literal(&mut tokens, &mut literal);
                let name = consume_ident(chars);
                tokens.push(expand_module(name));
            } else {
                literal.push('$');
            }
            continue;
        }

        chars.next();
        literal.push(c);
    }

    flush_literal(&mut tokens, &mut literal);
    tokens
}

fn flush_literal(tokens: &mut Vec<Token>, literal: &mut String) {
    if !literal.is_empty() {
        tokens.push(Token::Literal(std::mem::take(literal)));
    }
}

fn take_trailing_whitespace(literal: &mut String) -> String {
    let trimmed_len = literal.trim_end().len();
    literal.split_off(trimmed_len)
}

fn consume_ident(chars: &mut Peekable<Chars>) -> String {
    let mut name = String::new();
    while let Some(&next) = chars.peek() {
        if is_ident_continue(next) {
            name.push(next);
            chars.next();
        } else {
            break;
        }
    }
    name
}

fn expand_module(name: String) -> Token {
    // Back-compat alias: `$git_branch_sep` → `( | 🌿 $git_branch)`. Old
    // configs keep rendering identically without touching their TOML.
    if name == "git_branch_sep" {
        return Token::Group(vec![
            Token::Literal(" | 🌿 ".into()),
            Token::Module("git_branch".into()),
        ]);
    }
    Token::Module(name)
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
        "cost" => modules::cost::render(input),
        "project" => modules::project::render(input),
        "version" => modules::version::render(input),
        "duration" => modules::duration::render(input),
        "tokens_in" => modules::tokens::render_in(input),
        "tokens_out" => modules::tokens::render_out(input),
        "cache" => modules::cache::render(input),
        "context" => modules::context::render_percent(input),
        "context_bar" => modules::context::render_bar(input),
        "session" => modules::rate_limits::render_session(input),
        "session_reset" => modules::rate_limits::render_session_reset(input),
        "weekly" => modules::rate_limits::render_weekly(input),
        "weekly_reset" => modules::rate_limits::render_weekly_reset(input),
        "git_branch" => modules::git_branch::render_name(input),
        unknown => {
            eprintln!("ccline: unknown module '${unknown}' in format string");
            String::new()
        }
    }
}

pub fn render(fmt: &str, input: &Input) -> String {
    let tokens = parse_format(fmt);
    render_tokens(&tokens, input)
}

fn render_tokens(tokens: &[Token], input: &Input) -> String {
    let mut out = String::new();
    for tok in tokens {
        match tok {
            Token::Literal(s) => out.push_str(s),
            Token::Module(name) => out.push_str(&dispatch(name, input)),
            Token::Group(children) => out.push_str(&render_group(children, input)),
        }
    }
    out
}

fn render_group(children: &[Token], input: &Input) -> String {
    // A group suppresses itself entirely if any direct-child `$module`
    // renders empty. Nested groups make their own independent decision —
    // their emptiness does not force the outer group to suppress.
    let mut parts: Vec<String> = Vec::with_capacity(children.len());
    for child in children {
        match child {
            Token::Literal(s) => parts.push(s.clone()),
            Token::Module(name) => {
                let rendered = dispatch(name, input);
                if rendered.is_empty() {
                    return String::new();
                }
                parts.push(rendered);
            }
            Token::Group(nested) => parts.push(render_group(nested, input)),
        }
    }
    parts.concat()
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
    fn parses_empty_group() {
        assert_eq!(parse_format("()"), vec![Token::Group(vec![])]);
    }

    #[test]
    fn parses_group_with_literal_only() {
        assert_eq!(
            parse_format("(abc)"),
            vec![Token::Group(vec![Token::Literal("abc".into())])]
        );
    }

    #[test]
    fn parses_group_with_module() {
        assert_eq!(
            parse_format("( | $model)"),
            vec![Token::Group(vec![
                Token::Literal(" | ".into()),
                Token::Module("model".into()),
            ])]
        );
    }

    #[test]
    fn parses_nested_group() {
        // The space between `$a` and `(` is absorbed into the inner group,
        // so it disappears together with `$b` when the inner group is empty.
        assert_eq!(
            parse_format("($a ($b))"),
            vec![Token::Group(vec![
                Token::Module("a".into()),
                Token::Group(vec![
                    Token::Literal(" ".into()),
                    Token::Module("b".into()),
                ]),
            ])]
        );
    }

    #[test]
    fn parses_group_in_context() {
        assert_eq!(
            parse_format("x( | $y)z"),
            vec![
                Token::Literal("x".into()),
                Token::Group(vec![
                    Token::Literal(" | ".into()),
                    Token::Module("y".into()),
                ]),
                Token::Literal("z".into()),
            ]
        );
    }

    #[test]
    fn unclosed_group_extends_to_eof() {
        assert_eq!(
            parse_format("($model"),
            vec![Token::Group(vec![Token::Module("model".into())])]
        );
    }

    #[test]
    fn stray_close_paren_is_literal() {
        assert_eq!(
            parse_format("foo)bar"),
            vec![Token::Literal("foo)bar".into())]
        );
    }

    #[test]
    fn git_branch_sep_alias_expands_to_group() {
        assert_eq!(
            parse_format("$git_branch_sep"),
            vec![Token::Group(vec![
                Token::Literal(" | 🌿 ".into()),
                Token::Module("git_branch".into()),
            ])]
        );
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

    #[test]
    fn literal_newline_passes_through_for_multiline_output() {
        // TOML decodes \n in a basic string to a real newline, and the parser
        // streams literal chars verbatim — so multi-line formats Just Work
        // without any dedicated rendering hook.
        let input = Input {
            model: Some(Model {
                display_name: Some("Opus 4.6".into()),
            }),
            ..Default::default()
        };
        let out = render("$model\nline 2", &input);
        assert_eq!(out, "Opus 4.6\nline 2");
    }

    fn input_with_model() -> Input {
        Input {
            model: Some(Model {
                display_name: Some("Opus 4.6".into()),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn group_renders_when_module_present() {
        let out = render("x( | $model)y", &input_with_model());
        assert_eq!(out, "x | Opus 4.6y");
    }

    #[test]
    fn group_suppresses_when_module_empty() {
        // $version is None → renders empty → group vanishes, separator and all.
        let out = render("x( | $version)y", &Input::default());
        assert_eq!(out, "xy");
    }

    #[test]
    fn group_with_multiple_modules_suppresses_on_any_empty() {
        // $model present, $version missing → still suppresses the whole group.
        let out = render("($model $version)", &input_with_model());
        assert_eq!(out, "");
    }

    #[test]
    fn nested_group_emptiness_does_not_bubble() {
        // Outer group has $model (present). Inner group has $version (empty).
        // Inner vanishes; outer still renders its non-empty parts.
        let out = render("($model( | $version))", &input_with_model());
        assert_eq!(out, "Opus 4.6");
    }

    #[test]
    fn space_before_group_is_absorbed_into_group() {
        // Whitespace immediately before `(` belongs to the conditional
        // region — so `a (X)` and `a(X)` parse to equivalent shapes and
        // `a (X)` suppresses the space along with the group when X is empty.
        assert_eq!(
            parse_format("a ($b)"),
            vec![
                Token::Literal("a".into()),
                Token::Group(vec![
                    Token::Literal(" ".into()),
                    Token::Module("b".into()),
                ]),
            ]
        );
    }

    #[test]
    fn absorbed_space_replaces_leading_space_inside_group() {
        // Both forms must render identically: the outer space is absorbed
        // and the inner leading space is stripped so we don't double up.
        assert_eq!(
            parse_format("a ( | $b)"),
            parse_format("a( | $b)"),
        );
    }

    #[test]
    fn absorbed_group_with_only_whitespace_inner_literal_drops_it() {
        // Inner is just `(  )` whitespace → trimming leaves nothing, so we
        // remove the emptied literal rather than keep an empty token.
        assert_eq!(
            parse_format("a (  $b)"),
            vec![
                Token::Literal("a".into()),
                Token::Group(vec![
                    Token::Literal(" ".into()),
                    Token::Module("b".into()),
                ]),
            ]
        );
    }

    #[test]
    fn absorbed_space_suppressed_when_group_empty() {
        // `$version` is None → the whole group (including absorbed space)
        // vanishes, leaving no dangling separator.
        let out = render("x ( | $version)y", &Input::default());
        assert_eq!(out, "xy");
    }

    #[test]
    fn new_and_old_group_syntax_render_identically() {
        let input = input_with_model();
        assert_eq!(
            render("$model ( | $model)", &input),
            render("$model( | $model)", &input),
        );
    }

    #[test]
    fn git_branch_sep_alias_renders_via_group() {
        // In this test environment we're inside a git repo, so the alias
        // should render the absorbed separator + the branch name.
        let out = render("$git_branch_sep", &Input::default());
        assert!(out.starts_with(" | 🌿 "), "got: {out:?}");
    }
}
