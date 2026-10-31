use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::super::{database_error, exact_duplicates::read_unsigned};
use super::super::authority;

pub(super) fn inspect(
    connection: &Connection,
    digest: &mut blake3::Hasher,
    checkpoint: &mut dyn FnMut() -> Result<(), ScanError>,
) -> Result<u64, ScanError> {
    let count = connection.query_row(
        "SELECT count(*) FROM temp.organization_cleanup_groups AS selected
         WHERE EXISTS(SELECT 1 FROM organization_record_groups AS member
           JOIN classification_decisions AS decision USING(record_id)
           WHERE member.group_id = selected.group_id AND (decision.choice_revision > 0 OR decision.review_revision > 0))
           OR EXISTS(SELECT 1 FROM organization_record_groups AS member
             JOIN classification_review_bookmark AS bookmark USING(record_id) WHERE member.group_id = selected.group_id)",
        [], |row| read_unsigned(row, 0),
    ).map_err(database_error)?;
    for (kind, query, columns) in [
        ("classification_decisions", "SELECT decision.record_id, category_id, choice, choice_revision, choice_restored,
           deferred, review_revision, review_restored FROM temp.organization_cleanup_groups AS selected
         CROSS JOIN organization_record_groups AS member USING(group_id)
         CROSS JOIN classification_decisions AS decision USING(record_id) ORDER BY decision.record_id", 8),
        ("classification_bookmark", "SELECT bookmark.record_id, scope, admission_id, root_id, location_id,
           primary_missing, primary_text, primary_number, after_current, revision
         FROM classification_review_bookmark AS bookmark JOIN organization_record_groups AS member USING(record_id)
         JOIN temp.organization_cleanup_groups AS selected USING(group_id)", 10),
    ] {
        authority::field(digest, kind.as_bytes());
        let mut statement = connection.prepare(query).map_err(database_error)?;
        let mut rows = statement.query([]).map_err(database_error)?;
        let mut examined: u64 = 0;
        while let Some(row) = rows.next().map_err(database_error)? {
            super::inspection::hash_row(digest, row, columns)?;
            examined += 1;
            if examined.is_multiple_of(256) { checkpoint()?; }
        }
        checkpoint()?;
    }
    Ok(count)
}
