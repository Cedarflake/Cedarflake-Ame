use std::path::Path;
use std::sync::Arc;

use crate::domain::organization_batch::{OrganizationBatchAction, OrganizationBatchPhase};
use crate::domain::{GalleryCollection, GalleryDuplicateMode, GalleryQuery};
use crate::ports::organization_batch::{OrganizationBatchControl, OrganizationBatchRepository};

use super::queries::publish_batch;
use super::transitions::{change, create_album, decision};
use super::*;

mod mutations;

fn open(
    directory: &Path,
    catalog: &SqliteCatalog,
    query: GalleryQuery,
    all_matching: bool,
) -> (SqliteOrganizationBatch, Arc<OrganizationBatchControl>) {
    let control = Arc::new(OrganizationBatchControl::new());
    let batch = SqliteOrganizationBatch::new(
        SqliteCatalog::open(directory.join("catalog.sqlite3")).unwrap(),
        query,
        scalar(catalog, "SELECT revision FROM catalog_state"),
        all_matching,
        Arc::clone(&control),
    )
    .unwrap();
    (batch, control)
}

fn favorite(value: bool) -> OrganizationBatchAction {
    OrganizationBatchAction::Favorite { favorite: value }
}

fn organize(action: OrganizationAction) -> OrganizationBatchAction {
    OrganizationBatchAction::Organize { action }
}

#[test]
fn explicit_selection_spans_input_chunks_and_saves_every_selected_group() {
    let (directory, mut catalog) = fixture();
    let ids = (0..600).map(|i| format!("bulk-{i:04}")).collect::<Vec<_>>();
    let locations = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), None, i as i64))
        .collect::<Vec<_>>();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-bulk",
        "root-bulk",
        "E:\\Fixture",
        &locations,
    );
    let assets = ids
        .iter()
        .map(|id| selected(&catalog, id).asset_id)
        .collect::<Vec<_>>();
    let (mut batch, control) = open(directory.path(), &catalog, GalleryQuery::default(), false);
    assert_eq!(
        batch.append_ids(&assets[..257]).unwrap_err().code,
        "organization_selection_invalid"
    );
    for chunk in assets.chunks(256) {
        batch.append_ids(chunk).unwrap();
    }
    batch.append_ids(&assets[..2]).unwrap();
    let summary = batch.prepare().unwrap();
    assert_eq!(summary.group_count, 600);
    assert_eq!(summary.favorite_count, 0);
    assert_eq!(control.snapshot().phase, OrganizationBatchPhase::Ready);
    assert_eq!(
        batch
            .apply("all-explicit", &favorite(true))
            .unwrap()
            .changed_groups,
        600
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
        ),
        600
    );
    assert_eq!(control.snapshot().completed_groups, 600);
    assert_eq!(control.snapshot().phase, OrganizationBatchPhase::Completed);

    let query = GalleryQuery {
        root_id: Some("root-bulk".into()),
        collection: GalleryCollection::Favorites,
        ..GalleryQuery::default()
    };
    let (mut excluded, _) = open(directory.path(), &catalog, query, true);
    for chunk in assets[..300].chunks(256) {
        excluded.append_ids(chunk).unwrap();
    }
    assert_eq!(excluded.prepare().unwrap().group_count, 300);
    assert_eq!(
        excluded
            .apply("all-except", &favorite(false))
            .unwrap()
            .changed_groups,
        300
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
        ),
        300
    );
}

#[test]
fn visible_folded_exclusions_and_folder_scope_keep_whole_group_authority() {
    let (directory, mut catalog) = fixture();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET relative_path = 'Folder/c.jpg', parent_relative_path = 'Folder',
         absolute_path = 'D:\\Fixture\\Folder\\c.jpg' WHERE location_id = 'c'",
            [],
        )
        .unwrap();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let a = selected(&catalog, "a").asset_id;
    let folded = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    let (mut batch, _) = open(directory.path(), &catalog, folded, true);
    batch.append_ids(std::slice::from_ref(&a)).unwrap();
    assert_eq!(batch.prepare().unwrap().group_count, 1);
    drop(batch);

    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.append_ids(&[a]).unwrap();
    assert_eq!(batch.prepare().unwrap().group_count, 2);
    drop(batch);

    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        folder_relative_path: Some("Folder".into()),
        include_descendants: false,
        ..GalleryQuery::default()
    };
    let (mut batch, _) = open(directory.path(), &catalog, scoped, true);
    assert_eq!(batch.prepare().unwrap().group_count, 1);
    assert_eq!(
        batch
            .apply("scoped-favorite", &favorite(true))
            .unwrap()
            .changed_groups,
        1
    );
    assert!(decision(&catalog, "a").2);
    assert!(decision(&catalog, "c").2);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_sources"),
        2
    );
}

#[test]
fn empty_unknown_and_post_prepare_inputs_do_not_admit_a_write() {
    let (directory, catalog) = fixture();
    let (mut empty, _) = open(directory.path(), &catalog, GalleryQuery::default(), false);
    assert_eq!(
        empty.prepare().unwrap_err().code,
        "organization_selection_empty"
    );
    empty.append_ids(&["unknown-asset".into()]).unwrap();
    assert_eq!(
        empty.prepare().unwrap_err().code,
        "organization_selection_changed"
    );
    drop(empty);
    let (mut batch, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    batch.prepare().unwrap();
    assert_eq!(
        batch
            .append_ids(&[selected(&catalog, "a").asset_id])
            .unwrap_err()
            .code,
        "organization_selection_invalid"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
    drop(batch);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        scalar(&reopened, "SELECT COUNT(*) FROM sqlite_temp_master"),
        0
    );
}

#[test]
fn summaries_report_mixed_assignments_favorites_and_late_merge_conflicts() {
    let (directory, mut catalog) = fixture();
    let first = create_album(&mut catalog, "First");
    let second = create_album(&mut catalog, "Second");
    change(
        &mut catalog,
        "first",
        &["a", "b"],
        OrganizationAction::AssignAlbum {
            album_id: first.clone(),
        },
    );
    change(
        &mut catalog,
        "second",
        &["c"],
        OrganizationAction::AssignAlbum { album_id: second },
    );
    catalog
        .set_organization_favorite("one-favorite", &prepare(&catalog, &["a"]), true)
        .unwrap();
    let query = GalleryQuery {
        root_id: Some("root-a".into()),
        ..GalleryQuery::default()
    };
    let (mut scoped, _) = open(directory.path(), &catalog, query, true);
    let summary = scoped.prepare().unwrap();
    assert_eq!(
        (
            summary.group_count,
            summary.favorite_count,
            summary.assigned_count
        ),
        (2, 1, 2)
    );
    assert_eq!(summary.common_album.unwrap().album_id, first);
    drop(scoped);
    let (mut all, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    assert!(all.prepare().unwrap().common_album.is_none());
    drop(all);

    publish_batch(&mut catalog, &["a", "c"], 'a');
    let (mut merged, _) = open(directory.path(), &catalog, GalleryQuery::default(), true);
    let summary = merged.prepare().unwrap();
    assert_eq!(
        (
            summary.group_count,
            summary.conflicting_count,
            summary.assigned_count
        ),
        (2, 1, 1)
    );
    assert_eq!(summary.favorite_count, 1);
    assert!(summary.common_album.is_none());
}
