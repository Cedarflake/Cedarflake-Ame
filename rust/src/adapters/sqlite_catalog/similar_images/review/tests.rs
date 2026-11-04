use std::sync::atomic::AtomicBool;

use rusqlite::params;

use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::{
    SIMILARITY_SCHEME, SimilarAnalysisPhase, SimilarAnalysisProgress, SimilarAnalysisRun,
    SimilarEvidence, SimilarVariant, group_id,
};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;
use crate::ports::similar_images::SimilarAnalysisRepository;

use super::super::super::organization::tests::{fixture, publish, selected};
use super::*;

fn content(digest: char) -> SimilarContentKey {
    SimilarContentKey {
        scheme: EXACT_FINGERPRINT_SCHEME.into(),
        digest: digest.to_string().repeat(64),
        byte_length: 20,
    }
}

fn evidence_fixture() -> (tempfile::TempDir, SqliteCatalog, SimilarAnalysisRun) {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "b", 'a');
    publish(&mut catalog, "c", 'b');
    let run = SimilarAnalysisRun {
        run_id: "review".into(),
        source_run_id: "review-sources".into(),
        started_unix_ms: 1,
    };
    catalog.begin_similarity(&run).unwrap();
    publish_current_evidence(&mut catalog, &run);
    (directory, catalog, run)
}

fn publish_current_evidence(catalog: &mut SqliteCatalog, run: &SimilarAnalysisRun) {
    for source in catalog.load_similarity_sources("").unwrap() {
        let content = source.content.unwrap();
        let words = if content.digest.starts_with('a') {
            [255, 0, 0, 0]
        } else {
            [255, 15, 0, 0]
        };
        catalog
            .publish_similarity(
                run,
                &source.candidate,
                &content,
                &SimilarEvidence::Ready {
                    variants: vec![SimilarVariant {
                        candidate: words,
                        confirmation: [0x55; 32],
                    }],
                },
                &AtomicBool::new(false),
            )
            .unwrap();
    }
}

fn finish(catalog: &mut SqliteCatalog, run: &SimilarAnalysisRun) {
    for _ in 0..100 {
        if !catalog
            .match_similarity_window(run, &AtomicBool::new(false))
            .unwrap()
        {
            let mut progress = SimilarAnalysisProgress::running(run.run_id.clone());
            progress.phase = SimilarAnalysisPhase::Completed;
            catalog.finish_similarity(run, &progress).unwrap();
            return;
        }
    }
    panic!("two-content fixture did not finish");
}

#[test]
fn choices_follow_current_content_and_stale_undo_cannot_remove_a_new_exclusion() {
    let (directory, mut catalog, run) = evidence_fixture();
    finish(&mut catalog, &run);
    let exact_source = selected(&catalog, "b");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: content('a').digest,
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: exact_source.location_id,
                asset_id: exact_source.asset_id,
                source_generation: exact_source.source_generation,
            }),
        )
        .unwrap();
    assert_eq!(
        catalog
            .load_similar_groups(SimilarityDegree::Strict, false, "")
            .unwrap()
            .total_groups,
        0
    );
    let groups = catalog
        .load_similar_groups(SimilarityDegree::Standard, false, "")
        .unwrap();
    assert_eq!(groups.total_groups, 1);
    let group = &groups.groups[0];
    assert_eq!(group.member_count, 2);
    assert_eq!(group.representative.location_id, "b");
    let members = catalog
        .load_similar_members(&group.group_id, SimilarityDegree::Standard, None)
        .unwrap();
    assert_eq!(members.members[0].location_count, 2);
    assert_eq!(members.members[0].candidate.location.location_id, "b");
    assert_eq!(members.members[1].distance, 4);
    let mut context = SimilarReviewContext {
        group_id: group.group_id.clone(),
        degree: SimilarityDegree::Standard,
        revision: 0,
    };
    let selection = SimilarMemberSelection {
        content: content('b'),
        source: selected(&catalog, "c"),
    };
    let choice = catalog
        .prefer_similar_member(&context, Some(&selection))
        .unwrap();
    assert_eq!(choice.revision, 1);
    assert_eq!(
        catalog
            .ignore_similar_group(&context, true)
            .unwrap_err()
            .code,
        "similarity_group_changed"
    );
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, false, "")
            .unwrap()
            .groups[0]
            .representative
            .location_id,
        "c"
    );
    context.revision = choice.revision;
    context.revision = catalog
        .ignore_similar_group(&context, true)
        .unwrap()
        .revision;
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, false, "")
            .unwrap()
            .total_groups,
        0
    );
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, true, "")
            .unwrap()
            .total_groups,
        1
    );
    let old = catalog
        .exclude_similar_member(&context, &selection, "old-choice")
        .unwrap();
    assert_eq!(old.decision.preferred, Some(content('b')));
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, true, "")
            .unwrap()
            .total_groups,
        0
    );
    let undo = catalog.undo_similar_exclusion(&old.undo).unwrap();
    assert!(undo.restored);
    context.revision = undo.decision.revision;
    let new = catalog
        .exclude_similar_member(&context, &selection, "new-choice")
        .unwrap();
    let obsolete = catalog.undo_similar_exclusion(&old.undo).unwrap();
    assert!(!obsolete.restored);
    assert_eq!(obsolete.decision.revision, new.decision.revision);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute_batch(
            "DELETE FROM similarity_match_work;
         DELETE FROM similarity_members;
         DELETE FROM similarity_reference_variants;
         DELETE FROM similarity_references;
         DELETE FROM similarity_variants;
         DELETE FROM similarity_evidence;",
        )
        .unwrap();
    let reanalysis = SimilarAnalysisRun {
        run_id: "compatible-reanalysis".into(),
        source_run_id: "compatible-reanalysis-sources".into(),
        started_unix_ms: 2,
    };
    catalog.begin_similarity(&reanalysis).unwrap();
    publish_current_evidence(&mut catalog, &reanalysis);
    finish(&mut catalog, &reanalysis);
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, true, "")
            .unwrap()
            .total_groups,
        0
    );
    let restored = catalog.undo_similar_exclusion(&new.undo).unwrap();
    assert!(restored.restored);
    let rebuilt = catalog
        .load_similar_groups(context.degree, true, "")
        .unwrap();
    assert_eq!(rebuilt.groups[0].group_id, context.group_id);
    assert_eq!(rebuilt.groups[0].representative.location_id, "c");
    context.revision = restored.decision.revision;
    let decision = catalog.ignore_similar_group(&context, false).unwrap();
    context.revision = decision.revision;
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = 'b'",
        )
        .unwrap();
    let members = catalog
        .load_similar_members(&context.group_id, context.degree, None)
        .unwrap();
    assert_eq!(members.members[0].candidate.location.location_id, "a");
    assert_eq!(members.members[0].location_count, 1);
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = 'c'",
        )
        .unwrap();
    assert_eq!(
        catalog
            .prefer_similar_member(&context, Some(&selection))
            .unwrap_err()
            .code,
        "similarity_group_changed"
    );
    assert_eq!(
        catalog
            .load_similar_groups(context.degree, false, "")
            .unwrap()
            .total_groups,
        0
    );
    assert_eq!(
        catalog
            .load_similar_members(&context.group_id, context.degree, None)
            .unwrap()
            .decision
            .preferred,
        Some(content('b'))
    );
}

