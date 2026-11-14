use rusqlite::params;

use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, ExactAnalysisRun};
use crate::domain::similar_images::{SimilarAnalysisRun, SimilarContentKey, group_id};
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::similar_images::SimilarAnalysisRepository;

use super::confirmation::{LEGACY_PARAMETERS, LEGACY_SCHEME};
use super::*;

#[test]
fn similarity_upgrade_preserves_prior_choices_and_separates_new_choices_from_cache() {
    let (_directory, mut catalog) = fixture();
    downgrade_to_v73_for_test(&catalog.connection);
    catalog.connection.execute(
        "INSERT INTO exact_duplicate_decisions(scheme, digest, byte_length, ignored, updated_unix_ms)
         VALUES (?1, ?2, 17, 1, 42)",
        params![EXACT_FINGERPRINT_SCHEME, "a".repeat(64)],
    ).unwrap();
    migrate(&mut catalog.connection).unwrap();
    let retained: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT ignored, updated_unix_ms FROM exact_duplicate_decisions WHERE byte_length = 17",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(retained, (1, 42));
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "similarity-source-fixture".into(),
            started_unix_ms: 1,
        })
        .unwrap();
    let content = SimilarContentKey {
        scheme: EXACT_FINGERPRINT_SCHEME.into(),
        digest: "c".repeat(64),
        byte_length: 23,
    };
    let group = group_id(LEGACY_SCHEME, &content).unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO similarity_runs VALUES ('similarity-fixture', 'similarity-source-fixture',
             ?1, 'image_hasher', '3.1.1', ?2, 1)",
            params![LEGACY_SCHEME, LEGACY_PARAMETERS],
        )
        .unwrap();
    catalog.connection.execute(
        "INSERT INTO similarity_evidence VALUES (1, 'similarity-fixture', ?1, ?2, ?3, 23, 'ready', NULL)",
        params![LEGACY_SCHEME, content.scheme, content.digest],
    ).unwrap();
    catalog
        .connection
        .execute_batch("INSERT INTO similarity_variants VALUES (1, 0, 4660, 22136, 39612, 57072)")
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO similarity_groups VALUES (?1, ?2, ?3, ?4, 23)",
            params![group, LEGACY_SCHEME, content.scheme, content.digest],
        )
        .unwrap();
    catalog
        .connection
        .execute("INSERT INTO similarity_references VALUES (?1, 1)", [&group])
        .unwrap();
    catalog.connection.execute(
        "INSERT INTO similarity_reference_variants
         SELECT ?1, variant, word0, word1, word2, word3 FROM similarity_variants WHERE evidence_id = 1",
        [&group],
    ).unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO similarity_members VALUES (?1, 1, 0, 'similarity-fixture')",
            [&group],
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO similarity_decisions VALUES (?1, ?2, ?3, 23, 1, 1)",
            params![group, content.scheme, content.digest],
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO similarity_exclusions VALUES (?1, ?2, ?3, 24, 'saved-exclusion')",
            params![group, content.scheme, "d".repeat(64)],
        )
        .unwrap();
    validate_rows(&catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO similarity_match_work VALUES (1, 'references', 0, 0, 0, 0, 0, '', -1, 0);
         CREATE TRIGGER reject_confirmation_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 75 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        )
        .unwrap();
    catalog.connection.execute(
        "UPDATE similarity_dispatch SET active_run_id = 'similarity-fixture', lease_until_unix_ms = ?1",
        [crate::adapters::sqlite_catalog::unix_time_ms() + 60_000],
    ).unwrap();
    assert!(migrate_confirmation(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 74).unwrap();
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_confirmation_upgrade")
        .unwrap();
    migrate_confirmation(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 75).unwrap();
    let retained: (i64, i64, String) = catalog
        .connection
        .query_row(
            "SELECT word0, confirmation IS NULL, phase FROM similarity_variants
         JOIN similarity_match_work USING(evidence_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(retained, (4660, 1, "complete".into()));
    assert!(!catalog.has_similarity_evidence(&content).unwrap());
    let old_run = SimilarAnalysisRun {
        run_id: "similarity-fixture".into(),
        source_run_id: "similarity-source-fixture".into(),
        started_unix_ms: 1,
    };
    assert_eq!(
        catalog.renew_similarity(&old_run).unwrap_err().code,
        "similarity_run_retired"
    );
    catalog
        .begin_similarity(&SimilarAnalysisRun {
            run_id: "confirmed-fixture".into(),
            source_run_id: "confirmed-source-fixture".into(),
            started_unix_ms: 2,
        })
        .unwrap();
    let retired: String = catalog
        .connection
        .query_row(
            "SELECT issue_code FROM similarity_outcomes WHERE run_id = 'similarity-fixture'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retired, "similarity_scheme_retired");
    validate_rows(&catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("UPDATE similarity_variants SET confirmation = zeroblob(32)")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_similarity_contract_invalid"
    );
    catalog
        .connection
        .execute_batch("UPDATE similarity_variants SET confirmation = NULL")
        .unwrap();
    catalog
        .connection
        .execute_batch("UPDATE similarity_reference_variants SET word0 = word0 + 1")
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_similarity_contract_invalid"
    );
    catalog
        .connection
        .execute_batch(
            "DELETE FROM similarity_match_work;
         UPDATE similarity_dispatch SET active_run_id = NULL, lease_until_unix_ms = NULL;
         DELETE FROM similarity_outcomes;
         DELETE FROM similarity_members;
         DELETE FROM similarity_reference_variants;
         DELETE FROM similarity_references;
         DELETE FROM similarity_variants;
         DELETE FROM similarity_evidence;
         DELETE FROM similarity_runs;",
        )
        .unwrap();
    validate_rows(&catalog.connection).unwrap();
    let saved: (i64, String, String) = catalog.connection.query_row(
        "SELECT decision.ignored, decision.preferred_digest, excluded.exclusion_id
         FROM similarity_decisions AS decision JOIN similarity_exclusions AS excluded USING(group_id)",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(saved, (1, content.digest, "saved-exclusion".into()));
}

#[test]
fn similarity_validation_checks_owned_foreign_keys_without_claiming_other_tables() {
    let (_directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
         CREATE TABLE unrelated_parent(id INTEGER PRIMARY KEY);
         CREATE TABLE unrelated_child(parent_id INTEGER REFERENCES unrelated_parent(id));
         INSERT INTO unrelated_child VALUES (1);",
        )
        .unwrap();
    validate_rows(&catalog.connection).unwrap();

    catalog
        .connection
        .execute_batch(
            "INSERT INTO similarity_variants(evidence_id, variant, word0, word1, word2, word3)
         VALUES (1234567, 0, 1, 2, 3, 4);",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_similarity_contract_invalid"
    );
}

#[test]
fn similarity_upgrade_failure_rolls_back_all_new_objects_before_retry() {
    let (_directory, mut catalog) = fixture();
    downgrade_to_v73_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_similarity_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 74 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    let (version, new_objects): (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT version FROM schema_info),
                (SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'similarity_%')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((version, new_objects), (73, 0));
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_similarity_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    current_schema::validate_schema_version(&catalog.connection, 74).unwrap();
}
