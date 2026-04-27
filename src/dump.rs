//! Optional input snapshot dump.
//!
//! When `$CCLINE_INPUT_DUMP` is set to a non-empty path, ccline writes the
//! parsed stdin payload to that path on every invocation. The file is meant
//! to be consumed by other tools (menubar apps, dashboards) that want the
//! current Claude Code session state without re-implementing a statusline
//! reader.
//!
//! Writes are atomic: the JSON goes to `<path>.tmp` first and is then
//! renamed onto the final path. Readers either see the previous snapshot
//! or the new one — never a half-written file.
//!
//! Empty `CCLINE_INPUT_DUMP` is treated as unset, mirroring the
//! `CCLINE_CONFIG` / `XDG_CONFIG_HOME` convention. Failures are logged to
//! stderr and never bubble up — the statusline must keep rendering.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

const ENV_VAR: &str = "CCLINE_INPUT_DUMP";

pub fn try_dump(value: &Value) {
    let Some(path) = dump_path() else { return };
    if let Err(e) = write_dump(&path, value) {
        eprintln!(
            "ccline: failed to write {ENV_VAR} at {}: {e}",
            path.display()
        );
    }
}

fn dump_path() -> Option<PathBuf> {
    let raw = env::var_os(ENV_VAR);
    resolve_dump_path(raw.as_deref())
}

fn resolve_dump_path(value: Option<&OsStr>) -> Option<PathBuf> {
    value.filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn write_dump(path: &Path, value: &Value) -> io::Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let tmp = tmp_path(path);
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

/// Build the temp sibling path by appending `.tmp` to the *full* filename
/// rather than replacing the extension — `Path::with_extension` would turn
/// `snapshot.json` into `snapshot.tmp`, which is more surprising than
/// `snapshot.json.tmp` for anyone watching the directory.
fn tmp_path(path: &Path) -> PathBuf {
    let mut s = OsString::from(path);
    s.push(".tmp");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Per-process counter so concurrent tests don't collide on a path.
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = env::temp_dir().join(format!("ccline-dump-test-{pid}-{nanos}-{n}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn resolve_dump_path_some_when_non_empty() {
        let p = resolve_dump_path(Some(OsStr::new("/tmp/snapshot.json"))).expect("some");
        assert_eq!(p, PathBuf::from("/tmp/snapshot.json"));
    }

    #[test]
    fn resolve_dump_path_none_when_unset() {
        assert!(resolve_dump_path(None).is_none());
    }

    #[test]
    fn resolve_dump_path_none_when_empty() {
        // Match the empty-string-as-unset convention CCLINE_CONFIG uses.
        assert!(resolve_dump_path(Some(OsStr::new(""))).is_none());
    }

    #[test]
    fn tmp_path_appends_dot_tmp() {
        assert_eq!(
            tmp_path(Path::new("/var/run/snapshot.json")),
            PathBuf::from("/var/run/snapshot.json.tmp")
        );
        // Also works for paths without an extension.
        assert_eq!(
            tmp_path(Path::new("/var/run/snapshot")),
            PathBuf::from("/var/run/snapshot.tmp")
        );
    }

    #[test]
    fn write_dump_writes_pretty_json_atomically() {
        let dir = unique_dir();
        let path = dir.join("snapshot.json");

        let value: Value = serde_json::from_str(r#"{"model":{"display_name":"Haiku"}}"#).unwrap();
        write_dump(&path, &value).expect("write_dump succeeds");

        let written = fs::read_to_string(&path).expect("read back");
        // Pretty-printing inserts whitespace; round-tripping through Value
        // confirms it's still valid JSON with the same shape.
        let reparsed: Value = serde_json::from_str(&written).expect("reparses");
        assert_eq!(reparsed, value);
        assert!(
            written.contains('\n'),
            "expected pretty-printed output with newlines, got: {written:?}"
        );

        // No leftover temp sibling after a successful rename.
        assert!(!tmp_path(&path).exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_dump_overwrites_existing_file() {
        let dir = unique_dir();
        let path = dir.join("snapshot.json");

        let v1: Value = serde_json::from_str(r#"{"version":"1"}"#).unwrap();
        let v2: Value = serde_json::from_str(r#"{"version":"2"}"#).unwrap();

        write_dump(&path, &v1).unwrap();
        write_dump(&path, &v2).unwrap();

        let reparsed: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(reparsed, v2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_dump_errors_when_parent_missing() {
        // No directory creation on our side — surfacing the I/O error is
        // the right behavior. try_dump() will log it and rendering keeps
        // going.
        let path = env::temp_dir().join("ccline-dump-nonexistent-dir-xyz/snapshot.json");
        let value: Value = serde_json::from_str("{}").unwrap();
        assert!(write_dump(&path, &value).is_err());
    }

    #[test]
    fn write_dump_preserves_unknown_fields() {
        // The whole point of going through serde_json::Value rather than the
        // typed Input struct: forward-compat with new Claude Code fields.
        let dir = unique_dir();
        let path = dir.join("snapshot.json");

        let value: Value = serde_json::from_str(
            r#"{"model":{"display_name":"Opus"},"future_field":{"nested":42}}"#,
        )
        .unwrap();
        write_dump(&path, &value).unwrap();

        let reparsed: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(reparsed["future_field"]["nested"], 42);

        fs::remove_dir_all(&dir).ok();
    }
}
