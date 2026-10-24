use std::collections::HashMap;

use crate::adapters::sqlite_catalog::catalog_delta::location_mutation;
use crate::domain::{
    CatalogDeltaMutation, DerivedEvidenceDisposition, IncrementalReconciliationOutcome,
    LibraryChangeId,
};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

use super::*;

pub(in super::super) fn invalidate(catalog: &mut SqliteCatalog, id: &str) {
    let mut location = catalog.load_active_location(id).unwrap().unwrap();
    location.source_generation = 0;
    let transaction = catalog.connection.transaction().unwrap();
    location_mutation::apply(
        &transaction,
        &location.scan_id,
        None,
        &location.root_id,
        &CatalogDeltaMutation {
            change_id: LibraryChangeId::new(1).unwrap(),
            outcome: IncrementalReconciliationOutcome::Modified,
            evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
            remove_location_ids: vec![location.location_id.clone()],
            upsert_location: Some(location.clone()),
            retained_preview_expectation: None,
        },
        &mut HashMap::new(),
        &mut HashMap::new(),
    )
    .unwrap();
    transaction
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .unwrap();
    transaction.commit().unwrap();
}

#[test]
fn late_conflict_removes_both_album_memberships_but_keeps_the_latest_favorite() {
    let (_directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    assign(&mut catalog, "assign-a", &["a"], &first);
    assign(&mut catalog, "assign-c", &["c"], &second);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let favorites = GalleryQuery {
        collection: GalleryCollection::Favorites,
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..album_query(&first)
    };
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    assert_eq!(ids(&mut catalog, &album_query(&first)), ["a"]);
    assert_eq!(ids(&mut catalog, &album_query(&second)), ["c"]);
    assert_eq!(ids(&mut catalog, &favorites), ["c"]);
    let generations = [
        selected(&catalog, "a").source_generation,
        selected(&catalog, "c").source_generation,
    ];
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    assert!(ids(&mut catalog, &album_query(&first)).is_empty());
    assert!(ids(&mut catalog, &album_query(&second)).is_empty());
    assert_eq!(ids(&mut catalog, &favorites), ["a"]);
    assign(&mut catalog, "resolve", &["c"], &second);
    assert_eq!(ids(&mut catalog, &album_query(&second)), ["a"]);
    change(
        &mut catalog,
        "remove",
        &["c"],
        OrganizationAction::RemoveAlbum,
    );
    assert!(ids(&mut catalog, &album_query(&second)).is_empty());
    assert_eq!(ids(&mut catalog, &favorites), ["a"]);
}

#[test]
fn pending_display_keeps_the_album_until_confirmed_content_splits() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute("UPDATE asset_locations SET file_identity_scheme = 'fixture', file_identity_value = location_id", []).unwrap();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album = create_album(&mut catalog, "Retained");
    assign(&mut catalog, "assign", &["a"], &album);
    let query = album_query(&album);
    invalidate(&mut catalog, "a");
    assert_eq!(ids(&mut catalog, &query), ["a"]);
    let snapshot = catalog
        .load_query_snapshot(1, &query, "pending", None)
        .unwrap();
    assert_eq!(snapshot.timeline.total_source_items, 2);
    let annotations = catalog
        .load_gallery_duplicates(snapshot.snapshot.revision, &["a".into()])
        .unwrap();
    assert_eq!(annotations.annotations[0].pending_location_count, 1);
    assert_eq!(
        catalog
            .prepare_organization_selection(&[selected(&catalog, "a")])
            .unwrap_err()
            .code,
        "organization_evidence_pending"
    );
    publish_batch(&mut catalog, &["a"], 'b');
    assert_eq!(ids(&mut catalog, &query), ["c"]);
    let snapshot = catalog
        .load_query_snapshot(1, &query, "split", None)
        .unwrap();
    assert_eq!(
        (
            snapshot.timeline.total_items,
            snapshot.timeline.total_source_items
        ),
        (1, 1)
    );
}

#[test]
fn current_binding_and_preferred_copy_survive_cache_loss_but_not_content_replacement() {
    let (_directory, mut catalog, query) = grouped();
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: "c".into(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "DELETE FROM gallery_content_fingerprints; DELETE FROM content_fingerprints;",
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &query), ["b", "c"]);
    let hidden = selected(&catalog, "a");
    let anchored = catalog
        .load_snapshot_around_asset(1, &query, "cache-reset", "a", &hidden.asset_id, 1)
        .unwrap();
    assert_eq!(
        anchored
            .query_anchor_resolution
            .unwrap()
            .location_id
            .as_deref(),
        Some("c")
    );
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
    assert_eq!(ids(&mut catalog, &query), ["b", "c"]);
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = 'c'",
            [],
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &query), ["b"]);
}

#[test]
fn a_pending_collection_read_does_not_scan_fifty_thousand_lineage_records() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (_directory, mut catalog) = fixture();
    catalog.connection.execute("UPDATE asset_locations SET file_identity_scheme = 'fixture', file_identity_value = location_id", []).unwrap();
    publish_batch(&mut catalog, &["a"], 'a');
    let album = create_album(&mut catalog, "History");
    assign(&mut catalog, "assign", &["a"], &album);
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50000)
         INSERT INTO organization_records
         SELECT 'lineage-' || value, album_id, disposition, favorite, organization_revision, favorite_revision,
           created_unix_ms, updated_unix_ms, NULL FROM sequence, organization_records;
         INSERT INTO organization_record_groups
         SELECT record_id, (SELECT group_id FROM organization_record_groups LIMIT 1)
         FROM organization_records WHERE record_id LIKE 'lineage-%';
         INSERT INTO organization_bindings
         SELECT record.record_id, record.record_id, 100 + CAST(substr(record.record_id, 9) AS INTEGER),
           binding.byte_length, NULL, NULL, NULL, binding.fingerprint_scheme, binding.digest
         FROM organization_records AS record, organization_bindings AS binding
         WHERE record.record_id LIKE 'lineage-%';"
    ).unwrap();
    invalidate(&mut catalog, "a");
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = ticks.clone();
    catalog
        .connection
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let result = catalog.load_query_snapshot(1, &album_query(&album), "history", None);
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    let result = result.unwrap();
    assert_eq!(result.snapshot.assets.len(), 1);
    assert_eq!(result.timeline.total_items, 1);
    let steps = ticks.load(Ordering::Relaxed) * 100;
    assert!(
        steps < 20_000,
        "one pending group read used at least {steps} SQLite steps"
    );
}
