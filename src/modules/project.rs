//! `$project` — basename of `workspace.project_dir`.

use std::path::Path;

use crate::input::Input;

pub fn render(input: &Input) -> String {
    input
        .workspace
        .as_ref()
        .and_then(|w| w.project_dir.as_deref())
        .and_then(|path| Path::new(path).file_name())
        .map(|os| os.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Workspace;

    fn with_dir(dir: &str) -> Input {
        Input {
            workspace: Some(Workspace {
                project_dir: Some(dir.into()),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn renders_basename() {
        let input = with_dir("/Users/example/projects/claude-code-statusline");
        assert_eq!(render(&input), "claude-code-statusline");
    }

    #[test]
    fn handles_trailing_slash() {
        // Rust's Path::file_name strips trailing separators before returning.
        let input = with_dir("/tmp/foo/");
        assert_eq!(render(&input), "foo");
    }

    #[test]
    fn renders_empty_when_workspace_missing() {
        let input = Input::default();
        assert_eq!(render(&input), "");
    }

    #[test]
    fn renders_empty_when_project_dir_missing() {
        let input = Input {
            workspace: Some(Workspace { project_dir: None }),
            ..Default::default()
        };
        assert_eq!(render(&input), "");
    }
}
