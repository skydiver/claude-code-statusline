//! `$git_branch` / `$git_branch_sep` — current branch via a `git` subprocess.
//!
//! `$git_branch` renders the raw branch name and is empty outside a repository.
//! `$git_branch_sep` wraps the name in the shell script's conditional separator
//! (` | 🌿 <branch>`) and is also empty outside a repository — identical to the
//! shell's `{git_branch_sep}` placeholder, so feature parity needs no
//! render-engine suppression logic.
//!
//! The `Input` parameter is unused today but kept for signature uniformity
//! with every other module, and so future modules can grow to read config
//! without breaking the `dispatch` match arms.

use std::process::Command;

use crate::input::Input;

const SEP_PREFIX: &str = " | 🌿 ";

fn get_branch() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn render_name_str(branch: Option<&str>) -> String {
    branch.unwrap_or("").to_string()
}

fn render_sep_str(branch: Option<&str>) -> String {
    match branch {
        Some(name) if !name.is_empty() => format!("{SEP_PREFIX}{name}"),
        _ => String::new(),
    }
}

pub fn render_name(_input: &Input) -> String {
    render_name_str(get_branch().as_deref())
}

pub fn render_sep(_input: &Input) -> String {
    render_sep_str(get_branch().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_name_returns_branch_when_present() {
        assert_eq!(render_name_str(Some("master")), "master");
    }

    #[test]
    fn render_name_returns_empty_when_absent() {
        assert_eq!(render_name_str(None), "");
    }

    #[test]
    fn render_name_returns_empty_when_branch_is_empty() {
        assert_eq!(render_name_str(Some("")), "");
    }

    #[test]
    fn render_sep_wraps_branch_with_prefix() {
        assert_eq!(render_sep_str(Some("feature/x")), " | 🌿 feature/x");
    }

    #[test]
    fn render_sep_empty_outside_repo() {
        assert_eq!(render_sep_str(None), "");
    }

    #[test]
    fn render_sep_empty_when_branch_is_empty_string() {
        assert_eq!(render_sep_str(Some("")), "");
    }
}
