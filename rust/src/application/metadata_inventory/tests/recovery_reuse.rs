use std::fs::{File, FileTimes};

use super::*;
use crate::adapters::GuardedExactContentReader;
use crate::domain::exact_duplicates::ExactAnalysisRun;
use crate::domain::{ExactAnalysisPhase, ExactAnalysisProgress};
use crate::ports::exact_duplicates::{
    ExactContentReader, ExactDuplicateRepository, FingerprintReadLease,
};

#[test]
fn journal_trim_preserves_unchanged_results_and_reconciles_ordinary_offline_changes() {
    let mut fixture = InventoryFixture::new(&[
        "keep.png",
        "keep-copy.png",
        "modify.png",
        "preserve.bmp",
        "preserve-copy.bmp",
        "atomic.png",
        "delete.png",
        "rename.png",
        "album/move.png",
    ]);
    fingerprint_baseline(&mut fixture);
    let keep = fixture.location("keep.png").expect("original keep");
    let keep_copy = fixture.location("keep-copy.png").expect("original copy");
    let renamed = fixture.location("rename.png").expect("original rename");
    let moved = fixture.location("album/move.png").expect("original move");
    let preserved_path = fixture.source.path().join("preserve.bmp");
    let preserved_metadata = fs::metadata(&preserved_path).expect("original BMP metadata");
    write_png(
        &fixture.source.path().join("modify.png"),
        4,
        3,
        [90, 80, 70],
    );
    write_png(&preserved_path, 2, 2, [90, 80, 70]);
    File::options()
        .write(true)
        .open(&preserved_path)
        .expect("owned BMP")
        .set_times(FileTimes::new().set_modified(preserved_metadata.modified().expect("mtime")))
        .expect("preserve modification time");
    assert_eq!(
        fs::metadata(&preserved_path).expect("rewritten BMP").len(),
        preserved_metadata.len()
    );
    write_png(
        &fixture.source.path().join("replacement.png"),
        5,
        2,
        [30, 20, 10],
    );
    fs::rename(
        fixture.source.path().join("replacement.png"),
        fixture.source.path().join("atomic.png"),
    )
    .expect("replace owned image");
    fs::remove_file(fixture.source.path().join("delete.png")).expect("delete owned image");
    fs::rename(
        fixture.source.path().join("rename.png"),
        fixture.source.path().join("renamed.png"),
    )
    .expect("rename owned image");
    fs::rename(
        fixture.source.path().join("album"),
        fixture.source.path().join("moved-album"),
    )
    .expect("move owned directory");
    write_png(&fixture.source.path().join("新增.png"), 3, 4, [20, 40, 60]);

    let lease = enqueue_authorized_recovery_control_with_reason(
        &mut fixture,
        "trim-reuse",
        2_000,
        LibraryRecoveryAuthorityReason::JournalTrim,
    );
    finish_recovery(&mut fixture, lease, "trim-reuse");

    assert_eq!(fixture.location("keep.png"), Some(keep));
    assert_eq!(fixture.location("keep-copy.png"), Some(keep_copy));
    assert!(fixture.location("delete.png").is_none());
    assert!(fixture.location("rename.png").is_none());
    assert!(fixture.location("album/move.png").is_none());
    assert_eq!(
        fixture
            .location("renamed.png")
            .expect("renamed image")
            .asset_id,
        renamed.asset_id
    );
    assert_eq!(
        fixture
            .location("moved-album/move.png")
            .expect("moved image")
            .asset_id,
        moved.asset_id
    );
    assert_eq!(
        fixture
            .location("modify.png")
            .map(|image| (image.width, image.height)),
        Some((4, 3))
    );
    assert_eq!(
        fixture
            .location("atomic.png")
            .map(|image| (image.width, image.height)),
        Some((5, 2))
    );
    assert!(fixture.location("新增.png").is_some());
    let pending = fixture
        .catalog
        .load_fingerprint_candidates("")
        .expect("remaining content work");
    assert!(pending.iter().all(|candidate| !matches!(
        candidate.location.relative_path.as_str(),
        "keep.png" | "keep-copy.png"
    )));
    assert!(
        pending
            .iter()
            .any(|candidate| candidate.location.relative_path == "preserve.bmp")
    );
}

fn fingerprint_baseline(fixture: &mut InventoryFixture) {
    let cancelled = AtomicBool::new(false);
    fixture
        .catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "before-gap".into(),
            started_unix_ms: 1_000,
        })
        .expect("begin baseline fingerprints");
    let mut generations = Vec::new();
    for candidate in fixture
        .catalog
        .load_fingerprint_candidates("")
        .expect("baseline candidates")
    {
        let lease = GuardedExactContentReader
            .read_fingerprint(&candidate, &cancelled)
            .expect("read owned source")
            .expect("available owned image");
        assert!(
            fixture
                .catalog
                .publish_fingerprint("before-gap", &candidate, lease.fingerprint(), &cancelled)
                .expect("publish baseline fingerprint")
        );
        generations.push(candidate.location.source_generation);
    }
    fixture
        .catalog
        .publish_exact_analysis_batch("before-gap", &generations)
        .expect("publish baseline group");
    let mut progress = ExactAnalysisProgress::running("before-gap".into());
    progress.phase = ExactAnalysisPhase::Completed;
    fixture
        .catalog
        .finish_exact_analysis(&progress)
        .expect("finish baseline fingerprints");
    assert!(
        fixture
            .catalog
            .load_fingerprint_candidates("")
            .expect("current baseline")
            .is_empty()
    );
}

fn finish_recovery(fixture: &mut InventoryFixture, first: LeasedLibraryChange, run_id: &str) {
    let cancelled = AtomicBool::new(false);
    let change_id = first.change.id;
    let mut lease = Some(first);
    let mut retained = None;
    for step in 0..64 {
        let now = 2_000 + step;
        let root = fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("recovery root")
            .expect("published root");
        if let Some(current) = lease.take() {
            let page = process_leased_metadata_inventory_change_with_retained_source(
                &mut fixture.catalog,
                &root,
                &current,
                MetadataInventoryRecoveryExecution::without_progress(
                    now,
                    4_095,
                    queue_policy(),
                    &cancelled,
                )
                .with_recovery_time_slice(),
                retained.take(),
            )
            .expect("recover from journal trim");
            retained = page.retained_source;
            assert_eq!(page.report.incremental.retried_count, 0);
        }
        let drained = process_ready_metadata_inventory_recovery_candidates_cancellable(
            &mut fixture.catalog,
            &fixture.root_id,
            root.root_generation,
            now,
            queue_policy(),
            &cancelled,
        )
        .expect("publish changed candidates");
        assert_eq!(drained.retried_count, 0);
        lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                root.root_generation,
                now + 1,
                queue_policy(),
            )
            .expect("lease next recovery slice");
        if lease.is_none() {
            assert_eq!(
                fixture
                    .catalog
                    .load_metadata_inventory_run(run_id)
                    .expect("finished run")
                    .expect("finished run")
                    .status,
                MetadataInventoryRunStatus::Completed
            );
            assert!(
                fixture
                    .catalog
                    .load_metadata_inventory_recovery_authority(change_id)
                    .expect("completed authority")
                    .expect("retained authority")
                    .retired_unix_ms
                    .is_some()
            );
            return;
        }
    }
    panic!("recovery did not finish its bounded fixture");
}
