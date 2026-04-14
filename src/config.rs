//! Config: top-level `format` string + (eventually) per-module overrides.
//!
//! Loads `~/.config/claude-code-statusline/config.toml`, respecting
//! `$XDG_CONFIG_HOME`. Falls back to a baked-in default that matches the
//! shell script's `basic` template byte-for-byte. Load failures (missing
//! file, malformed TOML) degrade gracefully: the statusline must always
//! print something — never crash — so `load()` always returns a `Config`.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

pub const DEFAULT_FORMAT: &str = "🤖 $model | 💰 $cost | 📈 Session: $session [$session_reset] | 📅 Weekly: $weekly [$weekly_reset] | 🧠 $context_bar$git_branch_sep";

#[derive(Debug, Deserialize)]
struct RawConfig {
    format: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: DEFAULT_FORMAT.to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        match Self::from_toml_str(&raw) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "ccline: failed to parse config at {}: {e}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        let raw: RawConfig = toml::from_str(s)?;
        let defaults = Self::default();
        Ok(Self {
            format: raw.format.unwrap_or(defaults.format),
        })
    }
}

fn config_path() -> Option<PathBuf> {
    let base = if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        PathBuf::from(xdg)
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return None;
    };
    Some(base.join("claude-code-statusline").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_format_matches_shell_basic_template() {
        let c = Config::default();
        // Every placeholder the shell's basic template renders must be present.
        for needle in [
            "$model",
            "$cost",
            "$session",
            "$session_reset",
            "$weekly",
            "$weekly_reset",
            "$context_bar",
            "$git_branch_sep",
        ] {
            assert!(
                c.format.contains(needle),
                "default format missing {needle}: {:?}",
                c.format
            );
        }
    }

    #[test]
    fn from_toml_str_overrides_format() {
        let c = Config::from_toml_str(r#"format = "$model — $cost""#).expect("parses");
        assert_eq!(c.format, "$model — $cost");
    }

    #[test]
    fn from_toml_str_missing_format_falls_back_to_default() {
        let c = Config::from_toml_str("").expect("empty toml parses");
        assert_eq!(c.format, DEFAULT_FORMAT);
    }

    #[test]
    fn from_toml_str_multiline_format_preserves_newlines() {
        let raw = "format = \"line1\\nline2\"";
        let c = Config::from_toml_str(raw).expect("parses");
        assert_eq!(c.format, "line1\nline2");
    }

    #[test]
    fn from_toml_str_surfaces_parse_errors() {
        assert!(Config::from_toml_str("format = ").is_err());
    }

    #[test]
    fn from_toml_str_ignores_unknown_fields() {
        // Forward-compat: future per-module sections should not break loading.
        let raw = r#"
format = "$model"

[git_branch]
symbol = "🌿"
"#;
        let c = Config::from_toml_str(raw).expect("unknown sections ignored");
        assert_eq!(c.format, "$model");
    }
}
