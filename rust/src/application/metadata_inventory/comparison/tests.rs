use super::*;
use crate::adapters::LocalMediaInspector;
use crate::domain::{FileIdentityEvidence, PreviewStatus, SourceRevisionEvidence};

#[test]
fn recovery_reuses_a_complete_matching_windows_observation() {
    let (entry, prior) = matching_observation();
    for reason in [
        LibraryRecoveryAuthorityReason::JournalGap,
        LibraryRecoveryAuthorityReason::JournalReset,
        LibraryRecoveryAuthorityReason::JournalTrim,
        LibraryRecoveryAuthorityReason::JournalReconstructionFailure,
        LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure,
    ] {
        assert!(is_reused(&entry, &prior, reason), "{reason:?}");
    }
    for reason in [
        LibraryRecoveryAuthorityReason::ContainmentFailure,
        LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
    ] {
        assert!(!is_reused(&entry, &prior, reason), "{reason:?}");
    }
}

#[test]
fn equal_size_and_modification_time_do_not_hide_rewrites_or_replacements() {
    let (entry, prior) = matching_observation();
    let mut rewritten = entry.clone();
    rewritten.source_revision.as_mut().expect("revision").value = "01dc000000000002".into();
    assert!(!is_reused(
        &rewritten,
        &prior,
        LibraryRecoveryAuthorityReason::JournalTrim
    ));
    let mut replacement = entry;
    replacement.file_identity.as_mut().expect("identity").value =
        "0000000000000001:00000000000000000000000000000002".into();
    assert!(!is_reused(
        &replacement,
        &prior,
        LibraryRecoveryAuthorityReason::JournalTrim
    ));
}

#[test]
fn matching_incomplete_or_unavailable_evidence_cannot_cross_a_gap() {
    let (entry, prior) = matching_observation();
    for case in 0..7 {
        let mut entry = entry.clone();
        let mut prior = prior.clone();
        match case {
            0 => entry.file_identity = None,
            1 => entry.source_revision = None,
            2 => entry.file_identity.as_mut().expect("identity").scheme = "legacy".into(),
            3 => {
                entry.source_revision.as_mut().expect("revision").value = "0000000000000000".into()
            }
            4 => entry.file_identity.as_mut().expect("identity").value = "1:1".into(),
            5 => entry.is_reparse_point = true,
            6 => entry.placeholder_state = MetadataInventoryPlaceholderState::RecallOnDataAccess,
            _ => unreachable!(),
        }
        prior.file_identity = entry.file_identity.clone();
        prior.source_revision = entry.source_revision.clone();
        assert!(
            !is_reused(&entry, &prior, LibraryRecoveryAuthorityReason::JournalGap),
            "case {case}"
        );
    }
}

fn is_reused(
    entry: &MetadataInventoryEntry,
    prior: &AssetLocationView,
    reason: LibraryRecoveryAuthorityReason,
) -> bool {
    matches!(
        compare_entry(
            entry,
            Some(prior),
            None,
            &LocalMediaInspector::new(),
            &BTreeMap::new(),
            &mut BTreeSet::new(),
            InventoryReusePolicy::for_recovery(reason),
        ),
        EntryComparison::Unchanged
    )
}

fn matching_observation() -> (MetadataInventoryEntry, AssetLocationView) {
    let identity = FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".into(),
        value: "0000000000000001:00000000000000000000000000000001".into(),
    };
    let revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".into(),
        value: "01dc000000000001".into(),
    };
    let entry = MetadataInventoryEntry {
        relative_path: "photo.png".into(),
        kind: MetadataInventoryEntryKind::File,
        file_size: Some(100),
        modified_unix_ms: 200,
        file_identity: Some(identity.clone()),
        source_revision: Some(revision.clone()),
        placeholder_state: MetadataInventoryPlaceholderState::Available,
        is_reparse_point: false,
    };
    let prior = AssetLocationView {
        asset_id: "asset".into(),
        location_id: "location".into(),
        root_id: "root".into(),
        scan_id: "scan".into(),
        absolute_path: "photo.png".into(),
        display_path: "photo.png".into(),
        relative_path: "photo.png".into(),
        preview_path: "preview.webp".into(),
        file_size: 100,
        created_unix_ms: None,
        modified_unix_ms: 200,
        file_identity: Some(identity),
        source_revision: Some(revision),
        source_generation: 1,
        width: 2,
        height: 2,
        preview_status: PreviewStatus::Ready,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".into(),
        metadata_engine_version: "1".into(),
        camera: Default::default(),
        capture_time: None,
    };
    (entry, prior)
}
