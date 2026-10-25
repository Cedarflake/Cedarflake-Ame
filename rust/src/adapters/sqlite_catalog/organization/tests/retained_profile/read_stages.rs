use rusqlite::params_from_iter;

use crate::adapters::sqlite_catalog::gallery::{build_gallery_asset_query, organization_queue};
use crate::adapters::sqlite_catalog::gallery_snapshot::GalleryReadTransaction;

use super::*;

#[test]
#[ignore = "manual retained-catalog read profile; requires a populated isolated organization snapshot"]
fn profile_retained_organization_read_stages() {
    let (directory, path, mut profile) = prepare_copy("organization-read-stages.jsonl");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let album_id: String = catalog
        .connection
        .query_row(
            "SELECT album_id FROM organization_records WHERE album_id IS NOT NULL
             GROUP BY album_id ORDER BY COUNT(*) DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(scalar(&catalog, "SELECT COUNT(*) FROM organization_records") > 40_000);
    catalog
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for (name, collection, queue) in [
        ("album", GalleryCollection::Album { album_id }, None),
        ("favorites", GalleryCollection::Favorites, None),
        (
            "pending",
            GalleryCollection::Library,
            Some(OrganizationQueue::Pending),
        ),
    ] {
        let query = GalleryQuery {
            collection,
            organization_queue: queue,
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..GalleryQuery::default()
        };
        let built =
            build_gallery_asset_query(&query, None, None, None, i64::from(PAGE_SIZE)).unwrap();
        let plan = catalog
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {}", built.sql))
            .unwrap()
            .query_map(params_from_iter(&built.parameters), |row| {
                row.get::<_, String>(3)
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        profile.record(json!({ "query": name, "page_query_plan": plan }));
        for sample in 0..2 {
            let read = GalleryReadTransaction::begin(
                &mut catalog.connection,
                &path,
                &query,
                "read-profile",
            )
            .unwrap();
            let page = profile.measure(&format!("{name}_page_{sample}"), || {
                read.load_snapshot(PAGE_SIZE, None, None, None)
            });
            assert!(page.assets.len() <= PAGE_SIZE as usize);
            let timeline = profile.measure(&format!("{name}_timeline_{sample}"), || {
                read.load_timeline()
            });
            read.commit().unwrap();
            profile.record(json!({
                "query": name,
                "sample": sample,
                "groups": timeline.total_items,
                "physical_files": timeline.total_source_items,
                "returned_assets": page.assets.len(),
            }));
            if queue.is_some() {
                let transaction = catalog.connection.transaction().unwrap();
                profile.measure(&format!("{name}_counts_{sample}"), || {
                    organization_queue::load_counts(&transaction, &query)
                });
                transaction.commit().unwrap();
            }
        }
    }
    drop(catalog);
    directory.close().unwrap();
}
