use std::sync::{Arc, atomic::AtomicBool};

use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::{fixture, publish, selected};
use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_selection::{
    DuplicateCleanupGroup, DuplicateCleanupKind, FileOperationSelectionScope,
};
use crate::domain::similar_images::review::{
    SimilarMemberSelection, SimilarReviewContext, SimilarityDegree,
};
use crate::domain::similar_images::{SimilarAnalysisRun, SimilarEvidence, SimilarVariant};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;
use crate::ports::similar_images::SimilarAnalysisRepository;
use crate::ports::similar_images::review::SimilarReviewRepository;

use super::super::selection::SqliteFileOperationSelection;

fn open(
    path: &std::path::Path,
    kind: DuplicateCleanupKind,
    all_groups: bool,
) -> SqliteFileOperationSelection {
    SqliteFileOperationSelection::new(
        SqliteCatalog::open(path.to_path_buf()).unwrap(),
        FileOperationSelectionScope::DuplicateCleanup { kind, all_groups },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}

fn key() -> ExactDuplicateReviewKey {
    ExactDuplicateReviewKey {
        digest: "a".repeat(64),
        byte_length: 20,
    }
}

fn prefer(catalog: &mut SqliteCatalog, id: &str) {
    let source = selected(catalog, id);
    catalog
        .set_duplicate_preference(
            &key(),
            Some(&ExactDuplicatePreference {
                location_id: source.location_id,
                asset_id: source.asset_id,
                source_generation: source.source_generation,
            }),
        )
        .unwrap();
}

#[test]
fn cleanup_exact_freezes_only_the_selected_group_and_rejects_changed_keep_choices() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "b", 'a');
    let path = directory.path().join("catalog.sqlite3");
    let mut automatic = open(&path, DuplicateCleanupKind::Exact, true);
    let summary = automatic.prepare().unwrap();
    assert_eq!(summary.file_count, 1);
    assert_eq!(summary.cleanup.unwrap().skipped_groups, 0);
    assert_eq!(automatic.page(None).unwrap()[0].location.location_id, "b");
    assert_eq!(automatic.retained_page("").unwrap()[0].location_id, "a");
    prefer(&mut catalog, "b");
    assert!(automatic.freeze_rule(FileOrganizationRule::Direct).is_err());
    prefer(&mut catalog, "a");
    let mut selected = open(&path, DuplicateCleanupKind::Exact, false);
    selected
        .append_groups(&[DuplicateCleanupGroup::Exact { key: key() }])
        .unwrap();
    assert_eq!(selected.prepare().unwrap().file_count, 1);
    assert_eq!(selected.page(None).unwrap()[0].location.location_id, "b");
    assert_eq!(selected.retained_page("").unwrap()[0].location_id, "a");
    publish(&mut catalog, "c", 'a');
    selected.freeze_rule(FileOrganizationRule::Direct).unwrap();
    assert_eq!(selected.page(None).unwrap().len(), 1);
    prefer(&mut catalog, "b");
    assert!(selected.freeze_rule(FileOrganizationRule::Direct).is_err());
    catalog.set_duplicate_ignored(&key(), true).unwrap();
    let mut ignored = open(&path, DuplicateCleanupKind::Exact, true);
    assert_eq!(ignored.prepare().unwrap().file_count, 0);
    catalog.set_duplicate_ignored(&key(), false).unwrap();
    let mut empty_selection = open(&path, DuplicateCleanupKind::Exact, false);
    assert_eq!(empty_selection.prepare().unwrap().file_count, 0);
}

#[test]
fn cleanup_similar_keeps_selected_content_and_resolves_every_unselected_copy() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "b", 'a');
    publish(&mut catalog, "c", 'b');
    let run = SimilarAnalysisRun {
        run_id: "cleanup".into(),
        source_run_id: "cleanup-sources".into(),
        started_unix_ms: 1,
    };
    catalog.begin_similarity(&run).unwrap();
    for source in catalog.load_similarity_sources("").unwrap() {
        let content = source.content.unwrap();
        let candidate = if content.digest.starts_with('a') {
            [255, 0, 0, 0]
        } else {
            [255, 15, 0, 0]
        };
        catalog
            .publish_similarity(
                &run,
                &source.candidate,
                &content,
                &SimilarEvidence::Ready {
                    variants: vec![SimilarVariant {
                        candidate,
                        confirmation: [0x55; 32],
                    }],
                },
                &AtomicBool::new(false),
            )
            .unwrap();
    }
    let mut finished = false;
    for _ in 0..100 {
        if !catalog
            .match_similarity_window(&run, &AtomicBool::new(false))
            .unwrap()
        {
            finished = true;
            break;
        }
    }
    assert!(finished);
    let group = catalog
        .load_similar_groups(SimilarityDegree::Standard, false, "")
        .unwrap()
        .groups
        .remove(0);
    let members = catalog
        .load_similar_members(&group.group_id, SimilarityDegree::Standard, None)
        .unwrap();
    let mut context = SimilarReviewContext {
        group_id: group.group_id.clone(),
        degree: SimilarityDegree::Standard,
        revision: group.decision.revision,
    };
    let path = directory.path().join("catalog.sqlite3");
    let kind = DuplicateCleanupKind::Similar {
        degree: context.degree,
    };
    let mut automatic = open(&path, kind.clone(), true);
    let summary = automatic.prepare().unwrap();
    assert_eq!(group.representative.location_id, "a");
    assert_eq!(summary.file_count, 1);
    assert_eq!(summary.cleanup.unwrap().kept_files, 2);
    assert_eq!(automatic.page(None).unwrap()[0].location.location_id, "c");
    let keep_c = SimilarMemberSelection {
        content: members.members[1].content.clone(),
        source: selected(&catalog, "c"),
    };
    context.revision = catalog
        .prefer_similar_member(&context, &keep_c, true)
        .unwrap()
        .revision;
    assert!(automatic.freeze_rule(FileOrganizationRule::Direct).is_err());
    let mut cleanup = open(&path, kind.clone(), false);
    cleanup
        .append_groups(&[DuplicateCleanupGroup::Similar {
            group_id: group.group_id,
        }])
        .unwrap();
    let summary = cleanup.prepare().unwrap();
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.cleanup.unwrap().kept_files, 1);
    let ids: Vec<_> = cleanup
        .page(None)
        .unwrap()
        .into_iter()
        .map(|item| item.location.location_id)
        .collect();
    assert_eq!(ids, ["a", "b"]);
    cleanup.freeze_rule(FileOrganizationRule::Direct).unwrap();
    let keep_a = SimilarMemberSelection {
        content: members.members[0].content.clone(),
        source: selected(&catalog, "a"),
    };
    context.revision = catalog
        .prefer_similar_member(&context, &keep_a, true)
        .unwrap()
        .revision;
    assert!(cleanup.freeze_rule(FileOrganizationRule::Direct).is_err());
    let mut keep_all = open(&path, kind.clone(), true);
    assert_eq!(keep_all.prepare().unwrap().file_count, 0);
    catalog
        .prefer_similar_member(&context, &keep_c, false)
        .unwrap();
    let mut keep_copies = open(&path, kind, true);
    assert_eq!(keep_copies.prepare().unwrap().file_count, 1);
    assert_eq!(keep_copies.page(None).unwrap()[0].location.location_id, "c");
    let mut strict = open(
        &path,
        DuplicateCleanupKind::Similar {
            degree: SimilarityDegree::Strict,
        },
        true,
    );
    assert_eq!(strict.prepare().unwrap().file_count, 0);
}
