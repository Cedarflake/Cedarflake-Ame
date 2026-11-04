use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::{SimilarAnalysisPhase, SimilarVariant};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::super::organization::tests::fixture;
use super::*;

fn run(id: &str) -> SimilarAnalysisRun {
    SimilarAnalysisRun {
        run_id: id.into(),
        source_run_id: format!("source-{id}"),
        started_unix_ms: 1,
    }
}

fn content(number: u64, bytes: u64) -> SimilarContentKey {
    SimilarContentKey {
        scheme: EXACT_FINGERPRINT_SCHEME.into(),
        digest: format!("{number:064x}"),
        byte_length: bytes,
    }
}

fn scalar(catalog: &SqliteCatalog, sql: &str) -> i64 {
    catalog
        .connection
        .query_row(sql, [], |row| row.get(0))
        .unwrap()
}

fn drain(catalog: &mut SqliteCatalog, run: &SimilarAnalysisRun) {
    for _ in 0..2000 {
        if !catalog
            .match_similarity_window(run, &AtomicBool::new(false))
            .unwrap()
        {
            return;
        }
    }
    panic!("bounded fixture did not retire its continuations");
}

#[test]
fn unequal_file_sizes_are_analyzed_without_a_model_and_cached_by_verified_content() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'b';
         UPDATE asset_locations SET file_size = 22 WHERE location_id = 'c';",
        )
        .unwrap();
    assert!(catalog.load_fingerprint_candidates("").unwrap().is_empty());
    let run = run("different-sizes");
    catalog.begin_similarity(&run).unwrap();
    let sources = catalog.load_similarity_sources("").unwrap();
    assert_eq!(sources.len(), 3);
    for (index, source) in sources.into_iter().enumerate() {
        let content = content(index as u64 + 1, source.candidate.location.file_size);
        let fingerprint = ContentFingerprint {
            digest: content.digest.clone(),
            byte_length: content.byte_length,
        };
        assert!(
            catalog
                .verify_similarity_source(
                    &run,
                    &source.candidate,
                    &fingerprint,
                    &AtomicBool::new(false)
                )
                .unwrap()
        );
        // The third content collides with the reference's coarse hash but only matches
        // the fine hash of a different orientation, whose coarse hash is incompatible.
        let variants = match index {
            0 => vec![
                SimilarVariant {
                    candidate: [255, 0, 0, 0],
                    confirmation: [0x55; 32],
                },
                SimilarVariant {
                    candidate: [65535, 65535, 65535, 65534],
                    confirmation: [0xaa; 32],
                },
            ],
            1 => vec![SimilarVariant {
                candidate: [511, 0, 0, 0],
                confirmation: [0x55; 32],
            }],
            _ => vec![SimilarVariant {
                candidate: [255, 0, 0, 0],
                confirmation: [0xaa; 32],
            }],
        };
        assert!(
            catalog
                .publish_similarity(
                    &run,
                    &source.candidate,
                    &content,
                    &SimilarEvidence::Ready { variants },
                    &AtomicBool::new(false)
                )
                .unwrap()
        );
    }
    assert!(catalog.load_similarity_sources("").unwrap().is_empty());
    drain(&mut catalog, &run);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM similarity_groups"),
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM similarity_members"),
        2
    );
    assert_eq!(
        scalar(&catalog, "SELECT MAX(distance) FROM similarity_members"),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM gallery_content_fingerprints"
        ),
        3
    );
    let mut progress = SimilarAnalysisProgress::running(run.run_id.clone());
    progress.phase = SimilarAnalysisPhase::Completed;
    progress.checked_sources = 3;
    progress.analyzed_contents = 3;
    catalog.finish_similarity(&run, &progress).unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM similarity_dispatch WHERE active_run_id IS NOT NULL"
        ),
        0
    );
}

