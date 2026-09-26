use std::cell::RefCell;
use std::sync::Arc;

use crate::adapters::local_files::{FileDiscovery, FileVisitOutcome};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};

use super::*;

type BeforeWrite = Option<(PathBuf, Box<dyn FnOnce(&Path)>)>;

thread_local! {
    static BEFORE_WRITE: RefCell<BeforeWrite> = RefCell::new(None);
}

pub(super) fn before_write(artifact_path: &Path, temporary_path: &Path) {
    let hook = BEFORE_WRITE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|(expected, _)| expected == artifact_path)
        {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook(temporary_path);
    }
}

#[test]
fn staging_never_truncates_or_deletes_a_preexisting_source_hardlink() {
    let directory = tempfile::tempdir().expect("isolated staging fixture");
    let source_root = directory.path().join("source");
    let other_root = directory.path().join("other-source");
    fs::create_dir(&source_root).expect("preview input directory");
    fs::create_dir(&other_root).expect("external source directory");
    let source_path = source_root.join("input.png");
    let other_source = other_root.join("original.jpg");
    let source_bytes =
        encode_rgb_quadrants(MediaFixtureFormat::Png, 32, 16).expect("generated PNG input");
    let other_bytes =
        encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 16, 32).expect("generated external JPEG");
    fs::write(&source_path, &source_bytes).expect("preview input fixture");
    fs::write(&other_source, &other_bytes).expect("external source fixture");
    let discovery =
        FileDiscovery::new(&source_root.to_string_lossy()).expect("real file discovery");
    let FileVisitOutcome::File(file) = discovery.visit_relative_path("input.png").outcome else {
        panic!("generated input must be admitted as media");
    };
    let cache = directory.path().join("cache");
    let store = LocalPreviewStore::new(cache.clone(), 1024 * 1024).expect("preview store");
    let target = store.artifact_path(&file, 128);
    let collided_path = Arc::new(Mutex::new(None));
    let hook_collision = Arc::clone(&collided_path);
    let hook_source = other_source.clone();
    BEFORE_WRITE.with(|slot| {
        *slot.borrow_mut() = Some((
            target,
            Box::new(move |temporary_path| {
                fs::hard_link(&hook_source, temporary_path)
                    .expect("preexisting leaf aliases a generated external source");
                *hook_collision.lock().expect("observed collision") =
                    Some(temporary_path.to_path_buf());
            }),
        ));
    });
    let source = File::open(&source_path).expect("open preview input");
    let result = store.materialize(&file, &source, 128, 32, 16, false);
    BEFORE_WRITE.with(|slot| slot.borrow_mut().take());
    let collision = collided_path
        .lock()
        .expect("observed collision")
        .clone()
        .expect("the actual selected temporary path must reach the hook");
    let materialization = result.expect("encoding obtains another exclusively owned leaf");
    assert_ne!(materialization.staged_path.as_deref(), collision.to_str());
    store
        .discard_staged(&materialization)
        .expect("discard only an owned new staging file");
    let observed_source = fs::read(&other_source).expect("external source remains addressable");
    eprintln!(
        "staging collision: source_unchanged={}, existing_leaf_survives={}",
        observed_source == other_bytes,
        collision.exists(),
    );
    assert_eq!(
        observed_source, other_bytes,
        "encoding must not truncate a preexisting source alias"
    );
    assert_eq!(
        fs::read(&collision).expect("an unclaimed staging leaf cannot be removed on completion"),
        other_bytes,
    );
    assert_eq!(
        fs::read(&source_path).expect("preview input remains unchanged"),
        source_bytes
    );
    assert_eq!(fs::read_dir(&cache).expect("cache entries").count(), 1);
    assert_eq!(
        store.used_bytes(),
        0,
        "only newly owned staging bytes may enter accounting"
    );
}
