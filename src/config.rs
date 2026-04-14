//! Config: top-level `format` string + (eventually) per-module overrides.
//!
//! Resolution precedence:
//!
//! 1. `$CCLINE_CONFIG` — if set and non-empty, points at an explicit file.
//!    A missing or unreadable file at that path is logged to stderr (the
//!    user explicitly asked for it, so a typo shouldn't fail silently).
//! 2. `$XDG_CONFIG_HOME/claude-code-statusline/config.toml`
//! 3. `$HOME/.config/claude-code-statusline/config.toml`
//!
//! Uses `dirs::home_dir()` so Windows resolves via `%USERPROFILE%` and the
//! XDG-style layout stays consistent across macOS, Linux, and Windows — the
//! same convention starship, bat, and ripgrep use rather than Apple's
//! `~/Library/Application Support/`.
//!
//! Falls back to a baked-in default that matches the shell script's `basic`
//! template byte-for-byte. Load failures (missing file, malformed TOML)
//! degrade gracefully: the statusline must always print something — never
//! crash — so `load()` always returns a `Config`.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const DEFAULT_FORMAT: &str = "🤖 $model | 💰 $cost | 📈 $session [$session_reset] | 📅 $weekly [$weekly_reset] | 🧠 $context_bar ( | 🌿 $git_branch ) | 📁 $project";

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
        let Some((path, explicit)) = config_path() else {
            return Self::default();
        };
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                if explicit {
                    // Only log when the user explicitly pointed at a
                    // file via CCLINE_CONFIG — a missing XDG/HOME file
                    // is normal and shouldn't spam stderr.
                    eprintln!(
                        "ccline: failed to read CCLINE_CONFIG at {}: {e}",
                        path.display()
                    );
                }
                return Self::default();
            }
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

fn config_path() -> Option<(PathBuf, bool)> {
    let ccline = env::var_os("CCLINE_CONFIG");
    let xdg = env::var_os("XDG_CONFIG_HOME");
    let home = dirs::home_dir();
    resolve_config_path(ccline.as_deref(), xdg.as_deref(), home.as_deref())
}

fn resolve_config_path(
    ccline_config: Option<&OsStr>,
    xdg_config_home: Option<&OsStr>,
    home_dir: Option<&Path>,
) -> Option<(PathBuf, bool)> {
    if let Some(explicit) = ccline_config.filter(|v| !v.is_empty()) {
        return Some((PathBuf::from(explicit), true));
    }
    let base = if let Some(xdg) = xdg_config_home.filter(|v| !v.is_empty()) {
        PathBuf::from(xdg)
    } else {
        home_dir?.join(".config")
    };
    Some((
        base.join("claude-code-statusline").join("config.toml"),
        false,
    ))
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
            "$git_branch",
            "$project",
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
    fn resolve_config_path_prefers_explicit_ccline_config() {
        let (path, explicit) = resolve_config_path(
            Some(OsStr::new("/etc/ccline/custom.toml")),
            Some(OsStr::new("/xdg")),
            Some(Path::new("/home/martin")),
        )
        .expect("some");
        assert_eq!(path, PathBuf::from("/etc/ccline/custom.toml"));
        assert!(explicit);
    }

    #[test]
    fn resolve_config_path_empty_ccline_config_falls_through() {
        // Empty-string env vars are treated as unset — same rule we
        // already apply to XDG_CONFIG_HOME so the two vars behave
        // consistently.
        let (path, explicit) = resolve_config_path(
            Some(OsStr::new("")),
            Some(OsStr::new("/xdg")),
            Some(Path::new("/home/martin")),
        )
        .expect("some");
        assert_eq!(
            path,
            PathBuf::from("/xdg/claude-code-statusline/config.toml")
        );
        assert!(!explicit);
    }

    #[test]
    fn resolve_config_path_uses_xdg_when_ccline_config_unset() {
        let (path, explicit) = resolve_config_path(
            None,
            Some(OsStr::new("/xdg")),
            Some(Path::new("/home/martin")),
        )
        .expect("some");
        assert_eq!(
            path,
            PathBuf::from("/xdg/claude-code-statusline/config.toml")
        );
        assert!(!explicit);
    }

    #[test]
    fn resolve_config_path_falls_back_to_home_dir() {
        let (path, explicit) = resolve_config_path(
            None,
            None,
            Some(Path::new("/home/martin")),
        )
        .expect("some");
        assert_eq!(
            path,
            PathBuf::from("/home/martin/.config/claude-code-statusline/config.toml")
        );
        assert!(!explicit);
    }

    #[test]
    fn resolve_config_path_none_when_no_home_and_no_xdg() {
        assert!(resolve_config_path(None, None, None).is_none());
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
