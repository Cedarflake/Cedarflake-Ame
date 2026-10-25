use rusqlite::{Transaction, params_from_iter, types::Value};

use crate::domain::gallery_scroll_section::{GalleryScrollSection, GalleryScrollSectionKind};
use crate::domain::{GalleryQuery, ScanError};

use super::super::{database_error, sqlite_integer, sqlite_unsigned};
use super::{gallery_direction_sql, gallery_source, push_gallery_filters};

pub(super) fn load_name_sections(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<Vec<GalleryScrollSection>, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    // Numeric prefixes use the natural-sort marker. Symbols on either side of A-Z
    // stay separate so every section denotes a contiguous part of the existing order.
    let sql = format!(
        "SELECT CASE
           WHEN substr(locations.natural_name_key, 1, 1) = char(30) THEN '0-9'
           WHEN substr(locations.natural_name_key, 1, 1) < 'a' THEN 'symbols-before'
           WHEN substr(locations.natural_name_key, 1, 1) > 'z' THEN 'symbols-after'
           ELSE upper(substr(locations.natural_name_key, 1, 1)) END AS section_key,
           COUNT(*)
         {source} WHERE {scope}
         GROUP BY section_key
         ORDER BY MIN(locations.natural_name_key) {direction}",
        direction = gallery_direction_sql(query.order_direction()),
    );
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(params_from_iter(parameters.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(database_error)?;
    let mut sections = Vec::new();
    let mut start_ordinal = 0;
    for row in rows {
        let (key, count) = row.map_err(database_error)?;
        let item_count = sqlite_unsigned(count, "scroll section item count")?;
        sections.push(GalleryScrollSection {
            kind: GalleryScrollSectionKind::Name,
            key,
            start_ordinal,
            item_count,
        });
        start_ordinal += item_count;
    }
    Ok(sections)
}

pub(super) fn load_arrival_sections(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<Vec<GalleryScrollSection>, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let admission = if let Some(boundary) = query.new_since_admission {
        parameters.push(Value::Integer(sqlite_integer(
            boundary,
            "new-arrival boundary",
        )?));
        format!(
            "{} > ?",
            super::organization_queue::admission_expression("locations")
        )
    } else {
        "0".to_owned()
    };
    let source = gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    let sql = format!(
        "SELECT {admission} AS is_new, COUNT(*) {source} WHERE {scope}
         GROUP BY is_new ORDER BY is_new DESC",
    );
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(params_from_iter(parameters.iter()), |row| {
            Ok((row.get::<_, bool>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(database_error)?;
    let mut sections = Vec::new();
    let mut start_ordinal = 0;
    for row in rows {
        let (is_new, count) = row.map_err(database_error)?;
        let item_count = sqlite_unsigned(count, "arrival section item count")?;
        sections.push(GalleryScrollSection {
            kind: if is_new {
                GalleryScrollSectionKind::NewArrivals
            } else {
                GalleryScrollSectionKind::Earlier
            },
            key: if is_new { "new" } else { "earlier" }.to_owned(),
            start_ordinal,
            item_count,
        });
        start_ordinal += item_count;
    }
    Ok(sections)
}
