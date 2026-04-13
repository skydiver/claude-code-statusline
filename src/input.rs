//! Claude Code stdin JSON types.
//!
//! Every field is optional because Claude Code's statusline payload has
//! evolved over time — older versions miss `rate_limits`, some sessions miss
//! `context_window`, and anything nested inside can be absent. Serde's
//! default behavior also ignores unknown fields, so the struct stays
//! forward-compatible with new Claude Code releases.

// Many fields are consumed by Phase 2 modules, not by Phase 1 parsing/tests.
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Input {
    pub model: Option<Model>,
    pub version: Option<String>,
    pub cost: Option<Cost>,
    pub context_window: Option<ContextWindow>,
    pub rate_limits: Option<RateLimits>,
    pub workspace: Option<Workspace>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ContextWindow {
    pub used_percentage: Option<f64>,
    pub context_window_size: Option<u64>,
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Deserialize)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimits {
    pub five_hour: Option<RateLimitWindow>,
    pub seven_day: Option<RateLimitWindow>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitWindow {
    pub used_percentage: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Workspace {
    pub project_dir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_sample_fixture() {
        let raw = include_str!("../tests/fixtures/sample_input.json");
        let input: Input = serde_json::from_str(raw).expect("fixture should parse");

        assert_eq!(
            input.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("Opus 4.6")
        );
        assert_eq!(input.version.as_deref(), Some("2.0.76"));
        assert_eq!(
            input.cost.as_ref().and_then(|c| c.total_cost_usd),
            Some(1.79)
        );
        assert_eq!(
            input.cost.as_ref().and_then(|c| c.total_duration_ms),
            Some(425000)
        );

        let ctx = input.context_window.as_ref().expect("context_window set");
        assert_eq!(ctx.used_percentage, Some(17.3));
        assert_eq!(ctx.context_window_size, Some(200000));

        let five_hour = input
            .rate_limits
            .as_ref()
            .and_then(|r| r.five_hour.as_ref())
            .expect("five_hour set");
        assert_eq!(five_hour.used_percentage, Some(17.5));
        assert_eq!(five_hour.resets_at, Some(1744582800));

        assert_eq!(
            input
                .workspace
                .as_ref()
                .and_then(|w| w.project_dir.as_deref()),
            Some("/Users/example/projects/claude-code-statusline")
        );
    }

    #[test]
    fn parses_empty_object() {
        let input: Input = serde_json::from_str("{}").expect("empty object parses");
        assert!(input.model.is_none());
        assert!(input.cost.is_none());
        assert!(input.context_window.is_none());
        assert!(input.rate_limits.is_none());
        assert!(input.workspace.is_none());
        assert!(input.version.is_none());
    }

    #[test]
    fn ignores_unknown_top_level_fields() {
        let raw = r#"{"model":{"display_name":"Haiku"},"future_field":42}"#;
        let input: Input = serde_json::from_str(raw).expect("unknown fields ignored");
        assert_eq!(
            input.model.and_then(|m| m.display_name).as_deref(),
            Some("Haiku")
        );
    }
}
