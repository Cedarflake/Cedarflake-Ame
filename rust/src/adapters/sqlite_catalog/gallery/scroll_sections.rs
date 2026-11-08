use std::collections::BTreeMap;

use rusqlite::{Transaction, params_from_iter, types::Value};

use crate::domain::gallery_scroll_section::{GalleryScrollSection, GalleryScrollSectionKind};
use crate::domain::{GalleryQuery, GallerySortDirection, ScanError};

use super::super::{database_error, sqlite_unsigned};
use super::{gallery_direction_sql, gallery_source, organization_groups, push_gallery_filters};

pub(super) struct NameSections {
    pub sections: Vec<GalleryScrollSection>,
    pub total_source_items: Option<u64>,
}

pub(in crate::adapters::sqlite_catalog) fn project_name_sections<'a>(
    names: impl Iterator<Item = &'a str>,
    direction: &GallerySortDirection,
) -> Vec<GalleryScrollSection> {
    let mut groups = BTreeMap::<String, (&str, u64)>::new();
    for name in names {
        let key = match name.chars().next() {
            Some('\u{1e}') => "0-9".to_owned(),
            Some(first @ 'a'..='z') => first.to_ascii_uppercase().to_string(),
            Some(first) if first > 'z' => "symbols-after".to_owned(),
            _ => "symbols-before".to_owned(),
        };
        let (first, count) = groups.entry(key).or_insert((name, 0));
        *first = (*first).min(name);
        *count += 1;
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, (name, _))| *name);
    if *direction == GallerySortDirection::Descending {
        groups.reverse();
    }
    let mut start_ordinal = 0;
    groups
        .into_iter()
        .map(|(key, (_, item_count))| {
            let section = GalleryScrollSection {
                kind: GalleryScrollSectionKind::Name,
                key,
                start_ordinal,
                item_count,
            };
            start_ordinal += item_count;
            section
        })
        .collect()
}

pub(super) fn load_name_sections(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<NameSections, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let mut prefix = String::new();
    let (source, source_count) = if organization_groups::uses_grouped_organization(query) {
        let grouped = organization_groups::build_group_query(query);
        parameters.extend(grouped.parameters);
        prefix = format!("WITH organization_groups AS MATERIALIZED ({})", grouped.sql);
        let source = gallery_source(query, &mut parameters);
        clauses.push(
            "locations.location_id IN (SELECT location_id FROM organization_groups)".to_owned(),
        );
        (
            source,
            "(SELECT SUM(source_items) FROM organization_groups)",
        )
    } else {
        let source = gallery_source(query, &mut parameters);
        push_gallery_filters(query, &mut clauses, &mut parameters);
        (source, "NULL")
    };
    let scope = if clauses.is_empty() {
        "1".to_owned()
    } else {
        clauses.join(" AND ")
    };
    // Numeric prefixes use the natural-sort marker. Symbols on either side of A-Z
    // stay separate so every section denotes a contiguous part of the existing order.
    let sql = format!(
        "{prefix} SELECT CASE
           WHEN substr(locations.natural_name_key, 1, 1) = char(30) THEN '0-9'
           WHEN substr(locations.natural_name_key, 1, 1) < 'a' THEN 'symbols-before'
           WHEN substr(locations.natural_name_key, 1, 1) > 'z' THEN 'symbols-after'
           ELSE upper(substr(locations.natural_name_key, 1, 1)) END AS section_key,
           COUNT(*), {source_count}
         {source} WHERE {scope}
         GROUP BY section_key
         ORDER BY MIN(locations.natural_name_key) {direction}",
        direction = gallery_direction_sql(query.order_direction()),
    );
    let mut statement = transaction.prepare(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(params_from_iter(parameters.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(database_error)?;
    let mut sections = Vec::new();
    let mut start_ordinal = 0;
    let mut total_source_items = organization_groups::uses_grouped_organization(query).then_some(0);
    for row in rows {
        let (key, count, source_items) = row.map_err(database_error)?;
        if let Some(count) = source_items {
            total_source_items = Some(sqlite_unsigned(count, "gallery source item count")?);
        }
        let item_count = sqlite_unsigned(count, "scroll section item count")?;
        sections.push(GalleryScrollSection {
            kind: GalleryScrollSectionKind::Name,
            key,
            start_ordinal,
            item_count,
        });
        start_ordinal += item_count;
    }
    Ok(NameSections {
        sections,
        total_source_items,
    })
}

pub(super) fn load_arrival_sections(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<Vec<GalleryScrollSection>, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let admission = if let Some(boundary) = query.new_since_admission {
        parameters.push(Value::Integer(
            super::super::organization::admission_boundary::resolve(transaction, boundary)?,
        ));
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
