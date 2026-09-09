use super::*;
use crate::domain::LibraryChangeQueuePolicy;

pub(super) struct Target<'a> {
    pub control_id: LibraryChangeId,
    pub root_id: &'a str,
    pub source_path: &'a std::path::Path,
    pub entry_count: usize,
    pub live_root_id: &'a str,
    pub live_count: usize,
    pub deadline: Instant,
}

#[derive(Debug, PartialEq, Eq)]
struct Evidence {
    staged: i64,
    candidates: i64,
    owners: i64,
    completed_owners: i64,
    run: String,
    control: String,
    authority_retired: bool,
    baseline: String,
    checkpoint: String,
    root: String,
}

#[derive(Default)]
struct ObservationTiming {
    polls: u64,
    poll_wall: Duration,
    evidence_wall: Duration,
}

impl ObservationTiming {
    fn report(&self, elapsed: Duration) {
        eprintln!(
            "controlled recovery observation elapsed_ms={} polls={} poll_ms={} evidence_ms={}",
            elapsed.as_millis(),
            self.polls,
            self.poll_wall.as_millis(),
            self.evidence_wall.as_millis(),
        );
    }
}

impl Evidence {
    fn completed(total: usize) -> Self {
        let total = i64::try_from(total).expect("fixture total");
        Self {
            staged: total,
            candidates: total,
            owners: total,
            completed_owners: total,
            run: "completed".to_owned(),
            control: "completed".to_owned(),
            authority_retired: true,
            baseline: "completed".to_owned(),
            checkpoint: "current".to_owned(),
            root: "current".to_owned(),
        }
    }
}

fn evidence(connection: &rusqlite::Connection, target: &Target<'_>) -> Evidence {
    connection
        .query_row(
            "SELECT run.staged_entry_count, run.candidate_count,
            (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
             WHERE run_id = authority.run_id),
            (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners AS owner
             JOIN library_change_queue AS owned ON owned.id = owner.change_id
             WHERE owner.run_id = authority.run_id AND owned.status = 'completed'),
            run.status, control.status, authority.retired_unix_ms IS NOT NULL,
            baseline.phase, checkpoint.continuity_state, root.continuity_state
         FROM library_recovery_authorities AS authority
         JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
         JOIN library_change_queue AS control ON control.id = authority.change_id
         JOIN library_persistent_journal_baselines AS baseline
           ON baseline.change_id = authority.change_id
         JOIN library_persistent_journal_checkpoints AS checkpoint
           ON checkpoint.root_id = authority.root_id
          AND checkpoint.root_generation = authority.root_generation
         JOIN library_persistent_journal_root_state AS root
           ON root.root_id = authority.root_id
          AND root.root_generation = authority.root_generation
         WHERE authority.change_id = ?1 AND authority.root_id = ?2",
            rusqlite::params![
                i64::try_from(target.control_id.value()).expect("control ID"),
                target.root_id,
            ],
            |row| {
                Ok(Evidence {
                    staged: row.get(0)?,
                    candidates: row.get(1)?,
                    owners: row.get(2)?,
                    completed_owners: row.get(3)?,
                    run: row.get(4)?,
                    control: row.get(5)?,
                    authority_retired: row.get(6)?,
                    baseline: row.get(7)?,
                    checkpoint: row.get(8)?,
                    root: row.get(9)?,
                })
            },
        )
        .expect("exact recovery-control publication evidence")
}

