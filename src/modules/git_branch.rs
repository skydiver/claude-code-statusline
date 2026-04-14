//! `$git_branch` / `$git_branch_sep` — current branch via direct `.git/HEAD` read.
//!
//! `$git_branch` renders the raw branch name and is empty outside a repository
//! or on a detached HEAD. `$git_branch_sep` wraps the name in the shell
//! script's conditional separator (` | 🌿 <branch>`) and is also empty in
//! those cases.
//!
//! We walk parent directories looking for a `.git` entry: a directory means a
//! plain repo, a file means a worktree or submodule whose `gitdir: <path>`
//! line points at the real git directory. Once we have the git dir we read
//! `HEAD` and strip `ref: refs/heads/` — a raw SHA means detached HEAD and we
//! return `None`. Zero subprocesses, zero dependencies, cross-platform.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::input::Input;

const SEP_PREFIX: &str = " | 🌿 ";

fn get_branch() -> Option<String> {
    let cwd = env::current_dir().ok()?;
    let git_dir = find_git_dir(&cwd)?;
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    parse_head_ref(&head)
}

fn find_git_dir(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let dot_git = ancestor.join(".git");
        let Ok(md) = fs::metadata(&dot_git) else {
            continue;
        };
        if md.is_dir() {
            return Some(dot_git);
        }
        if md.is_file() {
            let contents = fs::read_to_string(&dot_git).ok()?;
            let pointer = parse_gitdir_pointer(&contents)?;
            let git_dir = PathBuf::from(pointer);
            return Some(if git_dir.is_absolute() {
                git_dir
            } else {
                ancestor.join(git_dir)
            });
        }
    }
    None
}

fn parse_head_ref(contents: &str) -> Option<String> {
    let rest = contents.trim().strip_prefix("ref: refs/heads/")?;
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn parse_gitdir_pointer(contents: &str) -> Option<&str> {
    contents
        .lines()
        .find_map(|l| l.strip_prefix("gitdir: "))
        .map(str::trim)
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
    fn parse_head_ref_on_branch() {
        assert_eq!(
            parse_head_ref("ref: refs/heads/master\n"),
            Some("master".into())
        );
    }

    #[test]
    fn parse_head_ref_on_branch_with_slash() {
        assert_eq!(
            parse_head_ref("ref: refs/heads/feature/x\n"),
            Some("feature/x".into())
        );
    }

    #[test]
    fn parse_head_ref_detached_returns_none() {
        // Detached HEAD: HEAD contains a raw SHA instead of a ref.
        assert_eq!(
            parse_head_ref("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0\n"),
            None
        );
    }

    #[test]
    fn parse_head_ref_empty_returns_none() {
        assert_eq!(parse_head_ref(""), None);
    }

    #[test]
    fn parse_head_ref_non_heads_returns_none() {
        // Some tools create HEADs under refs/remotes or refs/tags. We only
        // surface local branches, matching `git rev-parse --abbrev-ref HEAD`.
        assert_eq!(parse_head_ref("ref: refs/remotes/origin/master\n"), None);
    }

    #[test]
    fn parse_head_ref_no_trailing_newline() {
        assert_eq!(parse_head_ref("ref: refs/heads/main"), Some("main".into()));
    }

    #[test]
    fn parse_gitdir_pointer_reads_relative_path() {
        assert_eq!(
            parse_gitdir_pointer("gitdir: ../.git/worktrees/foo\n"),
            Some("../.git/worktrees/foo")
        );
    }

    #[test]
    fn parse_gitdir_pointer_reads_absolute_path() {
        assert_eq!(
            parse_gitdir_pointer("gitdir: /abs/repo/.git/modules/sub\n"),
            Some("/abs/repo/.git/modules/sub")
        );
    }

    #[test]
    fn parse_gitdir_pointer_missing_returns_none() {
        assert_eq!(parse_gitdir_pointer(""), None);
        assert_eq!(parse_gitdir_pointer("something else\n"), None);
    }

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