#[test]
fn dense_postings_resume_after_reopen_and_never_create_transitive_membership() {
    let (directory, mut catalog) = fixture();
    let run = run("dense");
    catalog.begin_similarity(&run).unwrap();
    let transaction = catalog.begin_write().unwrap();
    for number in 1..=140 {
        // B differs from A by 10 bits, C from B by 10, but A/C differ by 20.
        let words = match number {
            2 => [255, 1023, 0, 0],
            3 => [255, 65535, 15, 0],
            _ => [255, 0, 0, 0],
        };
        evidence::publish(
            &transaction,
            &run,
            &content(number, 20),
            &SimilarEvidence::Ready {
                variants: vec![SimilarVariant {
                    candidate: words,
                    confirmation: [0x55; 32],
                }],
            },
        )
        .unwrap();
    }
    transaction.commit().unwrap();
    for _ in 0..8 {
        catalog
            .match_similarity_window(&run, &AtomicBool::new(false))
            .unwrap();
        if scalar(
            &catalog,
            "SELECT COUNT(*) FROM similarity_match_work WHERE evidence_id = 1 AND phase = 'members'",
        ) == 1
        {
            break;
        }
    }
    catalog
        .match_similarity_window(&run, &AtomicBool::new(false))
        .unwrap();
    let count = scalar(&catalog, "SELECT COUNT(*) FROM similarity_members");
    assert!((2..=128).contains(&count));
    assert_eq!(
        scalar(
            &catalog,
            "SELECT after_evidence_id FROM similarity_match_work WHERE evidence_id = 1"
        ),
        128
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    drain(&mut catalog, &run);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM similarity_groups"),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM similarity_members AS m JOIN similarity_references AS r USING(group_id) WHERE r.evidence_id = 1 AND m.evidence_id = 3"
        ),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM similarity_members WHERE evidence_id = 2"
        ),
        2
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM similarity_members"),
        141
    );
}

#[test]
fn cancellation_superseded_sources_and_expired_owners_cannot_publish_or_release_new_work() {
    let (_directory, mut catalog) = fixture();
    let first = run("expired");
    let second = run("replacement");
    catalog.begin_similarity(&first).unwrap();
    assert_eq!(
        catalog.begin_similarity(&second).unwrap_err().code,
        "similarity_analysis_busy"
    );
    let candidate = catalog
        .load_similarity_sources("")
        .unwrap()
        .remove(0)
        .candidate;
    let fingerprint = ContentFingerprint {
        digest: "a".repeat(64),
        byte_length: 20,
    };
    catalog
        .connection
        .execute_batch("UPDATE similarity_dispatch SET lease_until_unix_ms = 0")
        .unwrap();
    catalog.begin_similarity(&second).unwrap();
    assert_eq!(
        catalog
            .verify_similarity_source(&first, &candidate, &fingerprint, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "similarity_run_retired"
    );
    assert!(
        !catalog
            .verify_similarity_source(&second, &candidate, &fingerprint, &AtomicBool::new(true))
            .unwrap()
    );
    let incompatible = SimilarContentKey {
        scheme: "other-fingerprint-v1".into(),
        digest: fingerprint.digest.clone(),
        byte_length: fingerprint.byte_length,
    };
    assert_eq!(
        catalog
            .publish_similarity(
                &second,
                &candidate,
                &incompatible,
                &SimilarEvidence::InsufficientStructure,
                &AtomicBool::new(false),
            )
            .unwrap_err()
            .code,
        "similarity_source_rejected",
    );
    catalog.connection.execute(
        "UPDATE asset_locations SET modified_unix_ms = modified_unix_ms + 1 WHERE location_id = ?1",
        [&candidate.location.location_id],
    ).unwrap();
    assert!(
        !catalog
            .verify_similarity_source(&second, &candidate, &fingerprint, &AtomicBool::new(false))
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM content_fingerprints"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM similarity_evidence"),
        0
    );
    let active: String = catalog
        .connection
        .query_row("SELECT active_run_id FROM similarity_dispatch", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(active, second.run_id);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM similarity_outcomes WHERE issue_code = 'similarity_lease_expired'"
        ),
        1
    );
}