pub(super) fn drive_to_publication(
    production: &mut ProductionSynchronization,
    storage: &crate::application::storage::StoragePaths,
    connection: &rusqlite::Connection,
    target: &Target<'_>,
    poll: &mut impl FnMut(
        &mut ProductionSynchronization,
        &crate::application::storage::StoragePaths,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError>,
) {
    let started = Instant::now();
    let mut timing = ObservationTiming::default();
    let mut next_report = Duration::from_secs(10);
    loop {
        if Instant::now() >= target.deadline {
            timing.report(started.elapsed());
        }
        assert!(
            Instant::now() < target.deadline,
            "full recovery exceeded its creation-to-reopen budget: {:?}; queue={:?}",
            evidence(connection, target),
            priority_queue_progress(connection),
        );
        let poll_started = Instant::now();
        let snapshot =
            poll(production, storage).expect("continue the original mixed-load recovery");
        timing.poll_wall += poll_started.elapsed();
        timing.polls += 1;
        let evidence_started = Instant::now();
        let current = evidence(connection, target);
        timing.evidence_wall += evidence_started.elapsed();
        if started.elapsed() >= next_report {
            timing.report(started.elapsed());
            eprintln!("controlled recovery progress evidence={current:?}");
            next_report = started.elapsed() + Duration::from_secs(10);
        }
        let synchronized = snapshot.roots.iter().any(|root| {
            root.root_id == target.root_id
                && root.freshness == crate::domain::CatalogFreshnessState::Synchronized
        });
        if current == Evidence::completed(target.entry_count) && synchronized {
            timing.report(started.elapsed());
            eprintln!(
                "controlled mixed-load full recovery tail_ms={} entries={} evidence={current:?}",
                started.elapsed().as_millis(),
                target.entry_count,
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(super) fn verify_reopened(
    storage: &crate::application::storage::StoragePaths,
    connection: &rusqlite::Connection,
    target: &Target<'_>,
) {
    let validations = crate::adapters::full_schema_validation_count(&storage.catalog_path);
    let catalog = SqliteCatalog::open(storage.catalog_path.clone())
        .expect("FULL reopen after complete mixed-load publication and owned stop");
    assert_eq!(
        crate::adapters::full_schema_validation_count(&storage.catalog_path),
        validations + 1,
        "reopen must validate the whole catalog, not reuse the process session",
    );
    assert_eq!(
        evidence(connection, target),
        Evidence::completed(target.entry_count)
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            target.root_id,
            LibraryRootGeneration::initial(),
            now_unix_ms().expect("clock"),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("reopened recovery queue");
    assert_eq!(
        metrics.pending_count + metrics.leased_count + metrics.retry_wait_count,
        0
    );
    assert_eq!(metrics.explicit_recovery_required_count, 0);
    for start in (0..target.entry_count).step_by(256) {
        assert!(
            Instant::now() < target.deadline,
            "reopen shares the completion budget"
        );
        let paths: Vec<_> = (start..(start + 256).min(target.entry_count))
            .map(|index| format!("recovery-{index:05}.jpg"))
            .collect();
        let locations = catalog
            .load_incremental_locations_by_relative_paths(target.root_id, &paths)
            .expect("bounded published recovery window");
        let terminal = catalog
            .load_terminal_media_evidence_by_relative_paths(target.root_id, &paths)
            .expect("bounded persisted negative-media evidence");
        assert_eq!(locations.len(), paths.len());
        assert_eq!(terminal.len(), paths.len());
        for path in paths {
            let location = locations
                .iter()
                .find(|location| location.relative_path == path)
                .expect("every candidate has a published result");
            assert!(matches!(
                location.preview_status,
                crate::domain::PreviewStatus::Failed
            ));
            assert!(location.preview_issue_code.is_some());
            assert!(location.source_revision.is_some());
            assert!(location.source_generation > 0);
            let observation = terminal
                .iter()
                .find(|observation| observation.relative_path == path)
                .expect("each terminal result retains its matching source evidence");
            assert_eq!(observation.source_revision, location.source_revision);
            assert_eq!(observation.source_generation, location.source_generation);
            assert_eq!(
                Some(&observation.issue.code),
                location.preview_issue_code.as_ref()
            );
            assert_eq!(
                std::fs::read(target.source_path.join(path)).expect("controlled source bytes"),
                b"metadata-only recovery fixture",
            );
        }
    }
    for index in 0..target.live_count {
        catalog
            .load_incremental_location_by_relative_path(
                target.live_root_id,
                &format!("live-{index:02}.png"),
            )
            .expect("retained P0 query")
            .expect("P2 completion preserves every visible P0 result");
    }
    let (active_spools, opening, closing): (i64, String, String) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spools
                      WHERE run_id = authority.run_id AND state != 'retired'),
                    baseline.opening_next_usn, baseline.closing_next_usn
             FROM library_recovery_authorities AS authority
             JOIN library_persistent_journal_baselines AS baseline
               ON baseline.change_id = authority.change_id
             WHERE authority.change_id = ?1",
            [i64::try_from(target.control_id.value()).expect("control ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retirement and closing-window evidence");
    assert_eq!(active_spools, 0, "retired storage cannot remain executable");
    assert!(
        closing.parse::<u64>().expect("closing USN") > opening.parse::<u64>().expect("opening USN"),
        "same-volume P1 changes require a nonempty P2 closing window",
    );
    assert!(
        Instant::now() < target.deadline,
        "complete creation-to-reopen budget"
    );
}
