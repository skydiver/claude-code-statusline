//! Config: top-level `format` string + (eventually) per-module overrides.
//!
//! Phase 2 keeps this minimal — just `format`. Phase 4 will add TOML loading
//! from `~/.config/claude-code-statusline/config.toml` and per-module sections.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format: String::from("🤖 $model | 💰 $cost | 📁 $project"),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // Phase 4: read ~/.config/claude-code-statusline/config.toml.
        // For now, always fall back to the baked-in default.
        Self::default()
    }
}
