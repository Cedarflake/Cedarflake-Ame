use rusqlite::types::Value;
use rusqlite::{Transaction, params_from_iter};

use crate::domain::{
    ExactDuplicateGroup, GalleryDuplicateAnnotation, GalleryDuplicatePage, ScanError,
};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

use super::super::exact_duplicates::read_unsigned;
use super::super::{SqliteCatalog, database_error, load_catalog_revision, user_visible_path};
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};

impl GalleryDuplicateRepository for SqliteCatalog {
    fn load_gallery_duplicate_locations(
        &mut self,
        digest: &str,
        byte_length: u64,
        after_location_id: &str,
    ) -> Result<crate::domain::exact_duplicates::GalleryDuplicateCandidates, ScanError> {
        super::member_locations::load(self, digest, byte_length, after_location_id)
    }
    fn load_gallery_duplicates(
        &mut self,
        revision: u64,
        location_ids: &[String],
    ) -> Result<GalleryDuplicatePage, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current_revision = load_catalog_revision(&transaction)?;
        if current_revision < revision {
            return Err(ScanError::new(
                "catalog_revision_changed",
                "Duplicate annotations cannot precede the requested gallery revision",
            ));
        }
        // Badges inspect current membership for retained visible locations. A newer
        // publication may change their count without invalidating the visible window.
        let annotations = load_annotations(&transaction, location_ids)?;
        transaction.commit().map_err(database_error)?;
        Ok(GalleryDuplicatePage {
            revision: current_revision,
            annotations,
        })
    }
}

fn load_annotations(
    transaction: &Transaction<'_>,
    location_ids: &[String],
) -> Result<Vec<GalleryDuplicateAnnotation>, ScanError> {
    if location_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = annotation_query(location_ids.len());
    let values = location_ids.iter().cloned().map(Value::Text);
    transaction
        .prepare(&sql)
        .map_err(database_error)?
        .query_map(params_from_iter(values), |row| {
            Ok(GalleryDuplicateAnnotation {
                location_id: row.get(0)?,
                pending_location_count: read_unsigned(row, 7)?,
                group: ExactDuplicateGroup {
                    digest: row.get(1)?,
                    byte_length: read_unsigned(row, 2)?,
                    location_count: read_unsigned(row, 3)?,
                    physical_file_count: read_unsigned(row, 4)?,
                    representative_location_id: row.get(5)?,
                    representative_path: user_visible_path(&row.get::<_, String>(6)?),
                },
            })
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)
}

fn annotation_query(location_count: usize) -> String {
    let placeholders = vec!["?"; location_count].join(",");
    let requested_match = compatible_fingerprint("requested");
    let member_match = compatible_fingerprint("member");
    // Materialize only the requested keys. Push their digests into both view branches
    // and join members by generation, avoiding repeated equal-size peer scans.
    format!(
        "WITH requested AS MATERIALIZED (
           SELECT requested.location_id, f.digest, f.byte_length
           FROM asset_locations AS requested
           JOIN library_roots AS roots ON requested.scan_id = roots.active_scan_id
           JOIN {DISPLAY_FINGERPRINTS} ON {requested_match}
           WHERE requested.location_id IN ({placeholders})
         ), grouped AS (
           SELECT keys.digest, keys.byte_length, COUNT(*) AS location_count,
                  SUM(f.is_pending) AS pending_location_count,
                  COUNT(DISTINCT member.source_generation) AS physical_file_count,
                  MIN(member.location_id) AS representative
           FROM (SELECT DISTINCT digest, byte_length FROM requested) AS keys
           JOIN {DISPLAY_FINGERPRINTS}
             ON f.digest = keys.digest AND f.byte_length = keys.byte_length
               AND f.digest IN (SELECT digest FROM requested)
           JOIN asset_locations AS member INDEXED BY asset_locations_exact_source ON {member_match}
           JOIN library_roots AS roots ON member.scan_id = roots.active_scan_id
           GROUP BY keys.digest, keys.byte_length HAVING COUNT(*) > 1
         )
         SELECT requested.location_id, grouped.digest, grouped.byte_length,
                grouped.location_count, grouped.physical_file_count,
                grouped.representative, representative.absolute_path, grouped.pending_location_count
         FROM requested
         JOIN grouped USING (digest, byte_length)
         JOIN asset_locations AS representative ON representative.location_id = grouped.representative
         JOIN library_roots AS representative_root ON representative.scan_id = representative_root.active_scan_id
         ORDER BY requested.location_id"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_plan_limits_content_keys_and_joins_members_by_source_generation() {
        let directory = tempfile::tempdir().unwrap();
        let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        let plan = catalog
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {}", annotation_query(1)))
            .unwrap()
            .query_map(["visible"], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(
            plan.iter().any(|step| step.contains(
                "SEARCH requested USING INDEX asset_locations_location_id (location_id=?)"
            )),
            "{plan:?}"
        );
        assert!(plan.iter().filter(|step| step.contains(
            "content_fingerprints_content (scheme=? AND digest=?)"
        )).count() >= 2, "{plan:?}");
        assert!(
            plan.iter().any(|step| step.contains(
                "SEARCH member USING INDEX asset_locations_exact_source (source_generation=?)"
            )),
            "{plan:?}"
        );
    }
}