#[test]
fn group_pages_and_seeks_are_bounded_and_exclusions_remain_group_scoped() {
    let (_directory, mut catalog, run) = evidence_fixture();
    let transaction = catalog.begin_write().unwrap();
    for number in 1..=70 {
        let reference = SimilarContentKey {
            scheme: EXACT_FINGERPRINT_SCHEME.into(),
            digest: format!("{number:064x}"),
            byte_length: 20,
        };
        let id = group_id(SIMILARITY_SCHEME, &reference).unwrap();
        transaction
            .execute(
                "INSERT INTO similarity_groups VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id,
                    SIMILARITY_SCHEME,
                    reference.scheme,
                    reference.digest,
                    reference.byte_length as i64
                ],
            )
            .unwrap();
        super::super::evidence::publish(
            &transaction,
            &run,
            &reference,
            &SimilarEvidence::Ready {
                variants: vec![SimilarVariant {
                    candidate: [255, 0, 0, 0],
                    confirmation: [0x55; 32],
                }],
            },
        )
        .unwrap();
        transaction
            .execute(
                "INSERT INTO similarity_members SELECT ?1, evidence_id,
               CASE WHEN digest = ?2 THEN 0 ELSE 4 END, ?4
             FROM similarity_evidence WHERE digest IN (?2, ?3)",
                params![id, content('a').digest, content('b').digest, run.run_id],
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    let first = catalog
        .load_similar_groups(SimilarityDegree::Standard, false, "")
        .unwrap();
    assert_eq!(first.total_groups, 70);
    assert_eq!(first.groups.len(), 64);
    let next = catalog
        .load_similar_groups(
            SimilarityDegree::Standard,
            false,
            first.next_group_id.as_deref().unwrap(),
        )
        .unwrap();
    assert_eq!(next.groups.len(), 6);
    assert!(next.next_group_id.is_none());
    assert!(first.groups.last().unwrap().group_id < next.groups[0].group_id);
    let seek = catalog
        .seek_similar_groups(SimilarityDegree::Standard, false, 999)
        .unwrap();
    assert_eq!(seek.start_index, 64);
    assert_eq!(seek.page.groups[0].group_id, next.groups[0].group_id);
    let context = SimilarReviewContext {
        group_id: first.groups[0].group_id.clone(),
        degree: SimilarityDegree::Standard,
        revision: 0,
    };
    let selection = SimilarMemberSelection {
        content: content('b'),
        source: selected(&catalog, "c"),
    };
    catalog
        .exclude_similar_member(&context, &selection, "one-group-only")
        .unwrap();
    let remaining = catalog
        .load_similar_groups(context.degree, false, "")
        .unwrap();
    assert_eq!(remaining.total_groups, 69);
    assert!(
        !remaining
            .groups
            .iter()
            .any(|group| group.group_id == context.group_id)
    );
    assert_eq!(
        catalog
            .load_similar_members(&first.groups[1].group_id, context.degree, None)
            .unwrap()
            .total_members,
        2
    );
}
