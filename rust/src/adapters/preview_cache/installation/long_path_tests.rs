use std::os::windows::fs::OpenOptionsExt;

use image::GenericImageView;
use tempfile::tempdir;
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

use super::*;
use crate::adapters::preview_cache::{LocalPreviewStore, PREVIEW_ALGORITHM, cache_inventory};
use crate::domain::DiscoveredFile;
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};
use crate::ports::PreviewStore;

fn long_cache_root(storage: &Path, minimum_target_units: usize) -> PathBuf {
    let artifact_name_units = PREVIEW_ALGORITHM.encode_utf16().count() + 1 + 64 + 4;
    let mut root = storage.join("预览");
    assert!(!root.to_string_lossy().starts_with(r"\\?\"));
    loop {
        let target_units = root.as_os_str().encode_wide().count() + 1 + artifact_name_units;
        if target_units >= minimum_target_units {
            return root;
        }
        root = root.join("p".repeat((minimum_target_units - target_units - 1).clamp(1, 48)));
    }
}

fn source_fixture(storage: &Path) -> (DiscoveredFile, Vec<u8>) {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Png, 64, 48).expect("source fixture");
    let source = storage.join("source.png");
    fs::write(&source, &bytes).expect("write source fixture");
    let file = DiscoveredFile {
        source_root_path: fs::canonicalize(storage)
            .expect("source root")
            .to_string_lossy()
            .into_owned(),
        absolute_path: source.to_string_lossy().into_owned(),
        relative_path: "source.png".to_owned(),
        file_size: bytes.len() as u64,
        created_unix_ms: None,
        modified_unix_ms: 0,
        file_identity: None,
        source_revision: None,
        source_generation: 1,
        issues: Vec::new(),
    };
    (file, bytes)
}

#[test]
fn long_paths_support_cold_install_replacement_rollback_and_warm_reuse() {
    for minimum_target_units in [257, 320] {
        let directory = tempdir().expect("isolated cache fixture");
        let root = long_cache_root(directory.path(), minimum_target_units);
        let (file, source_bytes) = source_fixture(directory.path());
        let source = File::open(&file.absolute_path).expect("source handle");
        let source_modified = source
            .metadata()
            .expect("source metadata")
            .modified()
            .unwrap();
        let store = LocalPreviewStore::new(root.clone(), 1024 * 1024).expect("long cache");
        let cold = store
            .materialize(&file, &source, 128, 64, 48, false)
            .expect("cold staging");
        let staged_path = PathBuf::from(cold.staged_path.as_ref().expect("staged path"));
        assert!(staged_path.as_os_str().encode_wide().count() > 260);
        assert!(staged_path.is_file());
        let artifact = store.commit(cold).expect("native long-path cold install");
        let target = PathBuf::from(&artifact.path);
        assert!(target.as_os_str().encode_wide().count() >= minimum_target_units);
        assert!(!staged_path.exists());
        let preview_bytes = fs::read(&target).expect("installed bytes");
        assert_eq!(
            image::load_from_memory(&preview_bytes)
                .expect("valid preview")
                .dimensions(),
            (128, 96)
        );
        assert_eq!(store.used_bytes(), artifact.byte_size);

        let replacement = store
            .materialize(&file, &source, 128, 64, 48, true)
            .expect("replacement staging");
        assert!(replacement.replace_existing);
        REPLACEMENT_FAULTS
            .lock()
            .expect("replacement faults")
            .insert(
                target.clone(),
                ReplacementFault {
                    error: 1177,
                    ..ReplacementFault::default()
                },
            );
        let issue = store
            .commit(replacement)
            .expect_err("restore after partial replacement");
        assert_eq!(issue.code, "preview_publish_failed");
        assert_eq!(
            issue.message,
            io::Error::from_raw_os_error(1177).to_string()
        );
        assert_eq!(fs::read(&target).expect("restored bytes"), preview_bytes);
        assert_eq!(store.used_bytes(), artifact.byte_size);
        assert_eq!(fs::read_dir(&root).expect("settled rollback").count(), 1);

        let replacement = store
            .materialize(&file, &source, 128, 64, 48, true)
            .expect("replacement retry");
        assert!(replacement.staged_path.is_some());
        let replaced = store
            .commit(replacement)
            .expect("native long-path replacement");
        assert_eq!(replaced.path, artifact.path);
        assert_eq!(fs::read(&target).expect("replacement bytes"), preview_bytes);
        assert_eq!(store.used_bytes(), replaced.byte_size);
        let modified = fs::metadata(&target)
            .expect("preview metadata")
            .modified()
            .unwrap();
        let warm = store
            .materialize(&file, &source, 128, 64, 48, false)
            .expect("warm cache");
        assert!(warm.staged_path.is_none());
        assert_eq!(warm.reserved_bytes, 0);
        store.commit(warm).expect("warm commit");
        assert_eq!(
            fs::metadata(&target)
                .expect("warm metadata")
                .modified()
                .unwrap(),
            modified
        );
        assert_eq!(
            cache_inventory(&root).expect("inventory").0,
            replaced.byte_size
        );
        assert_eq!(fs::read_dir(&root).expect("settled cache").count(), 1);
        assert_eq!(
            fs::read(&file.absolute_path).expect("source after requests"),
            source_bytes
        );
        assert_eq!(
            source
                .metadata()
                .expect("source after metadata")
                .modified()
                .unwrap(),
            source_modified
        );
    }
}

#[test]
fn long_path_install_and_restore_do_not_overwrite_a_concurrent_target() {
    let directory = tempdir().expect("isolated cache");
    let root = long_cache_root(directory.path(), 320);
    fs::create_dir_all(&root).expect("long directory");
    let target = root.join(format!("{PREVIEW_ALGORITHM}-{}.jpg", "a".repeat(64)));
    let staged = target.with_extension("1-1.tmp");
    fs::write(&target, b"concurrent target").expect("target");
    fs::write(&staged, b"owned bytes").expect("staged");
    assert!(install_new(&staged, &target).is_err());
    assert!(
        NativeInstallation::for_target(&target)
            .restore(&staged, &target)
            .is_err()
    );
    assert_eq!(
        fs::read(&target).expect("retained target"),
        b"concurrent target"
    );
    assert_eq!(
        fs::read(&staged).expect("retained owned bytes"),
        b"owned bytes"
    );
}

#[test]
fn missing_target_probe_preserves_the_native_sharing_error() {
    let directory = tempdir().expect("isolated cache");
    let (file, _) = source_fixture(directory.path());
    let source = File::open(&file.absolute_path).expect("source");
    let store =
        LocalPreviewStore::new(directory.path().join("previews"), 1024 * 1024).expect("cache");
    let staged = store
        .materialize(&file, &source, 128, 64, 48, false)
        .expect("staging");
    let reserved_bytes = staged.reserved_bytes;
    let staged_path = PathBuf::from(staged.staged_path.as_ref().expect("staged path"));
    let target = PathBuf::from(&staged.artifact.path);
    let locked = File::options()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&staged_path)
        .expect("deny rename and deletion");
    let native_error = install_new(&staged_path, &target)
        .err()
        .expect("native sharing error");
    assert_eq!(
        native_error.message,
        io::Error::from_raw_os_error(32).to_string()
    );
    let issue = store.commit(staged).expect_err("locked staging");
    assert_eq!(issue.code, native_error.code);
    assert_eq!(issue.message, native_error.message);
    assert!(!target.exists());
    assert!(staged_path.exists());
    assert_eq!(store.used_bytes(), reserved_bytes);
    drop(locked);
    store
        .remove_staged_file(&staged_path, reserved_bytes)
        .expect("release owned staging after lock closes");
    assert_eq!(store.used_bytes(), 0);
}

#[test]
fn native_path_encoding_resolves_relative_parents_and_rejects_nul() {
    let expected = fs::canonicalize(".")
        .expect("current directory")
        .join("new-preview.jpg");
    assert_eq!(
        wide_path(Path::new("new-preview.jpg")).expect("relative target"),
        expected
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        wide_path(&expected).expect("already verbatim path"),
        expected
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        wide_path(Path::new("bad\0name.jpg"))
            .expect_err("embedded NUL")
            .kind(),
        io::ErrorKind::InvalidInput
    );
}
