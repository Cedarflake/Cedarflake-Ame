use rusqlite::types::Value;
use rusqlite::{Transaction, params_from_iter};

use crate::domain::{
    ExactDuplicateGroup, GalleryDuplicateAnnotation, GalleryDuplicatePage, ScanError,
};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

use super::super::exact_duplicates::read_unsigned;
use super::super::{SqliteCatalog, database_error, load_catalog_revision, user_visible_path};
use super::duplicates::{PUBLISHED_FINGERPRINTS, compatible_fingerprint};

impl GalleryDuplicateRepository for SqliteCatalog {
    fn load_gallery_duplicates(
        &mut self,
        revision: u64,
        location_ids: &[String],
    ) -> Result<GalleryDuplicatePage, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current_revision = load_catalog_revision(&transaction)?;
        if current_revision != revision {
            return Err(ScanError::new(
                "catalog_revision_changed",
                "The requested gallery window belongs to an older catalog revision",
            ));
        }
        let annotations = load_annotations(&transaction, location_ids)?;
        transaction.commit().map_err(database_error)?;
        Ok(GalleryDuplicatePage {
            revision,
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
    let placeholders = vec!["?"; location_ids.len()].join(",");
    let requested_match = compatible_fingerprint("requested");
    let member_match = compatible_fingerprint("member");
    // Resolve only content keys present in the requested window; the content and
    // source-generation indexes bound the join to their matching current members.
    let sql = format!(
        "WITH requested AS (
           SELECT requested.location_id, f.digest, f.byte_length
           FROM asset_locations AS requested
           JOIN library_roots AS roots ON requested.scan_id = roots.active_scan_id
           JOIN {PUBLISHED_FINGERPRINTS} ON {requested_match}
           WHERE requested.location_id IN ({placeholders})
         ), grouped AS (
           SELECT keys.digest, keys.byte_length, COUNT(*) AS location_count,
                  COUNT(DISTINCT member.source_generation) AS physical_file_count,
                  MIN(member.location_id) AS representative
           FROM (SELECT DISTINCT digest, byte_length FROM requested) AS keys
           JOIN {PUBLISHED_FINGERPRINTS}
             ON f.digest = keys.digest AND f.byte_length = keys.byte_length
           JOIN asset_locations AS member ON {member_match}
           JOIN library_roots AS roots ON member.scan_id = roots.active_scan_id
           GROUP BY keys.digest, keys.byte_length HAVING COUNT(*) > 1
         )
         SELECT requested.location_id, grouped.digest, grouped.byte_length,
                grouped.location_count, grouped.physical_file_count,
                grouped.representative, representative.absolute_path
         FROM requested
         JOIN grouped USING (digest, byte_length)
         JOIN asset_locations AS representative ON representative.location_id = grouped.representative
         JOIN library_roots AS representative_root ON representative.scan_id = representative_root.active_scan_id
         ORDER BY requested.location_id"
    );
    let values = location_ids.iter().cloned().map(Value::Text);
    transaction
        .prepare(&sql)
        .map_err(database_error)?
        .query_map(params_from_iter(values), |row| {
            Ok(GalleryDuplicateAnnotation {
                location_id: row.get(0)?,
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
