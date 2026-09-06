use crate::adapters::LocalMediaInspector;
use crate::domain::{
    FileIdentityEvidence, LibraryChangeFailure, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, SourceRevisionEvidence, TerminalMediaEvidence,
};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};
use crate::ports::MediaInspector;

use super::inventory_matches_terminal_media_evidence;

#[test]
fn legacy_ico_negative_evidence_is_not_reused_after_content_admission_revision() {
    let bytes = encode_rgb_quadrants(MediaFixtureFormat::Ico, 32, 32).expect("real ICO bytes");
    assert_eq!(
        image::guess_format(&bytes).expect("ICO signature"),
        image::ImageFormat::Ico
    );
    let entry = MetadataInventoryEntry {
        relative_path: "valid-ico.data".to_owned(),
        kind: MetadataInventoryEntryKind::File,
        file_size: Some(bytes.len() as u64),
        modified_unix_ms: 42,
        file_identity: Some(FileIdentityEvidence {
            scheme: "fixture-identity".to_owned(),
            value: "same-file".to_owned(),
        }),
        source_revision: Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0000000000000001".to_owned(),
        }),
        placeholder_state: MetadataInventoryPlaceholderState::Available,
        is_reparse_point: false,
    };
    let inspector = LocalMediaInspector::new();
    assert_eq!(inspector.inspection_engine_version(), 2);
    let mut evidence = TerminalMediaEvidence {
        relative_path: entry.relative_path.clone(),
        file_size: bytes.len() as u64,
        modified_unix_ms: entry.modified_unix_ms,
        file_identity: entry.file_identity.clone(),
        source_revision: entry.source_revision.clone(),
        source_generation: 1,
        inspection_engine_id: inspector.inspection_engine_id().to_owned(),
        inspection_engine_version: 1,
        issue: LibraryChangeFailure {
            code: "media_type_unsupported".to_owned(),
            message: "Legacy missing ICO signature".to_owned(),
        },
    };
    assert!(!inventory_matches_terminal_media_evidence(
        &entry, &evidence, &inspector
    ));
    evidence.inspection_engine_version = inspector.inspection_engine_version();
    assert!(
        inventory_matches_terminal_media_evidence(&entry, &evidence, &inspector),
        "unchanged negative caching still requires an exactly matching inspection algorithm"
    );
    evidence
        .source_revision
        .as_mut()
        .expect("fixture revision")
        .value = "0000000000000002".to_owned();
    assert!(!inventory_matches_terminal_media_evidence(
        &entry, &evidence, &inspector
    ));
}
