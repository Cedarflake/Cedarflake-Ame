use rusqlite::params;

use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, ExactAnalysisRun};
use crate::domain::organization::OrganizationSelection;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::organization::OrganizationRepository;

use super::super::super::{source_revision_token, sqlite_integer};
use super::*;

#[test]
fn file_operation_content_proof_requires_consistent_authorized_version() {
    let (_directory, mut journal, authorization) = fixture();
    assert!(journal.content_proof(&authorization).unwrap().is_none());
    journal
        .catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "original".into(),
            started_unix_ms: 1,
        })
        .unwrap();
    let state = &authorization.request.source.state;
    let identity = state.file_identity.as_ref().unwrap();
    journal
        .catalog
        .connection
        .execute(
            "INSERT INTO content_fingerprints(source_generation, scheme, digest, byte_length,
            file_identity_scheme, file_identity_value, source_revision_token, run_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'original')",
            params![
                sqlite_integer(authorization.catalog.source_generation, "generation").unwrap(),
                EXACT_FINGERPRINT_SCHEME,
                "a".repeat(64),
                sqlite_integer(state.file_size, "length").unwrap(),
                identity.scheme,
                identity.value,
                source_revision_token(state.source_revision.as_ref().unwrap()).unwrap()
            ],
        )
        .unwrap();
    let proof = journal.content_proof(&authorization).unwrap().unwrap();
    assert_eq!(proof.digest, "a".repeat(64));
    let mut changed = authorization.clone();
    changed
        .request
        .source
        .state
        .source_revision
        .as_mut()
        .unwrap()
        .value = "0000000000000002".into();
    assert!(journal.content_proof(&changed).unwrap().is_none());
    let selection = OrganizationSelection {
        asset_id: authorization.catalog.asset_id.clone(),
        location_id: authorization.catalog.location_id.clone(),
        source_generation: authorization.catalog.source_generation,
    };
    let prepared = journal
        .catalog
        .prepare_organization_selection(&[selection])
        .unwrap();
    journal
        .catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    assert_eq!(
        journal.content_proof(&authorization).unwrap(),
        Some(proof.clone())
    );
    journal
        .catalog
        .connection
        .execute_batch("DELETE FROM content_fingerprints")
        .unwrap();
    assert_eq!(journal.content_proof(&authorization).unwrap(), Some(proof));
    assert!(journal.content_proof(&changed).unwrap().is_none());
    journal
        .catalog
        .connection
        .execute(
            "INSERT INTO content_fingerprints(source_generation, scheme, digest, byte_length,
            file_identity_scheme, file_identity_value, source_revision_token, run_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'original')",
            params![
                sqlite_integer(authorization.catalog.source_generation, "generation").unwrap(),
                EXACT_FINGERPRINT_SCHEME,
                "b".repeat(64),
                sqlite_integer(state.file_size, "length").unwrap(),
                identity.scheme,
                identity.value,
                source_revision_token(state.source_revision.as_ref().unwrap()).unwrap()
            ],
        )
        .unwrap();
    assert!(journal.content_proof(&authorization).unwrap().is_none());
}
