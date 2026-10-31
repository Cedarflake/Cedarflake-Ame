use crate::domain::gallery_metadata_filter::GalleryMetadataFilter;

use super::*;

#[test]
fn metadata_scope_selects_eligible_group_member_for_album_and_whole_result_selection() {
    let (_directory, mut catalog, mut query) = grouped();
    catalog.connection.execute_batch("UPDATE asset_locations SET camera_make = CASE location_id WHEN 'c' THEN 'Selected' ELSE 'Other' END;").unwrap();
    query.metadata_filter = GalleryMetadataFilter {
        camera_text: "Selected".into(),
        ..Default::default()
    };
    let snapshot = catalog
        .load_query_snapshot(10, &query, "filtered", None)
        .unwrap();
    assert_eq!(snapshot.timeline.total_items, 1);
    assert_eq!(snapshot.timeline.total_source_items, 1);
    assert_eq!(snapshot.snapshot.assets[0].location_id, "c");
    assert_eq!(
        catalog
            .load_gallery_layout_manifest_chunk(10, &query, "filtered", None)
            .unwrap()
            .location_ids,
        ["c"]
    );
    let member = selected(&catalog, "c");
    let anchored = catalog
        .load_snapshot_around_asset(10, &query, "filtered", "c", &member.asset_id, 0)
        .unwrap();
    assert_eq!(anchored.assets[0].location_id, "c");
    catalog
        .connection
        .execute_batch("CREATE TEMP TABLE organization_batch_ids(asset_id TEXT PRIMARY KEY)")
        .unwrap();
    let selection =
        crate::adapters::sqlite_catalog::gallery::selection::selected_locations(&query, true)
            .unwrap();
    let selected = catalog
        .connection
        .prepare(&selection.sql)
        .unwrap()
        .query_map(rusqlite::params_from_iter(selection.parameters), |row| {
            row.get::<_, String>(1)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(selected, ["c"]);
}
