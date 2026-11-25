use std::fs;
use std::path::Path;

use super::package::{reject_reparse, storage};
use crate::domain::application_update::ApplicationUpdateError;

pub(super) fn retire_superseded(
    root: &Path,
    current: &Path,
    portable: bool,
) -> Result<(), ApplicationUpdateError> {
    reject_reparse(root)?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(storage(error)),
    };
    for entry in entries.take(64).flatten() {
        let path = entry.path();
        if portable {
            retire_file(&path, current, true);
        } else if entry.file_name().to_str().is_some_and(is_digest)
            && reject_reparse(&path).is_ok()
            && let Ok(files) = fs::read_dir(&path)
        {
            for file in files.take(8).flatten() {
                retire_file(&file.path(), current, false);
            }
            let _ = fs::remove_dir(path);
        }
    }
    Ok(())
}

fn retire_file(path: &Path, current: &Path, portable: bool) {
    if path == current || path == current.with_extension("partial") || reject_reparse(path).is_err()
    {
        return;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let stem = name
        .strip_suffix(if portable { ".nupkg" } else { ".msi" })
        .or_else(|| name.strip_suffix(".partial"));
    let version = stem.and_then(|stem| {
        if portable {
            stem.strip_prefix("CedarflakeAme-")?.strip_suffix("-full")
        } else {
            let stem = stem.strip_prefix("Cedarflake-Ame-v")?;
            stem.strip_suffix("-windows-x64")
                .or_else(|| stem.strip_suffix("-windows-arm64"))
        }
    });
    if version.is_some_and(|version| semver::Version::parse(version).is_ok()) {
        // A prepared handoff holds a no-delete lease; it remains responsible for its package.
        let _ = fs::remove_file(path);
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retirement_preserves_active_resume_bytes_and_unowned_files() {
        let root = tempfile::tempdir().unwrap();
        let active = root.path().join("CedarflakeAme-1.2.4-full.nupkg");
        let partial = active.with_extension("partial");
        let old = root.path().join("CedarflakeAme-1.2.3-full.nupkg");
        let unrelated = root.path().join("notes.nupkg");
        for path in [&active, &partial, &old, &unrelated] {
            fs::write(path, b"fixture").unwrap();
        }
        retire_superseded(root.path(), &active, true).unwrap();
        assert!(!old.exists());
        assert!(active.exists() && partial.exists() && unrelated.exists());
    }
}
